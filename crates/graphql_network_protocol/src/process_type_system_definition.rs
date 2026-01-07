use std::collections::{BTreeMap, HashMap};

use common_lang_types::{
    DescriptionValue, Diagnostic, EmbeddedLocation, EntityName, Location, SelectableName,
    VariableName, WithEmbeddedLocation, WithLocationPostfix,
};
use graphql_lang_types::{
    GraphQLConstantValue, GraphQLDirective, GraphQLInterfaceTypeDefinition,
    GraphQLTypeSystemDefinition, GraphQLTypeSystemDocument, GraphQLTypeSystemExtension,
    GraphQLTypeSystemExtensionDocument, GraphQLTypeSystemExtensionOrDefinition,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    ArgumentKeyAndValue, Description, EmptyDirectiveSet, NonConstantValue, ScalarSelection,
    ScalarSelectionDirectiveSet, SelectionSet, SelectionTypePostfix, TypeAnnotationDeclaration,
    VariableDeclaration, VariableNameWrapper,
};
use isograph_schema::{
    ClientFieldVariant, ClientScalarSelectable, DeprecatedParseTypeSystemOutcome, FieldMapItem,
    ID_ENTITY_NAME, ID_FIELD_NAME, ID_VARIABLE_NAME, ImperativelyLoadedFieldVariant,
    IsographDatabase, NODE_FIELD_NAME, RefetchStrategy, WrappedSelectionMapSelection,
    generate_refetch_field_strategy, insert_selectable_or_multiple_definition_diagnostic,
};
use lazy_static::lazy_static;
use prelude::Postfix;

use crate::{GraphQLAndJavascriptProfile, GraphQLRootTypes};

lazy_static! {
    // TODO use schema_data.string_type_id or something
    static ref STRING_TYPE_NAME: EntityName = "String".intern().into();
    static ref NODE_INTERFACE_NAME: EntityName= "Node".intern().into();
    pub static ref REFETCH_FIELD_NAME: SelectableName = "__refetch".intern().into();

}

#[expect(clippy::too_many_arguments)]
pub fn process_graphql_type_system_document(
    db: &IsographDatabase<GraphQLAndJavascriptProfile>,
    type_system_document: GraphQLTypeSystemDocument,
    graphql_root_types: &mut Option<GraphQLRootTypes>,
    outcome: &mut DeprecatedParseTypeSystemOutcome<GraphQLAndJavascriptProfile>,
    directives: &mut HashMap<EntityName, Vec<GraphQLDirective<GraphQLConstantValue>>>,
    interfaces_to_process: &mut Vec<WithEmbeddedLocation<GraphQLInterfaceTypeDefinition>>,
    non_fatal_diagnostics: &mut Vec<Diagnostic>,
) {
    for with_location in type_system_document.0 {
        let WithEmbeddedLocation {
            location,
            item: type_system_definition,
        } = with_location;
        match type_system_definition {
            GraphQLTypeSystemDefinition::ObjectTypeDefinition(object_type_definition) => {
                let server_object_entity_name = object_type_definition.name.item.to::<EntityName>();

                let _typename_entity_name = format!("{}__discriminator", server_object_entity_name)
                    .intern()
                    .to::<EntityName>()
                    // And make it not selectable!
                    .note_todo("Come up with a way to not have these be in the same namespace");

                directives
                    .entry(server_object_entity_name)
                    .or_default()
                    .extend(object_type_definition.directives);

                let mut has_id_field = false;
                for field in object_type_definition.fields {
                    if field.item.name.item == *ID_FIELD_NAME {
                        has_id_field = true;
                    }
                }

                let subfields_or_inline_fragments = vec![
                    WrappedSelectionMapSelection::InlineFragment(server_object_entity_name),
                    WrappedSelectionMapSelection::LinkedField {
                        // TODO this should be query
                        parent_object_entity_name: server_object_entity_name,
                        server_object_selectable_name: *NODE_FIELD_NAME,
                        arguments: vec![ArgumentKeyAndValue {
                            key: (*ID_FIELD_NAME).unchecked_conversion(),
                            value: NonConstantValue::Variable((*ID_VARIABLE_NAME).into()),
                        }],
                        // None -> node is not concrete.
                        // Note that this doesn't matter!
                        concrete_target_entity_name: None,
                    },
                ];

                // TODO do this if the type implements Node instead
                if has_id_field {
                    insert_selectable_or_multiple_definition_diagnostic(
                        &mut outcome.selectables,
                        (server_object_entity_name, (*REFETCH_FIELD_NAME)),
                        get_refetch_selectable(
                            server_object_entity_name,
                            subfields_or_inline_fragments.clone(),
                        )
                        .interned_value(db)
                        .scalar_selected()
                        .with_generated_location(),
                        non_fatal_diagnostics,
                    );

                    outcome.client_scalar_refetch_strategies.push(
                        (
                            server_object_entity_name,
                            *REFETCH_FIELD_NAME,
                            refetch_selectable_refetch_strategy(subfields_or_inline_fragments),
                        )
                            .with_generated_location()
                            .wrap_ok(),
                    );
                }
            }
            GraphQLTypeSystemDefinition::ScalarTypeDefinition(_scalar_type_definition) => {}
            GraphQLTypeSystemDefinition::InterfaceTypeDefinition(interface_definition) => {
                interfaces_to_process.push(interface_definition.with_location(location));
            }
            GraphQLTypeSystemDefinition::InputObjectTypeDefinition(input_object_definition) => {
                let server_object_entity_name =
                    input_object_definition.name.item.to::<EntityName>();

                directives
                    .entry(server_object_entity_name)
                    .or_default()
                    .extend(input_object_definition.directives);

                // inputs do not implement interfaces
                // nor have typenames
            }
            GraphQLTypeSystemDefinition::DirectiveDefinition(_) => {
                // For now, Isograph ignores directive definitions,
                // but it might choose to allow-list them.
            }
            GraphQLTypeSystemDefinition::EnumDefinition(_enum_definition) => {}
            GraphQLTypeSystemDefinition::UnionTypeDefinition(union_definition) => {
                let server_object_entity_name = union_definition.name.item.to::<EntityName>();

                directives
                    .entry(server_object_entity_name)
                    .or_default()
                    .extend(union_definition.directives);

                // unions do not implement interfaces
            }
            GraphQLTypeSystemDefinition::SchemaDefinition(schema_definition) => {
                if graphql_root_types.is_some() {
                    non_fatal_diagnostics.push(Diagnostic::new(
                        "Duplicate schema definition".to_string(),
                        location.to::<Location>().wrap_some(),
                    ));
                    continue;
                }
                *graphql_root_types = GraphQLRootTypes {
                    query: schema_definition
                        .query
                        .map(|entity_name| entity_name.item)
                        .unwrap_or_else(|| "Query".intern().into()),
                    mutation: schema_definition
                        .mutation
                        .map(|entity_name| entity_name.item)
                        .unwrap_or_else(|| "Mutation".intern().into()),
                    subscription: schema_definition
                        .subscription
                        .map(|entity_name| entity_name.item)
                        .unwrap_or_else(|| "Subscription".intern().into()),
                }
                .wrap_some()
            }
        }
    }
}

fn refetch_selectable_refetch_strategy(
    subfields_or_inline_fragments: Vec<WrappedSelectionMapSelection>,
) -> RefetchStrategy {
    RefetchStrategy::UseRefetchField(generate_refetch_field_strategy(
        SelectionSet {
            selections: vec![
                ScalarSelection {
                    name: (*ID_FIELD_NAME)
                        .to::<SelectableName>()
                        .with_location(EmbeddedLocation::todo_generated()),
                    reader_alias: None,
                    arguments: vec![],
                    scalar_selection_directive_set: ScalarSelectionDirectiveSet::None(
                        EmptyDirectiveSet {},
                    ),
                }
                .scalar_selected()
                .with_location(EmbeddedLocation::todo_generated()),
            ],
        }
        .with_location(EmbeddedLocation::todo_generated()),
        // TODO use the type from the schema
        "Query".intern().into(),
        subfields_or_inline_fragments,
    ))
}

fn get_refetch_selectable(
    server_object_entity_name: EntityName,
    subfields_or_inline_fragments: Vec<WrappedSelectionMapSelection>,
) -> ClientScalarSelectable<GraphQLAndJavascriptProfile> {
    ClientScalarSelectable {
        description: format!(
            "A refetch field for the {} type.",
            server_object_entity_name
        )
        .intern()
        .to::<DescriptionValue>()
        .wrap(Description)
        .with_no_location()
        .wrap_some(),
        name: (*REFETCH_FIELD_NAME),
        variant: ClientFieldVariant::ImperativelyLoadedField(ImperativelyLoadedFieldVariant {
            client_selection_name: (*REFETCH_FIELD_NAME),
            // TODO use the actual schema query type
            root_object_entity_name: "Query".intern().into(),
            subfields_or_inline_fragments,
            field_map: vec![FieldMapItem {
                from: (*ID_FIELD_NAME).unchecked_conversion(),
                to: (*ID_FIELD_NAME).unchecked_conversion(),
            }],
            top_level_schema_field_arguments: vec![VariableDeclaration {
                name: (*ID_FIELD_NAME)
                    .unchecked_conversion::<VariableName>()
                    .to::<VariableNameWrapper>()
                    .with_location(EmbeddedLocation::todo_generated()),
                type_: TypeAnnotationDeclaration::Scalar((*ID_ENTITY_NAME).into())
                    .with_location(EmbeddedLocation::todo_generated()),
                default_value: None,
            }],
        }),
        variable_definitions: vec![],
        parent_entity_name: server_object_entity_name,
        phantom_data: std::marker::PhantomData,
    }
}

#[expect(clippy::too_many_arguments)]
pub fn process_graphql_type_system_extension_document(
    db: &IsographDatabase<GraphQLAndJavascriptProfile>,
    extension_document: GraphQLTypeSystemExtensionDocument,
    graphql_root_types: &mut Option<GraphQLRootTypes>,
    outcome: &mut DeprecatedParseTypeSystemOutcome<GraphQLAndJavascriptProfile>,
    directives: &mut HashMap<EntityName, Vec<GraphQLDirective<GraphQLConstantValue>>>,
    interfaces_to_process: &mut Vec<WithEmbeddedLocation<GraphQLInterfaceTypeDefinition>>,
    non_fatal_diagnostics: &mut Vec<Diagnostic>,
) {
    let mut definitions = Vec::with_capacity(extension_document.0.len());
    let mut extensions = Vec::with_capacity(extension_document.0.len());

    for extension_or_definition in extension_document.0 {
        let WithEmbeddedLocation { location, item } = extension_or_definition;
        match item {
            GraphQLTypeSystemExtensionOrDefinition::Definition(definition) => {
                definitions.push(definition.with_location(location));
            }
            GraphQLTypeSystemExtensionOrDefinition::Extension(extension) => {
                extensions.push(extension.with_location(location))
            }
        }
    }

    process_graphql_type_system_document(
        db,
        GraphQLTypeSystemDocument(definitions),
        graphql_root_types,
        outcome,
        directives,
        interfaces_to_process,
        non_fatal_diagnostics,
    );

    for extension in extensions {
        match extension.item {
            GraphQLTypeSystemExtension::ObjectTypeExtension(object_type_extension) => {
                directives
                    .entry(object_type_extension.name.item.unchecked_conversion())
                    .or_default()
                    .extend(object_type_extension.directives);
            }
        }
    }
}

pub type UnvalidatedTypeRefinementMap = BTreeMap<EntityName, Vec<EntityName>>;

pub fn multiple_entity_definitions_found_diagnostic(
    server_object_entity_name: EntityName,
    location: Option<Location>,
) -> Diagnostic {
    Diagnostic::new(
        format!("Multiple definitions of `{server_object_entity_name}` were found."),
        location,
    )
}
