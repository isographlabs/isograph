use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use common_lang_types::{
    ClientScalarSelectableName, DescriptionValue, Diagnostic, DiagnosticResult, QueryExtraInfo,
    QueryOperationName, QueryText, ScalarSelectableName, SelectableName, ServerObjectEntityName,
    ServerObjectSelectableName, ServerScalarEntityName, ServerSelectableName, UnvalidatedTypeName,
    WithLocation, WithLocationPostfix, WithSpanPostfix,
};
use graphql_lang_types::from_graphql_directives;
use intern::Lookup;
use intern::string_key::Intern;
use isograph_lang_types::{
    Description, EmptyDirectiveSet, ObjectSelection, ScalarSelection, SelectionSet,
    SelectionTypePostfix, TypeAnnotation, UnionTypeAnnotation, UnionVariant, VariableDefinition,
};
use isograph_schema::{
    BOOLEAN_ENTITY_NAME, BOOLEAN_JAVASCRIPT_TYPE, ClientFieldVariant, ClientScalarSelectable,
    FLOAT_ENTITY_NAME, Format, ID_ENTITY_NAME, INT_ENTITY_NAME, ImperativelyLoadedFieldVariant,
    MemoRefServerEntity, MergedSelectionMap, NUMBER_JAVASCRIPT_TYPE, NetworkProtocol,
    ParseTypeSystemOutcome, RefetchStrategy, RootOperationName, STRING_ENTITY_NAME,
    STRING_JAVASCRIPT_TYPE, ServerObjectEntity, ServerObjectEntityDirectives,
    ServerObjectSelectable, ServerObjectSelectableVariant, ServerScalarSelectable,
    TYPENAME_FIELD_NAME, ValidatedVariableDefinition, WrappedSelectionMapSelection,
    generate_refetch_field_strategy, imperative_field_subfields_or_inline_fragments,
    multiple_selectable_definitions_found_diagnostic, server_object_entity_named,
    to_isograph_constant_value,
};
use isograph_schema::{IsographDatabase, ServerScalarEntity};
use pico_macros::memo;
use prelude::Postfix;

use crate::process_type_system_definition::{
    get_typename_selectable, multiple_entity_definitions_found_diagnostic,
    process_graphql_type_system_document, process_graphql_type_system_extension_document,
};
use crate::{parse_graphql_schema, query_text::generate_query_text};

pub(crate) struct GraphQLRootTypes {
    pub query: ServerObjectEntityName,
    pub mutation: ServerObjectEntityName,
    pub subscription: ServerObjectEntityName,
}

impl Default for GraphQLRootTypes {
    fn default() -> Self {
        Self {
            query: "Query".intern().into(),
            mutation: "Mutation".intern().into(),
            subscription: "Subscription".intern().into(),
        }
    }
}

impl From<GraphQLRootTypes> for BTreeMap<ServerObjectEntityName, RootOperationName> {
    fn from(val: GraphQLRootTypes) -> Self {
        let mut map = BTreeMap::new();
        map.insert(val.query, RootOperationName("query"));
        map.insert(val.mutation, RootOperationName("mutation"));
        map.insert(val.subscription, RootOperationName("subscription"));
        map
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct GraphQLNetworkProtocol {}

impl NetworkProtocol for GraphQLNetworkProtocol {
    type SchemaObjectAssociatedData = GraphQLSchemaObjectAssociatedData;

    #[memo]
    fn parse_type_system_documents(
        db: &IsographDatabase<Self>,
    ) -> DiagnosticResult<(
        ParseTypeSystemOutcome<Self>,
        // fetchable types
        BTreeMap<ServerObjectEntityName, RootOperationName>,
    )> {
        let mut outcome = ParseTypeSystemOutcome::default();
        define_default_graphql_types(db, &mut outcome)?;

        let mut graphql_root_types = None;
        let mut directives = HashMap::new();
        let mut fields_to_process = vec![];
        let mut supertype_to_subtype_map = BTreeMap::new();
        let mut interfaces_to_process = vec![];

        let (type_system_document, type_system_extension_documents) = parse_graphql_schema(db)
            .to_owned()
            .note_todo("Do not clone. Use a MemoRef.")?;

        process_graphql_type_system_document(
            db,
            type_system_document
                .to_owned(db)
                .note_todo("Do not clone. Use a MemoRef."),
            &mut graphql_root_types,
            &mut outcome,
            &mut directives,
            &mut fields_to_process,
            &mut supertype_to_subtype_map,
            &mut interfaces_to_process,
        )?;

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
            )?;
        }

        // We process interfaces later, because we need to know all of the subtypes that an interface
        // implements. In an ideal world, this info would not be part of the ServerObjectEntity struct,
        // and we should make that refactor.
        for with_location in interfaces_to_process {
            let interface_definition = with_location.item;
            let server_object_entity_name = interface_definition
                .name
                .item
                .to::<ServerObjectEntityName>();

            insert_entity_or_multiple_definition_diagnostic(
                &mut outcome.entities,
                server_object_entity_name.into(),
                ServerObjectEntity {
                    description: interface_definition.description.map(|description_value| {
                        description_value
                            .item
                            .unchecked_conversion::<DescriptionValue>()
                            .wrap(Description)
                    }),
                    name: server_object_entity_name,
                    concrete_type: None,
                    network_protocol_associated_data: GraphQLSchemaObjectAssociatedData {
                        original_definition_type: GraphQLSchemaOriginalDefinitionType::Interface,
                        subtypes: supertype_to_subtype_map
                            .get(&server_object_entity_name.into())
                            .cloned()
                            .unwrap_or_default(),
                    },
                }
                .interned_value(db)
                .object_selected()
                .with_location(with_location.location),
            )?;

            directives
                .entry(server_object_entity_name)
                .or_default()
                .extend(interface_definition.directives);

            for field in interface_definition.fields {
                fields_to_process.push((server_object_entity_name, field));
            }

            insert_selectable_or_multiple_definition_diagnostic(
                &mut outcome.server_selectables,
                (server_object_entity_name, (*TYPENAME_FIELD_NAME).into()),
                get_typename_selectable(
                    db,
                    server_object_entity_name,
                    with_location.location,
                    None,
                )
                .scalar_selected()
                .with_location(with_location.location),
            )?;

            // I don't think interface-to-interface refinement is handled correctly, let's just
            // ignore it for now.
        }

        // Note: we need to know whether a field points to an object entity or scalar entity, and we
        // do not have that information when we first encounter that field. So, we accumulate fields
        // and handle them now. A future refactor will get rid of this: selectables will all be the
        // the same struct, and you will have to do a follow up request for the target entity to
        // know whether it is an object or scalar selectable.
        for (parent_object_entity_name, field) in fields_to_process {
            let target: ServerObjectEntityName = (*field.item.type_.inner()).unchecked_conversion();

            if is_object_entity(&outcome.entities, target) {
                insert_selectable_or_multiple_definition_diagnostic(
                    &mut outcome.server_selectables,
                    (parent_object_entity_name, field.item.name.item.into()),
                    ServerObjectSelectable {
                        description: field
                            .item
                            .description
                            .map(|with_span| with_span.item.into()),
                        name: field.item.name.map(|name| name.unchecked_conversion()),
                        target_object_entity: TypeAnnotation::from_graphql_type_annotation(
                            field
                                .item
                                .type_
                                .clone()
                                .map(|entity_name| entity_name.unchecked_conversion()),
                        ),
                        object_selectable_variant: ServerObjectSelectableVariant::LinkedField,
                        parent_object_entity_name,
                        arguments: field
                            .item
                            .arguments
                            .into_iter()
                            .map(|with_location| {
                                with_location.map(|arg| VariableDefinition {
                                    name: arg.name.map(|input_value_name| {
                                        input_value_name.unchecked_conversion()
                                    }),
                                    type_: arg.type_.map(|input_type_name| {
                                        // Another linear scan!
                                        if is_object_entity(
                                            &outcome.entities,
                                            input_type_name.unchecked_conversion(),
                                        ) {
                                            input_type_name
                                                .unchecked_conversion::<ServerObjectEntityName>()
                                                .object_selected()
                                        } else {
                                            input_type_name
                                                .unchecked_conversion::<ServerScalarEntityName>()
                                                .scalar_selected()
                                        }
                                    }),
                                    default_value: arg.default_value.map(|with_location| {
                                        with_location.map(to_isograph_constant_value)
                                    }),
                                })
                            })
                            .collect(),
                        phantom_data: std::marker::PhantomData,
                    }
                    .interned_value(db)
                    .object_selected()
                    .with_location(field.location),
                )?;
            } else {
                insert_selectable_or_multiple_definition_diagnostic(
                    &mut outcome.server_selectables,
                    (parent_object_entity_name, field.item.name.item.into()),
                    ServerScalarSelectable {
                        description: field
                            .item
                            .description
                            .map(|with_span| with_span.item.into()),
                        name: field.item.name.map(|name| name.unchecked_conversion()),
                        parent_object_entity_name,
                        arguments: field
                            .item
                            .arguments
                            .into_iter()
                            .map(|with_location| {
                                with_location.map(|arg| VariableDefinition {
                                    name: arg.name.map(|input_value_name| {
                                        input_value_name.unchecked_conversion()
                                    }),
                                    type_: arg.type_.map(|input_type_name| {
                                        if is_object_entity(
                                            &outcome.entities,
                                            input_type_name.unchecked_conversion(),
                                        ) {
                                            input_type_name
                                                .unchecked_conversion::<ServerObjectEntityName>()
                                                .object_selected()
                                        } else {
                                            input_type_name
                                                .unchecked_conversion::<ServerScalarEntityName>()
                                                .scalar_selected()
                                        }
                                    }),
                                    default_value: arg.default_value.map(|with_location| {
                                        with_location.map(to_isograph_constant_value)
                                    }),
                                })
                            })
                            .collect(),
                        phantom_data: std::marker::PhantomData,
                        target_scalar_entity: TypeAnnotation::from_graphql_type_annotation(
                            field
                                .item
                                .type_
                                .clone()
                                .map(|entity_name| entity_name.unchecked_conversion()),
                        ),
                        javascript_type_override: None,
                    }
                    .interned_value(db)
                    .scalar_selected()
                    .with_location(field.location),
                )?;
            }
        }

        // asConcreteType fields
        for (abstract_parent_entity_name, concrete_child_entity_names) in supertype_to_subtype_map {
            for concrete_child_entity_name in concrete_child_entity_names.iter() {
                insert_selectable_or_multiple_definition_diagnostic(
                    &mut outcome.server_selectables,
                    (
                        abstract_parent_entity_name.unchecked_conversion(),
                        format!("as{concrete_child_entity_name}").intern().into(),
                    ),
                    ServerObjectSelectable {
                        description: format!(
                            "A client pointer for the {} type.",
                            concrete_child_entity_name
                        )
                        .intern()
                        .to::<DescriptionValue>()
                        .wrap(Description)
                        .wrap_some(),
                        name: format!("as{}", concrete_child_entity_name)
                            .intern()
                            .to::<ServerObjectSelectableName>()
                            .with_generated_location(),
                        target_object_entity: TypeAnnotation::Union(UnionTypeAnnotation {
                            variants: {
                                let mut variants = BTreeSet::new();
                                variants.insert(UnionVariant::Scalar(
                                    concrete_child_entity_name.unchecked_conversion(),
                                ));
                                variants
                            },
                            nullable: true,
                        }),
                        object_selectable_variant: ServerObjectSelectableVariant::InlineFragment,
                        parent_object_entity_name: abstract_parent_entity_name
                            .unchecked_conversion(),
                        arguments: vec![],
                        phantom_data: std::marker::PhantomData,
                    }
                    .interned_value(db)
                    .object_selected()
                    .with_generated_location(),
                )?;
            }
        }

        // exposeField directives -> fields
        for (parent_object_entity_name, directives) in directives {
            let result = from_graphql_directives::<ServerObjectEntityDirectives>(&directives)?;
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
                    .collect::<Vec<ServerSelectableName>>();

                let mutation_subfield_name: ServerObjectSelectableName = field.intern().into();

                let mutation_field = &outcome
                    .server_selectables
                    .values()
                    .filter_map(|x| x.item.as_object())
                    .find_map(|server_object_selectable| {
                        let object = server_object_selectable.lookup(db);
                        if object.name.item == mutation_subfield_name {
                            Some(object)
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| Diagnostic::new("Mutation field not found".to_string(), None))?;

                let payload_object_entity_name = *mutation_field.target_object_entity.inner();

                let client_field_scalar_selection_name = expose_field_directive
                    .expose_as
                    .unwrap_or(mutation_field.name.item.into());
                let top_level_schema_field_parent_object_entity_name =
                    mutation_field.parent_object_entity_name;
                let mutation_field_arguments = mutation_field.arguments.clone();

                let top_level_schema_field_concrete_type = outcome
                    .entities
                    .get(&payload_object_entity_name.into())
                    .and_then(|entity| entity.item.as_object())
                    .expect("Expected entity to exist and to be an object.")
                    .lookup(db)
                    .concrete_type;

                let (mut parts_reversed, target_parent_object_entity) =
                    traverse_selections_and_return_path(
                        db,
                        &outcome,
                        payload_object_entity_name,
                        &primary_field_name_selection_parts,
                    )?;
                let target_parent_object_entity_name = target_parent_object_entity.name;
                parts_reversed.reverse();

                let fields = expose_field_directive
                    .field_map
                    .iter()
                    .map(|field_map_item| {
                        ScalarSelection {
                            name: field_map_item
                                .from
                                .unchecked_conversion::<ScalarSelectableName>()
                                .with_generated_location(),
                            reader_alias: None,
                            associated_data: (),
                            arguments: vec![],
                            scalar_selection_directive_set:
                                isograph_lang_types::ScalarSelectionDirectiveSet::None(
                                    EmptyDirectiveSet {},
                                ),
                        }
                        .scalar_selected::<ObjectSelection<(), ()>>()
                        .with_generated_span()
                    })
                    .collect::<Vec<_>>();

                let top_level_schema_field_arguments = mutation_field_arguments
                    .into_iter()
                    .map(|variable_definition| variable_definition.item)
                    .collect::<Vec<_>>();

                let mut subfields_or_inline_fragments = parts_reversed
                    .iter()
                    .map(|server_object_selectable| {
                        match server_object_selectable.object_selectable_variant {
                            ServerObjectSelectableVariant::LinkedField => {
                                WrappedSelectionMapSelection::LinkedField {
                                    parent_object_entity_name: server_object_selectable
                                        .parent_object_entity_name,
                                    server_object_selectable_name: server_object_selectable
                                        .name
                                        .item,
                                    arguments: vec![],
                                    concrete_type: Some(target_parent_object_entity.name)
                                        .note_todo(
                                            "This is 100% a bug when there are \
                                            multiple items in parts_reversed, or this \
                                            field is ignored.",
                                        ),
                                }
                            }
                            ServerObjectSelectableVariant::InlineFragment => {
                                WrappedSelectionMapSelection::InlineFragment(
                                    *server_object_selectable.target_object_entity.inner(),
                                )
                            }
                        }
                    })
                    .collect::<Vec<_>>();

                subfields_or_inline_fragments.push(imperative_field_subfields_or_inline_fragments(
                    mutation_subfield_name,
                    &top_level_schema_field_arguments,
                    top_level_schema_field_concrete_type,
                    top_level_schema_field_parent_object_entity_name,
                ));

                let mutation_client_scalar_selectable = ClientScalarSelectable {
                    description: mutation_field.description,
                    name: client_field_scalar_selection_name
                        .unchecked_conversion::<ClientScalarSelectableName>()
                        .with_generated_location(),
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
                    parent_object_entity_name: target_parent_object_entity.name,
                    network_protocol: std::marker::PhantomData::<GraphQLNetworkProtocol>,
                };

                outcome.client_scalar_selectables.push(
                    mutation_client_scalar_selectable
                        .with_generated_location()
                        .wrap_ok(),
                );

                outcome.client_scalar_refetch_strategies.push(
                    (
                        target_parent_object_entity_name,
                        client_field_scalar_selection_name
                            .unchecked_conversion::<ClientScalarSelectableName>(),
                        RefetchStrategy::UseRefetchField(generate_refetch_field_strategy(
                            SelectionSet {
                                selections: fields.to_vec(),
                            }
                            .with_generated_span(),
                            parent_object_entity_name,
                            subfields_or_inline_fragments,
                        )),
                    )
                        .with_generated_location()
                        .wrap_ok(),
                )
            }
        }

        (outcome, graphql_root_types.unwrap_or_default().into()).wrap_ok()
    }

    fn generate_link_type<'a>(
        db: &IsographDatabase<Self>,
        server_object_entity_name: &ServerObjectEntityName,
    ) -> String {
        let server_object_entity = &server_object_entity_named(db, *server_object_entity_name)
            .as_ref()
            .expect(
                "Expected validation to have worked. \
                This is indicative of a bug in Isograph.",
            )
            .as_ref()
            .expect(
                "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
            )
            .lookup(db);

        if let Some(concrete_type) = server_object_entity.concrete_type {
            return format!("Link<\"{concrete_type}\">");
        }

        let subtypes = server_object_entity
            .network_protocol_associated_data
            .subtypes
            .iter()
            .map(|name| format!("\n  | Link<\"{name}\">"))
            .collect::<Vec<_>>();

        subtypes.join("")
    }

    fn generate_query_text<'a>(
        db: &IsographDatabase<Self>,
        query_name: QueryOperationName,
        selection_map: &MergedSelectionMap,
        query_variables: impl Iterator<Item = &'a ValidatedVariableDefinition> + 'a,
        root_operation_name: &RootOperationName,
        format: Format,
    ) -> QueryText {
        generate_query_text(
            db,
            query_name,
            selection_map,
            query_variables,
            root_operation_name,
            format,
        )
    }

    fn generate_query_extra_info(
        query_name: QueryOperationName,
        operation_name: ServerObjectEntityName,
        indentation_level: u8,
    ) -> QueryExtraInfo {
        let indent = "  ".repeat((indentation_level + 1) as usize);
        QueryExtraInfo(format!(
            "{{\n\
            {indent}  kind: \"PersistedOperationExtraInfo\",\n\
            {indent}  operationName: \"{query_name}\",\n\
            {indent}  operationKind: \"{operation_name}\",\n\
            {indent}}}"
        ))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct GraphQLSchemaObjectAssociatedData {
    pub original_definition_type: GraphQLSchemaOriginalDefinitionType,
    // TODO expose this as a separate memoized method
    pub subtypes: Vec<UnvalidatedTypeName>,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum GraphQLSchemaOriginalDefinitionType {
    InputObject,
    Object,
    Interface,
    Union,
}

impl GraphQLSchemaOriginalDefinitionType {
    pub fn sdl_keyword(&self) -> &'static str {
        match self {
            GraphQLSchemaOriginalDefinitionType::InputObject => "input",
            GraphQLSchemaOriginalDefinitionType::Object => "object",
            GraphQLSchemaOriginalDefinitionType::Interface => "interface",
            GraphQLSchemaOriginalDefinitionType::Union => "union",
        }
    }
}

fn define_default_graphql_types(
    db: &IsographDatabase<GraphQLNetworkProtocol>,
    outcome: &mut ParseTypeSystemOutcome<GraphQLNetworkProtocol>,
) -> DiagnosticResult<()> {
    insert_entity_or_multiple_definition_diagnostic(
        &mut outcome.entities,
        (*ID_ENTITY_NAME).into(),
        ServerScalarEntity {
            description: None,
            name: *ID_ENTITY_NAME,
            javascript_name: "string".intern().into(),
            network_protocol: std::marker::PhantomData,
        }
        .interned_value(db)
        .scalar_selected()
        .with_generated_location(),
    )?;
    insert_entity_or_multiple_definition_diagnostic(
        &mut outcome.entities,
        (*STRING_ENTITY_NAME).into(),
        ServerScalarEntity {
            description: None,
            name: *STRING_ENTITY_NAME,
            javascript_name: *STRING_JAVASCRIPT_TYPE,
            network_protocol: std::marker::PhantomData,
        }
        .interned_value(db)
        .scalar_selected()
        .with_generated_location(),
    )?;
    insert_entity_or_multiple_definition_diagnostic(
        &mut outcome.entities,
        (*BOOLEAN_ENTITY_NAME).into(),
        ServerScalarEntity {
            description: None,
            name: *BOOLEAN_ENTITY_NAME,
            javascript_name: *BOOLEAN_JAVASCRIPT_TYPE,
            network_protocol: std::marker::PhantomData,
        }
        .interned_value(db)
        .scalar_selected()
        .with_generated_location(),
    )?;
    insert_entity_or_multiple_definition_diagnostic(
        &mut outcome.entities,
        (*FLOAT_ENTITY_NAME).into(),
        ServerScalarEntity {
            description: None,
            name: *FLOAT_ENTITY_NAME,
            javascript_name: *NUMBER_JAVASCRIPT_TYPE,
            network_protocol: std::marker::PhantomData,
        }
        .interned_value(db)
        .scalar_selected()
        .with_generated_location(),
    )?;
    insert_entity_or_multiple_definition_diagnostic(
        &mut outcome.entities,
        (*INT_ENTITY_NAME).into(),
        ServerScalarEntity {
            description: None,
            name: *INT_ENTITY_NAME,
            javascript_name: *NUMBER_JAVASCRIPT_TYPE,
            network_protocol: std::marker::PhantomData,
        }
        .interned_value(db)
        .scalar_selected()
        .with_generated_location(),
    )?;

    ().wrap_ok()
}

fn is_object_entity(
    entities: &BTreeMap<
        UnvalidatedTypeName,
        WithLocation<MemoRefServerEntity<GraphQLNetworkProtocol>>,
    >,
    target: ServerObjectEntityName,
) -> bool {
    entities
        .get(&target.into())
        .and_then(|entity| entity.item.as_object())
        .is_some()
}

fn traverse_selections_and_return_path<'a>(
    db: &'a IsographDatabase<GraphQLNetworkProtocol>,
    outcome: &'a ParseTypeSystemOutcome<GraphQLNetworkProtocol>,
    payload_object_entity_name: ServerObjectEntityName,
    primary_field_selection_name_parts: &[ServerSelectableName],
) -> DiagnosticResult<(
    Vec<&'a ServerObjectSelectable<GraphQLNetworkProtocol>>,
    &'a ServerObjectEntity<GraphQLNetworkProtocol>,
)> {
    // TODO do not do a linear scan
    let mut current_entity = outcome
        .entities
        .get(&payload_object_entity_name.into())
        .and_then(|entity| entity.item.as_object())
        .ok_or_else(|| {
            Diagnostic::new(
                format!(
                    "Invalid @exposeField directive. Entity {} \
                    not found or is not an object.",
                    payload_object_entity_name
                ),
                None,
            )
        })?
        .lookup(db);

    let mut output = vec![];

    for selection_name in primary_field_selection_name_parts {
        let selectable = outcome
            .server_selectables
            .get(&(current_entity.name, selection_name.dereference().into()))
            .and_then(|x| x.item.as_object())
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

        let next_entity_name = selectable.target_object_entity.inner().dereference().into();

        current_entity = outcome
            .entities
            .get(&next_entity_name)
            .and_then(|entity| entity.item.as_object())
            .ok_or_else(|| {
                Diagnostic::new(
                    format!(
                        "Invalid @exposeField directive. Entity {} \
                        not found or is a not an object.",
                        next_entity_name
                    ),
                    None,
                )
            })?
            .lookup(db);

        output.push(selectable);
    }

    (output, current_entity).wrap_ok()
}

// TODO make this generic over value, too
pub(crate) fn insert_entity_or_multiple_definition_diagnostic<Value>(
    map: &mut BTreeMap<UnvalidatedTypeName, WithLocation<Value>>,
    key: UnvalidatedTypeName,
    item: WithLocation<Value>,
) -> DiagnosticResult<()> {
    match map.entry(key) {
        Entry::Vacant(vacant_entry) => {
            vacant_entry.insert(item);
            ().wrap_ok()
        }
        Entry::Occupied(_) => {
            multiple_entity_definitions_found_diagnostic(key, item.location.wrap_some()).wrap_err()
        }
    }
}

// TODO make this generic over value, too
pub(crate) fn insert_selectable_or_multiple_definition_diagnostic<Value>(
    map: &mut BTreeMap<(ServerObjectEntityName, SelectableName), WithLocation<Value>>,
    key: (ServerObjectEntityName, SelectableName),
    item: WithLocation<Value>,
) -> DiagnosticResult<()> {
    match map.entry(key) {
        Entry::Vacant(vacant_entry) => {
            vacant_entry.insert(item);
            ().wrap_ok()
        }
        Entry::Occupied(_) => {
            multiple_selectable_definitions_found_diagnostic(key.0, key.1, item.location).wrap_err()
        }
    }
}
