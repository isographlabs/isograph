use std::collections::{BTreeMap, HashMap};

use common_lang_types::{
    Diagnostic, DiagnosticResult, EmbeddedLocation, EntityName, SelectableName,
    WithLocationPostfix, WithNonFatalDiagnostics,
};
use graphql_lang_types::from_graphql_directives;
use intern::Lookup;
use intern::string_key::Intern;
use isograph_lang_types::{
    EmptyDirectiveSet, ObjectSelection, ScalarSelection, SelectionSet, SelectionTypePostfix,
};
use isograph_schema::{
    ClientFieldVariant, ClientScalarSelectable, ConcreteTargetEntityName,
    DeprecatedParseTypeSystemOutcome, FlattenedDataModelEntity, FlattenedDataModelSelectable,
    ImperativelyLoadedFieldVariant, IsographDatabase, RefetchStrategy, RootOperationName,
    ServerEntityDirectives, WrappedSelectionMapSelection, flattened_entity_named,
    flattened_selectable_named, generate_refetch_field_strategy,
    imperative_field_subfields_or_inline_fragments,
    insert_selectable_or_multiple_definition_diagnostic,
};
use prelude::{ErrClone, Postfix};

use crate::{
    GraphQLAndJavascriptProfile, parse_graphql_schema,
    process_type_system_definition::{
        process_graphql_type_system_document, process_graphql_type_system_extension_document,
    },
};

#[expect(clippy::type_complexity)]
pub(crate) fn parse_type_system_document(
    db: &IsographDatabase<GraphQLAndJavascriptProfile>,
) -> DiagnosticResult<(
    WithNonFatalDiagnostics<DeprecatedParseTypeSystemOutcome<GraphQLAndJavascriptProfile>>,
    // fetchable types
    BTreeMap<EntityName, RootOperationName>,
)> {
    let mut outcome = DeprecatedParseTypeSystemOutcome::default();
    let mut non_fatal_diagnostics = vec![];

    let mut graphql_root_types = None;
    let mut directives = HashMap::new();
    let mut interfaces_to_process = vec![];

    let (type_system_document, type_system_extension_documents) =
        parse_graphql_schema(db).to_owned()?;

    process_graphql_type_system_document(
        db,
        type_system_document,
        &mut graphql_root_types,
        &mut outcome,
        &mut directives,
        &mut interfaces_to_process,
        &mut non_fatal_diagnostics,
    );

    for type_system_extension_document in type_system_extension_documents.values() {
        process_graphql_type_system_extension_document(
            db,
            type_system_extension_document
                .to_owned(db)
                .note_todo("Don't clone, use a MemoRef"),
            &mut graphql_root_types,
            &mut outcome,
            &mut directives,
            &mut interfaces_to_process,
            &mut non_fatal_diagnostics,
        );
    }

    // We process interfaces later, because we need to know all of the subtypes that an interface
    // implements. In an ideal world, this info would not be part of the ServerEntity struct,
    // and we should make that refactor.
    for with_location in interfaces_to_process {
        let interface_definition = with_location.item;
        let server_object_entity_name = interface_definition.name.item;

        directives
            .entry(server_object_entity_name)
            .or_default()
            .extend(interface_definition.directives);

        // I don't think interface-to-interface refinement is handled correctly, let's just
        // ignore it for now.
    }

    // exposeField directives -> fields
    'exposeField: for (parent_object_entity_name, directives) in directives {
        let result = from_graphql_directives::<ServerEntityDirectives>(&directives)?;
        for expose_field_directive in result.expose_field {
            // HACK: we're essentially splitting the field arg by . and keeping the same
            // implementation as before. But really, there isn't much a distinction
            // between field and path, and we should clean this up.
            //
            // But, this is an expedient way to combine field and path.
            let mut path = expose_field_directive.field.lookup().split('.');
            let field = path.next().expect(
                "Expected iter to have at least one element. \
                    This is indicative of a bug in Isograph.",
            );
            let primary_field_name_selection_parts = path
                .map(|x| x.intern().into())
                .collect::<Vec<SelectableName>>();

            let mutation_subfield_name: SelectableName = field.intern().into();

            let mutation_field = match flattened_selectable_named(
                db,
                parent_object_entity_name,
                mutation_subfield_name,
            ) {
                Some(s) => s.lookup(db),
                None => {
                    non_fatal_diagnostics.push(Diagnostic::new(
                        "Mutation field not found".to_string(),
                        None,
                    ));
                    continue 'exposeField;
                }
            };

            let payload_object_entity_name = match mutation_field.target_entity.item.clone_err() {
                Ok(annotation) => annotation.inner().0,
                Err(e) => {
                    non_fatal_diagnostics.push(e);
                    continue 'exposeField;
                }
            };

            let client_field_scalar_selection_name = expose_field_directive
                .expose_as
                .unwrap_or(mutation_field.name.item);
            let mutation_field_arguments = mutation_field.arguments.clone();

            let top_level_schema_field_selection_info =
                flattened_entity_named(db, payload_object_entity_name)
                    .and_then(|entity| entity.lookup(db).selection_info.as_object())
                    .expect("Expected entity to exist and to be an object.");

            let (mut parts_reversed, target_parent_object_entity) =
                match traverse_selections_and_return_path(
                    db,
                    payload_object_entity_name,
                    &primary_field_name_selection_parts,
                ) {
                    Ok(ok) => ok,
                    Err(e) => {
                        non_fatal_diagnostics.push(e);
                        continue 'exposeField;
                    }
                };

            let target_parent_object_entity_name = target_parent_object_entity.name;
            parts_reversed.reverse();

            let fields = expose_field_directive
                .field_map
                .iter()
                .map(|field_map_item| {
                    ScalarSelection {
                        name: field_map_item
                            .from
                            .unchecked_conversion::<SelectableName>()
                            .with_location(EmbeddedLocation::todo_generated()),
                        reader_alias: None,
                        arguments: vec![],
                        scalar_selection_directive_set:
                            isograph_lang_types::ScalarSelectionDirectiveSet::None(
                                EmptyDirectiveSet {},
                            ),
                    }
                    .scalar_selected::<ObjectSelection>()
                    .with_location(EmbeddedLocation::todo_generated())
                })
                .collect::<Vec<_>>();

            let top_level_schema_field_arguments =
                mutation_field_arguments.into_iter().collect::<Vec<_>>();

            let mut subfields_or_inline_fragments = parts_reversed
                .iter()
                .map(|server_object_selectable| {
                    let target_entity = server_object_selectable
                        .target_entity
                        .item
                        .as_ref()
                        .expect("Expected target entity to be valid");
                    if server_object_selectable.is_inline_fragment.0 {
                        WrappedSelectionMapSelection::InlineFragment(target_entity.inner().0)
                    } else {
                        WrappedSelectionMapSelection::LinkedField {
                            is_fallible: target_entity.is_nullable(),
                            server_object_selectable_name: server_object_selectable.name.item,
                            arguments: vec![],
                            concrete_target_entity_name: ConcreteTargetEntityName::Concrete(
                                target_parent_object_entity_name.item,
                            )
                            .note_todo(
                                "This is 100% a bug when there are \
                                multiple items in parts_reversed, or this \
                                field is ignored.",
                            ),
                        }
                    }
                })
                .collect::<Vec<_>>();

            subfields_or_inline_fragments.push(imperative_field_subfields_or_inline_fragments(
                mutation_subfield_name,
                &top_level_schema_field_arguments,
                if top_level_schema_field_selection_info.is_concrete.0 {
                    ConcreteTargetEntityName::Concrete(payload_object_entity_name)
                } else {
                    ConcreteTargetEntityName::Abstract
                },
                mutation_field
                    .target_entity
                    .item
                    .as_ref()
                    .expect("Expected target entity to be valid")
                    .is_nullable(),
            ));

            let mutation_client_scalar_selectable = ClientScalarSelectable {
                description: mutation_field.description,
                name: client_field_scalar_selection_name.unchecked_conversion::<SelectableName>(),
                variant: ClientFieldVariant::ImperativelyLoadedField(
                    ImperativelyLoadedFieldVariant {
                        client_selection_name: client_field_scalar_selection_name
                            .unchecked_conversion(),
                        root_object_entity_name: parent_object_entity_name,
                        // This is fishy! subfields_or_inline_fragments is cloned and stored in multiple locations,
                        // but presumably we could access it from one location only
                        subfields_or_inline_fragments: subfields_or_inline_fragments.clone(),
                        field_map: expose_field_directive.field_map,
                        top_level_schema_field_arguments,
                    },
                ),
                arguments: vec![],
                parent_entity_name: target_parent_object_entity_name.item,
                phantom_data: std::marker::PhantomData,
            };

            insert_selectable_or_multiple_definition_diagnostic(
                &mut outcome.selectables,
                (
                    target_parent_object_entity_name.item,
                    client_field_scalar_selection_name.unchecked_conversion(),
                ),
                mutation_client_scalar_selectable
                    .interned_value(db)
                    .scalar_selected()
                    .with_generated_location(),
                &mut non_fatal_diagnostics,
            );

            outcome.client_scalar_refetch_strategies.push(
                (
                    target_parent_object_entity_name.item,
                    client_field_scalar_selection_name.unchecked_conversion::<SelectableName>(),
                    RefetchStrategy::UseRefetchField(generate_refetch_field_strategy(
                        SelectionSet {
                            selections: fields.to_vec(),
                        }
                        .with_location(EmbeddedLocation::todo_generated()),
                        parent_object_entity_name,
                        subfields_or_inline_fragments,
                    )),
                )
                    .with_generated_location()
                    .wrap_ok(),
            )
        }
    }

    (
        WithNonFatalDiagnostics::new(outcome, non_fatal_diagnostics),
        graphql_root_types.unwrap_or_default().into(),
    )
        .wrap_ok()
}

fn traverse_selections_and_return_path<'a>(
    db: &'a IsographDatabase<GraphQLAndJavascriptProfile>,
    payload_object_entity_name: EntityName,
    primary_field_selection_name_parts: &[SelectableName],
) -> DiagnosticResult<(
    Vec<&'a FlattenedDataModelSelectable<GraphQLAndJavascriptProfile>>,
    &'a FlattenedDataModelEntity<GraphQLAndJavascriptProfile>,
)> {
    let mut current_entity = flattened_entity_named(db, payload_object_entity_name)
        .ok_or_else(|| {
            Diagnostic::new(
                format!(
                    "Invalid @exposeField directive. Entity {} was not found.",
                    payload_object_entity_name
                ),
                None,
            )
        })?
        .lookup(db);

    if current_entity.selection_info.as_object().is_none() {
        return Diagnostic::new(
            format!(
                "Invalid @exposeField directive. Entity {} is not an object.",
                payload_object_entity_name
            ),
            None,
        )
        .wrap_err();
    }

    let mut output = vec![];

    for selection_name in primary_field_selection_name_parts {
        let selectable =
            flattened_selectable_named(db, current_entity.name.item, selection_name.dereference())
                .ok_or_else(|| {
                    Diagnostic::new(
                        format!(
                            "Invalid @exposeField directive. Field {} not found.",
                            selection_name
                        ),
                        None,
                    )
                })?
                .lookup(db);

        let next_entity_name = selectable.target_entity.item.clone_err()?.inner();

        current_entity = flattened_entity_named(db, next_entity_name.0)
            .ok_or_else(|| {
                Diagnostic::new(
                    format!(
                        "Invalid @exposeField directive. Entity {} not found.",
                        next_entity_name
                    ),
                    None,
                )
            })?
            .lookup(db);

        if current_entity.selection_info.as_object().is_none() {
            return Diagnostic::new(
                format!(
                    "Invalid @exposeField directive. Entity {} is not an object.",
                    next_entity_name
                ),
                None,
            )
            .wrap_err();
        }

        output.push(selectable);
    }

    (output, current_entity).wrap_ok()
}
