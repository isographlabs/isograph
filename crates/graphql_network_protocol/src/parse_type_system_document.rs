use std::collections::{BTreeMap, BTreeSet, HashMap};

use common_lang_types::{
    DescriptionValue, Diagnostic, DiagnosticResult, EmbeddedLocation, EntityName, SelectableName,
    VariableName, WithGenericLocation, WithLocation, WithLocationPostfix, WithNonFatalDiagnostics,
};
use graphql_lang_types::from_graphql_directives;
use intern::Lookup;
use intern::string_key::Intern;
use isograph_lang_types::{
    DefinitionLocation, DefinitionLocationPostfix, Description, EmptyDirectiveSet, ObjectSelection,
    ScalarSelection, SelectionSet, SelectionType, SelectionTypePostfix, TypeAnnotationDeclaration,
    UnionTypeAnnotationDeclaration, UnionVariant, VariableDeclaration,
};
use isograph_schema::{
    BOOLEAN_ENTITY_NAME, ClientFieldVariant, ClientScalarSelectable, FLOAT_ENTITY_NAME,
    ID_ENTITY_NAME, INT_ENTITY_NAME, ImperativelyLoadedFieldVariant, IsConcrete, IsographDatabase,
    ParseTypeSystemOutcome, RefetchStrategy, RootOperationName, STRING_ENTITY_NAME, ServerEntity,
    ServerEntityDirectives, ServerObjectSelectableVariant, ServerObjectSelectionInfo,
    ServerSelectable, TYPENAME_FIELD_NAME, WrappedSelectionMapSelection,
    generate_refetch_field_strategy, imperative_field_subfields_or_inline_fragments,
    insert_selectable_or_multiple_definition_diagnostic, to_isograph_constant_value,
};
use prelude::Postfix;

use crate::{
    BOOLEAN_JAVASCRIPT_TYPE, GraphQLAndJavascriptProfile, GraphQLSchemaObjectAssociatedData,
    NUMBER_JAVASCRIPT_TYPE, STRING_JAVASCRIPT_TYPE,
    insert_entity_or_multiple_definition_diagnostic, parse_graphql_schema,
    process_type_system_definition::{
        get_typename_selectable, process_graphql_type_system_document,
        process_graphql_type_system_extension_document,
    },
};

#[expect(clippy::type_complexity)]
pub(crate) fn parse_type_system_document(
    db: &IsographDatabase<GraphQLAndJavascriptProfile>,
) -> DiagnosticResult<(
    WithNonFatalDiagnostics<ParseTypeSystemOutcome<GraphQLAndJavascriptProfile>>,
    // fetchable types
    BTreeMap<EntityName, RootOperationName>,
)> {
    let mut outcome = ParseTypeSystemOutcome::default();
    let mut non_fatal_diagnostics = vec![];
    define_default_graphql_types(db, &mut outcome, &mut non_fatal_diagnostics);

    let mut graphql_root_types = None;
    let mut directives = HashMap::new();
    let mut fields_to_process = vec![];
    let mut supertype_to_subtype_map = BTreeMap::new();
    let mut interfaces_to_process = vec![];

    let (type_system_document, type_system_extension_documents) =
        parse_graphql_schema(db).to_owned()?;

    process_graphql_type_system_document(
        db,
        type_system_document,
        &mut graphql_root_types,
        &mut outcome,
        &mut directives,
        &mut fields_to_process,
        &mut supertype_to_subtype_map,
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
            &mut fields_to_process,
            &mut supertype_to_subtype_map,
            &mut interfaces_to_process,
            &mut non_fatal_diagnostics,
        );
    }

    // We process interfaces later, because we need to know all of the subtypes that an interface
    // implements. In an ideal world, this info would not be part of the ServerEntity struct,
    // and we should make that refactor.
    for with_location in interfaces_to_process {
        let interface_definition = with_location.item;
        let server_object_entity_name = interface_definition.name.item.to::<EntityName>();

        insert_entity_or_multiple_definition_diagnostic(
            &mut outcome.entities,
            server_object_entity_name,
            ServerEntity {
                description: interface_definition.description.map(|description_value| {
                    description_value
                        .item
                        .unchecked_conversion::<DescriptionValue>()
                        .wrap(Description)
                }),
                name: server_object_entity_name,
                network_protocol_associated_data: (),
                selection_info: ServerObjectSelectionInfo {
                    is_concrete: IsConcrete(false),
                }
                .object_selected(),
                target_platform_associated_data: GraphQLSchemaObjectAssociatedData {
                    subtypes: supertype_to_subtype_map
                        .get(&server_object_entity_name)
                        .cloned()
                        .unwrap_or_default(),
                }
                .object_selected(),
            }
            .interned_value(db)
            .with_location(with_location.location)
            .into(),
            &mut non_fatal_diagnostics,
        );

        directives
            .entry(server_object_entity_name)
            .or_default()
            .extend(interface_definition.directives);

        for field in interface_definition.fields {
            fields_to_process.push((server_object_entity_name, field));
        }

        insert_selectable_or_multiple_definition_diagnostic(
            &mut outcome.selectables,
            (server_object_entity_name, (*TYPENAME_FIELD_NAME)),
            get_typename_selectable(db, server_object_entity_name, None)
                .server_defined()
                .with_location(with_location.location)
                .into(),
            &mut non_fatal_diagnostics,
        );

        // I don't think interface-to-interface refinement is handled correctly, let's just
        // ignore it for now.
    }

    // Note: we need to know whether a field points to an object entity or scalar entity, and we
    // do not have that information when we first encounter that field. So, we accumulate fields
    // and handle them now. A future refactor will get rid of this: selectables will all be the
    // the same struct, and you will have to do a follow up request for the target entity to
    // know whether it is an object or scalar selectable.
    for (parent_entity_name, field) in fields_to_process {
        let target: EntityName = field.item.type_.item.inner().unchecked_conversion();

        if is_object_entity(db, &outcome.entities, target) {
            insert_selectable_or_multiple_definition_diagnostic(
                &mut outcome.selectables,
                (parent_entity_name, field.item.name.item),
                ServerSelectable {
                    description: field
                        .item
                        .description
                        .map(WithGenericLocation::drop_location)
                        .map(|x| x.map(Description::from)),

                    name: field.item.name.item.unchecked_conversion(),
                    target_entity_name: field
                        .item
                        .type_
                        .item
                        .clone()
                        .wrap(TypeAnnotationDeclaration::from_graphql_type_annotation),

                    is_inline_fragment: ServerObjectSelectableVariant::LinkedField
                        .object_selected(),
                    parent_entity_name,
                    arguments: field
                        .item
                        .arguments
                        .into_iter()
                        .map(|with_location| {
                            let arg = with_location.item;
                            VariableDeclaration {
                                name: arg.name.map(|input_value_name| {
                                    input_value_name
                                        .unchecked_conversion::<VariableName>()
                                        .into()
                                }),
                                type_: arg
                                    .type_
                                    .map(TypeAnnotationDeclaration::from_graphql_type_annotation),
                                default_value: arg.default_value.map(|with_location| {
                                    with_location.map(to_isograph_constant_value)
                                }),
                            }
                        })
                        .collect(),
                    network_protocol_associated_data: (),
                    target_platform_associated_data: ().object_selected(),
                }
                .interned_value(db)
                .server_defined()
                .with_location(field.location)
                .into(),
                &mut non_fatal_diagnostics,
            );
        } else {
            insert_selectable_or_multiple_definition_diagnostic(
                &mut outcome.selectables,
                (parent_entity_name, field.item.name.item),
                ServerSelectable {
                    description: field
                        .item
                        .description
                        .map(|x| x.drop_location().map(Description::from)),
                    name: field.item.name.item.unchecked_conversion(),
                    parent_entity_name,
                    arguments: field
                        .item
                        .arguments
                        .into_iter()
                        .map(|with_location| {
                            let arg = with_location.item;
                            VariableDeclaration {
                                name: arg.name.map(|input_value_name| {
                                    input_value_name
                                        .unchecked_conversion::<VariableName>()
                                        .into()
                                }),
                                type_: arg
                                    .type_
                                    .map(TypeAnnotationDeclaration::from_graphql_type_annotation),
                                default_value: arg.default_value.map(|with_location| {
                                    with_location.map(to_isograph_constant_value)
                                }),
                            }
                        })
                        .collect(),
                    target_entity_name: field
                        .item
                        .type_
                        .item
                        .clone()
                        .wrap(TypeAnnotationDeclaration::from_graphql_type_annotation),

                    network_protocol_associated_data: (),
                    is_inline_fragment: ().scalar_selected(),
                    target_platform_associated_data: None.scalar_selected(),
                }
                .interned_value(db)
                .server_defined()
                .with_location(field.location)
                .into(),
                &mut non_fatal_diagnostics,
            );
        }
    }

    // asConcreteType fields
    for (abstract_parent_entity_name, concrete_child_entity_names) in supertype_to_subtype_map {
        for concrete_child_entity_name in concrete_child_entity_names.iter() {
            insert_selectable_or_multiple_definition_diagnostic(
                &mut outcome.selectables,
                (
                    abstract_parent_entity_name.unchecked_conversion(),
                    format!("as{concrete_child_entity_name}").intern().into(),
                ),
                ServerSelectable {
                    description: format!(
                        "A client pointer for the {} type.",
                        concrete_child_entity_name
                    )
                    .intern()
                    .to::<DescriptionValue>()
                    .wrap(Description)
                    .with_no_location()
                    .wrap_some(),
                    name: format!("as{}", concrete_child_entity_name)
                        .intern()
                        .to::<SelectableName>(),
                    target_entity_name: TypeAnnotationDeclaration::Union(
                        UnionTypeAnnotationDeclaration {
                            variants: {
                                let mut variants = BTreeSet::new();
                                variants.insert(UnionVariant::Scalar(
                                    concrete_child_entity_name.dereference().into(),
                                ));
                                variants
                            },
                            nullable: true,
                        },
                    ),
                    is_inline_fragment: ServerObjectSelectableVariant::InlineFragment
                        .object_selected(),
                    parent_entity_name: abstract_parent_entity_name.unchecked_conversion(),
                    arguments: vec![],
                    network_protocol_associated_data: (),
                    target_platform_associated_data: ().object_selected(),
                }
                .interned_value(db)
                .server_defined()
                .with_generated_location(),
                &mut non_fatal_diagnostics,
            );
        }
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

            let mutation_field = match outcome
                .selectables
                .values()
                .filter_map(|x| x.item.as_server())
                .find_map(|server_object_selectable| {
                    let server_object_selectable = server_object_selectable.lookup(db);
                    if server_object_selectable.name == mutation_subfield_name {
                        Some(server_object_selectable)
                    } else {
                        None
                    }
                }) {
                Some(s) => s,
                None => {
                    non_fatal_diagnostics.push(Diagnostic::new(
                        "Mutation field not found".to_string(),
                        None,
                    ));
                    continue 'exposeField;
                }
            };

            let payload_object_entity_name = mutation_field.target_entity_name.inner().0;

            let client_field_scalar_selection_name = expose_field_directive
                .expose_as
                .unwrap_or(mutation_field.name);
            let top_level_schema_field_parent_object_entity_name =
                mutation_field.parent_entity_name;
            let mutation_field_arguments = mutation_field.arguments.clone();

            let top_level_schema_field_selection_info = outcome
                .entities
                .get(&payload_object_entity_name)
                .and_then(|entity| entity.item.lookup(db).selection_info.as_object())
                .expect("Expected entity to exist and to be an object.");

            let (mut parts_reversed, target_parent_object_entity) =
                match traverse_selections_and_return_path(
                    db,
                    &outcome,
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
                    let object_selectable_variant =
                        match server_object_selectable.is_inline_fragment.reference() {
                            SelectionType::Scalar(_) => {
                                panic!("Expected selectable to be an object")
                            }
                            SelectionType::Object(o) => o,
                        };

                    match object_selectable_variant {
                        ServerObjectSelectableVariant::LinkedField => {
                            WrappedSelectionMapSelection::LinkedField {
                                parent_object_entity_name: server_object_selectable
                                    .parent_entity_name,
                                server_object_selectable_name: server_object_selectable.name,
                                arguments: vec![],
                                concrete_target_entity_name: target_parent_object_entity_name
                                    .wrap_some()
                                    .note_todo(
                                        "This is 100% a bug when there are \
                                            multiple items in parts_reversed, or this \
                                            field is ignored.",
                                    ),
                            }
                        }
                        ServerObjectSelectableVariant::InlineFragment => {
                            WrappedSelectionMapSelection::InlineFragment(
                                server_object_selectable.target_entity_name.inner().0,
                            )
                        }
                    }
                })
                .collect::<Vec<_>>();

            subfields_or_inline_fragments.push(imperative_field_subfields_or_inline_fragments(
                mutation_subfield_name,
                &top_level_schema_field_arguments,
                if top_level_schema_field_selection_info.is_concrete.0 {
                    payload_object_entity_name.wrap_some()
                } else {
                    None
                },
                top_level_schema_field_parent_object_entity_name,
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
                variable_definitions: vec![],
                parent_entity_name: target_parent_object_entity_name,
                phantom_data: std::marker::PhantomData,
            };

            insert_selectable_or_multiple_definition_diagnostic(
                &mut outcome.selectables,
                (
                    target_parent_object_entity_name,
                    client_field_scalar_selection_name.unchecked_conversion(),
                ),
                mutation_client_scalar_selectable
                    .interned_value(db)
                    .scalar_selected()
                    .client_defined()
                    .with_generated_location(),
                &mut non_fatal_diagnostics,
            );

            outcome.client_scalar_refetch_strategies.push(
                (
                    target_parent_object_entity_name,
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

fn define_default_graphql_types(
    db: &IsographDatabase<GraphQLAndJavascriptProfile>,
    outcome: &mut ParseTypeSystemOutcome<GraphQLAndJavascriptProfile>,
    non_fatal_diagnostics: &mut Vec<Diagnostic>,
) {
    insert_entity_or_multiple_definition_diagnostic(
        &mut outcome.entities,
        *ID_ENTITY_NAME,
        ServerEntity {
            description: None,
            name: *ID_ENTITY_NAME,
            selection_info: ().scalar_selected(),
            network_protocol_associated_data: (),
            target_platform_associated_data: (*STRING_JAVASCRIPT_TYPE).scalar_selected(),
        }
        .interned_value(db)
        .with_generated_location(),
        non_fatal_diagnostics,
    );
    insert_entity_or_multiple_definition_diagnostic(
        &mut outcome.entities,
        *STRING_ENTITY_NAME,
        ServerEntity {
            description: None,
            name: *STRING_ENTITY_NAME,
            selection_info: ().scalar_selected(),
            network_protocol_associated_data: (),
            target_platform_associated_data: (*STRING_JAVASCRIPT_TYPE).scalar_selected(),
        }
        .interned_value(db)
        .with_generated_location(),
        non_fatal_diagnostics,
    );
    insert_entity_or_multiple_definition_diagnostic(
        &mut outcome.entities,
        *BOOLEAN_ENTITY_NAME,
        ServerEntity {
            description: None,
            name: *BOOLEAN_ENTITY_NAME,
            selection_info: ().scalar_selected(),
            network_protocol_associated_data: (),
            target_platform_associated_data: (*BOOLEAN_JAVASCRIPT_TYPE).scalar_selected(),
        }
        .interned_value(db)
        .with_generated_location(),
        non_fatal_diagnostics,
    );
    insert_entity_or_multiple_definition_diagnostic(
        &mut outcome.entities,
        *FLOAT_ENTITY_NAME,
        ServerEntity {
            description: None,
            name: *FLOAT_ENTITY_NAME,
            selection_info: ().scalar_selected(),
            network_protocol_associated_data: (),
            target_platform_associated_data: (*NUMBER_JAVASCRIPT_TYPE).scalar_selected(),
        }
        .interned_value(db)
        .with_generated_location(),
        non_fatal_diagnostics,
    );
    insert_entity_or_multiple_definition_diagnostic(
        &mut outcome.entities,
        *INT_ENTITY_NAME,
        ServerEntity {
            description: None,
            name: *INT_ENTITY_NAME,
            selection_info: ().scalar_selected(),
            network_protocol_associated_data: (),
            target_platform_associated_data: (*NUMBER_JAVASCRIPT_TYPE).scalar_selected(),
        }
        .interned_value(db)
        .with_generated_location(),
        non_fatal_diagnostics,
    );
}

// Defaults to false if item is missing... should this panic?
fn is_object_entity(
    db: &IsographDatabase<GraphQLAndJavascriptProfile>,
    entities: &EntitiesDefinedBySchema,
    target: EntityName,
) -> bool {
    entities
        .get(&target)
        .map(|entity| entity.item.lookup(db).selection_info.as_object().is_some())
        .unwrap_or(false)
}

fn traverse_selections_and_return_path<'a>(
    db: &'a IsographDatabase<GraphQLAndJavascriptProfile>,
    outcome: &'a ParseTypeSystemOutcome<GraphQLAndJavascriptProfile>,
    payload_object_entity_name: EntityName,
    primary_field_selection_name_parts: &[SelectableName],
) -> DiagnosticResult<(
    Vec<&'a ServerSelectable<GraphQLAndJavascriptProfile>>,
    &'a ServerEntity<GraphQLAndJavascriptProfile>,
)> {
    let mut current_entity = outcome
        .entities
        .get(&payload_object_entity_name)
        .ok_or_else(|| {
            Diagnostic::new(
                format!(
                    "Invalid @exposeField directive. Entity {} was not found.",
                    payload_object_entity_name
                ),
                None,
            )
        })?
        .item
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
        let selectable = outcome
            .selectables
            .get(&(current_entity.name, selection_name.dereference()))
            .and_then(|x| match x.item {
                DefinitionLocation::Server(s) => {
                    match s.lookup(db).is_inline_fragment.reference() {
                        SelectionType::Scalar(_) => None,
                        SelectionType::Object(_) => s.wrap_some(),
                    }
                }
                DefinitionLocation::Client(_) => None,
            })
            .ok_or_else(|| {
                Diagnostic::new(
                    format!(
                        "Invalid @exposeField directive. Field {} \
                        not found or is not an object field.",
                        selection_name
                    ),
                    None,
                )
            })?
            .lookup(db);

        let next_entity_name = selectable.target_entity_name.inner();

        current_entity = outcome
            .entities
            .get(&next_entity_name)
            .ok_or_else(|| {
                Diagnostic::new(
                    format!(
                        "Invalid @exposeField directive. Entity {} not found.",
                        next_entity_name
                    ),
                    None,
                )
            })?
            .item
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

type EntitiesDefinedBySchema =
    BTreeMap<EntityName, WithLocation<pico::MemoRef<ServerEntity<GraphQLAndJavascriptProfile>>>>;
