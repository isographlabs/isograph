use common_lang_types::{
    Diagnostic, EmbeddedLocation, EntityName, EntityNameAndSelectableName, IsographCodeAction,
    Location, SelectableName,
};
use isograph_lang_types::{
    DefinitionLocation, DefinitionLocationPostfix, ObjectSelectionDirectiveSet,
    ScalarSelectionDirectiveSet, SelectionSet, SelectionType,
};
use prelude::{ErrClone, Postfix};
use std::collections::HashSet;

use crate::{
    ClientFieldVariant, CompilationProfile, IsographDatabase, ServerEntity,
    entity_not_defined_diagnostic, flattened_entity_named, reader_selection_set_map,
    selectable_is_wrong_type_diagnostic, selectable_named,
};

pub(crate) fn validate_selection_sets<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> Vec<Diagnostic> {
    let selection_sets = reader_selection_set_map(db);

    let mut errors = vec![];
    for (key, selection_set) in selection_sets {
        let selection_set = match selection_set.clone_err() {
            Ok(s) => s,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };
        let selection_set = match selection_set {
            SelectionType::Scalar(s) => s.lookup(db).item.reference(),
            SelectionType::Object(o) => o.lookup(db).item.reference(),
        };

        let parent_entity = flattened_entity_named(db, key.0);
        let parent_entity = match parent_entity {
            Some(s) => s.lookup(db),
            None => {
                // TODO better location... and this is probably already validated elsewhere.
                // Maybe we can just continue
                errors.push(entity_not_defined_diagnostic(key.0, Location::Generated));
                continue;
            }
        };

        validate_selection_set(
            db,
            &mut errors,
            selection_set,
            parent_entity,
            EntityNameAndSelectableName {
                parent_entity_name: key.0,
                selectable_name: key.1,
            },
        )
    }

    errors
}

/// for each selection, validate that it corresponds to a selectable of the correct SelectionType,
/// as well as ensure that @loadable is only selected on client fields that aren't exposed fields
fn validate_selection_set<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    errors: &mut Vec<Diagnostic>,
    selection_set: &SelectionSet,
    parent_entity: &ServerEntity<TCompilationProfile>,
    selectable_declaration_info: EntityNameAndSelectableName,
) {
    let mut encountered_names_or_aliases = HashSet::new();

    for selection in selection_set.selections.iter() {
        if !encountered_names_or_aliases.insert(selection.item.name_or_alias().item) {
            errors.push(Diagnostic::new(
                format!(
                    "A field with name or alias `{}` \
                    has already been defined",
                    selection.item.name_or_alias().item
                ),
                selection.location.to::<Location>().wrap_some(),
            ));
        }

        match selection.item.reference() {
            SelectionType::Scalar(scalar_selection) => {
                let selectable_name = scalar_selection.name.item;
                let selectable = match selectable_named(
                    db,
                    parent_entity.name.item,
                    selectable_name,
                )
                .clone_err()
                {
                    Ok(s) => match s {
                        Some(s) => s,
                        None => {
                            errors.push(selection_does_not_exist_diagnostic(
                                parent_entity.name.item,
                                selectable_name,
                                scalar_selection.name.location,
                                SelectionType::Scalar(()),
                                selectable_declaration_info,
                            ));
                            continue;
                        }
                    },
                    Err(e) => {
                        errors.push(e);
                        continue;
                    }
                };

                let scalar_selectable = match selectable {
                    DefinitionLocation::Server(s) => {
                        let selectable = s.lookup(db);
                        let target_entity_name = match selectable.target_entity.item.clone_err() {
                            Ok(annotation) => annotation.inner().0,
                            Err(e) => {
                                errors.push(e);
                                continue;
                            }
                        };
                        let entity = flattened_entity_named(db, target_entity_name);
                        let entity = match entity {
                            Some(entity) => entity.lookup(db),
                            None => {
                                errors.push(entity_not_defined_diagnostic(
                                    target_entity_name,
                                    scalar_selection.name.location.to::<Location>(),
                                ));
                                continue;
                            }
                        };

                        if entity.selection_info.as_object().is_some() {
                            let location = scalar_selection.name.location.to::<Location>();
                            errors.push(selectable_is_wrong_type_diagnostic(
                                selectable.parent_entity_name.item,
                                selectable.name.item,
                                "a scalar",
                                "an object",
                                location,
                            ));
                            continue;
                        }

                        selectable.server_defined()
                    }
                    DefinitionLocation::Client(c) => match c {
                        SelectionType::Scalar(s) => s.client_defined(),
                        SelectionType::Object(_) => {
                            errors.push(selection_wrong_selection_type_diagnostic(
                                parent_entity.name.item,
                                selectable_name,
                                "a scalar",
                                "an object",
                                scalar_selection.name.location,
                                selectable_declaration_info,
                            ));
                            continue;
                        }
                    },
                };

                // @loadable is not supported on server scalar selections
                // @updatable is not supported on client scalar selections
                // @loadable is not supported on selections of exposed selectables
                // (which are client scalar selectables)
                match scalar_selectable {
                    DefinitionLocation::Server(_) => {
                        match scalar_selection.scalar_selection_directive_set {
                            ScalarSelectionDirectiveSet::Loadable(_) => {
                                errors.push(Diagnostic::new(
                                    format!(
                                        "`{}.{}` is a server scalar field, and \
                                        @loadable is not supported on selections of server scalar fields.",
                                        parent_entity.name, scalar_selection.name.item,
                                    ),
                                    scalar_selection.name.location.to::<Location>().wrap_some(),
                                ));
                            }
                            ScalarSelectionDirectiveSet::Updatable(_) => {
                                // TODO ensure that the selectable is not "special",
                                // i.e. is not id or __typename
                            }
                            ScalarSelectionDirectiveSet::None(_) => {}
                        }
                    }
                    DefinitionLocation::Client(client_scalar_selectable) => {
                        match scalar_selection.scalar_selection_directive_set {
                            ScalarSelectionDirectiveSet::Loadable(_) => {
                                let client_scalar_selectable = client_scalar_selectable.lookup(db);
                                match client_scalar_selectable.variant.reference() {
                                    ClientFieldVariant::UserWritten(_) => {}
                                    ClientFieldVariant::ImperativelyLoadedField(_) => {
                                        errors.push(Diagnostic::new(
                                            format!(
                                                "`{}.{}` is an exposed field. \
                                                @loadable is not supported on exposed fields.",
                                                parent_entity.name, scalar_selection.name.item,
                                            ),
                                            scalar_selection
                                                .name
                                                .location
                                                .to::<Location>()
                                                .wrap_some(),
                                        ));
                                    }
                                    ClientFieldVariant::Link => {
                                        errors.push(Diagnostic::new(
                                            "@loadable is not supported on __link fields"
                                                .to_string(),
                                            scalar_selection
                                                .name
                                                .location
                                                .to::<Location>()
                                                .wrap_some(),
                                        ));
                                    }
                                }
                            }
                            ScalarSelectionDirectiveSet::Updatable(_) => {
                                errors.push(Diagnostic::new(
                                    format!(
                                        "`{}.{}` is a client scalar field, and \
                                        @updatable is not supported on selections of client scalar fields.",
                                        parent_entity.name, scalar_selection.name.item,
                                    ),
                                    scalar_selection.name.location.to::<Location>().wrap_some(),
                                ));
                            }
                            ScalarSelectionDirectiveSet::None(_) => {}
                        }
                    }
                }
            }
            SelectionType::Object(object_selection) => {
                let selectable_name = object_selection.name.item;
                let selectable = match selectable_named(
                    db,
                    parent_entity.name.item,
                    selectable_name,
                )
                .clone_err()
                {
                    Ok(s) => match s {
                        Some(s) => s,
                        None => {
                            errors.push(selection_does_not_exist_diagnostic(
                                parent_entity.name.item,
                                selectable_name,
                                object_selection.name.location,
                                SelectionType::Object(()),
                                selectable_declaration_info,
                            ));
                            continue;
                        }
                    },
                    Err(e) => {
                        errors.push(e);
                        continue;
                    }
                };

                let selectable = match selectable {
                    DefinitionLocation::Server(s) => {
                        let selectable = s.lookup(db);
                        let target_entity_name = match selectable.target_entity.item.clone_err() {
                            Ok(annotation) => annotation.inner().0,
                            Err(e) => {
                                errors.push(e);
                                continue;
                            }
                        };
                        let entity = flattened_entity_named(db, target_entity_name);

                        let entity = match entity {
                            Some(entity) => entity.lookup(db),
                            None => {
                                errors.push(entity_not_defined_diagnostic(
                                    target_entity_name,
                                    object_selection.name.location.to::<Location>(),
                                ));
                                continue;
                            }
                        };

                        if entity.selection_info.as_scalar().is_some() {
                            let location = object_selection.name.location.to::<Location>();
                            errors.push(selectable_is_wrong_type_diagnostic(
                                selectable.parent_entity_name.item,
                                selectable.name.item,
                                "an object",
                                "a scalar",
                                location,
                            ));
                            continue;
                        }

                        s.server_defined()
                    }
                    DefinitionLocation::Client(c) => match c {
                        SelectionType::Scalar(_) => {
                            errors.push(selection_wrong_selection_type_diagnostic(
                                parent_entity.name.item,
                                selectable_name,
                                "an object",
                                "a scalar",
                                object_selection.name.location,
                                selectable_declaration_info,
                            ));
                            continue;
                        }
                        SelectionType::Object(o) => o.client_defined(),
                    },
                };

                // @updatable is not supported on client fields
                let target_entity_name = match selectable {
                    DefinitionLocation::Server(s) => {
                        match s.lookup(db).target_entity.item.clone_err() {
                            Ok(annotation) => annotation.inner(),
                            Err(e) => {
                                errors.push(e);
                                continue;
                            }
                        }
                    }
                    DefinitionLocation::Client(c) => {
                        match object_selection.object_selection_directive_set {
                            ObjectSelectionDirectiveSet::Updatable(_) => {
                                errors.push(Diagnostic::new(
                                    format!(
                                        "`{}.{}` is a client object field. \
                                        @updatable is not supported on client object fields.",
                                        parent_entity.name, object_selection.name.item
                                    ),
                                    object_selection.name.location.to::<Location>().wrap_some(),
                                ))
                            }
                            ObjectSelectionDirectiveSet::None(_) => {}
                        }

                        c.lookup(db).target_entity_name.inner()
                    }
                }
                .0;

                let new_parent_entity = flattened_entity_named(db, target_entity_name);
                let new_parent_entity = match new_parent_entity {
                    Some(s) => s.lookup(db),
                    None => {
                        // This was probably validated elsewhere??
                        errors.push(entity_not_defined_diagnostic(
                            target_entity_name,
                            object_selection.name.location.to(),
                        ));
                        continue;
                    }
                };

                validate_selection_set(
                    db,
                    errors,
                    &object_selection.selection_set.item,
                    new_parent_entity,
                    selectable_declaration_info,
                );
            }
        }
    }
}

fn selection_wrong_selection_type_diagnostic(
    selectable_entity_name: EntityName,
    selectable_name: SelectableName,
    selected_as: &str,
    proper_way_to_select: &str,
    location: EmbeddedLocation,
    selectable_declaration_info: EntityNameAndSelectableName,
) -> Diagnostic {
    Diagnostic::new(
        format!(
            "In `{}.{}`, `{selectable_entity_name}.{selectable_name}` \
            is selected as {selected_as}. It should be selected \
            as {proper_way_to_select}.",
            selectable_declaration_info.parent_entity_name,
            selectable_declaration_info.selectable_name
        ),
        location.to::<Location>().wrap_some(),
    )
}

fn selection_does_not_exist_diagnostic(
    selectable_parent_object_entity_name: EntityName,
    selectable_name: SelectableName,
    location: EmbeddedLocation,
    selection_type: SelectionType<(), ()>,
    selectable_declaration_info: EntityNameAndSelectableName,
) -> Diagnostic {
    Diagnostic::new_with_code_actions(
        format!(
            "In `{}.{}`, `{selectable_parent_object_entity_name}.{selectable_name}` is selected. \
            However, `{selectable_name}` does not exist on `{selectable_parent_object_entity_name}`.",
            selectable_declaration_info.parent_entity_name,
            selectable_declaration_info.selectable_name
        ),
        location.to::<Location>().wrap_some(),
        match selection_type {
            SelectionType::Scalar(_) => {
                IsographCodeAction::CreateNewScalarSelectable(EntityNameAndSelectableName {
                    parent_entity_name: selectable_parent_object_entity_name,
                    selectable_name,
                })
            }
            SelectionType::Object(_) => {
                IsographCodeAction::CreateNewObjectSelectable(EntityNameAndSelectableName {
                    parent_entity_name: selectable_parent_object_entity_name,
                    selectable_name,
                })
            }
        }
        .wrap_vec(),
    )
}
