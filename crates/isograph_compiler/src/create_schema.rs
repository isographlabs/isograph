use std::{
    collections::HashMap,
    error::Error,
    ops::{Deref, DerefMut},
};

use common_lang_types::{
    RelativePathToSourceFile, SelectableName, ServerObjectEntityName, TextSource,
    UnvalidatedTypeName, VariableName, WithLocation,
};
use graphql_lang_types::{
    GraphQLConstantValue, GraphQLInputValueDefinition, NameValuePair, RootOperationKind,
};
use isograph_config::CompilerConfigOptions;
use isograph_lang_parser::IsoLiteralExtractionResult;
use isograph_lang_types::{
    ConstantValue, IsoLiteralsSource, SelectionType, TypeAnnotation, VariableDefinition,
};
use isograph_schema::{
    validate_entrypoints, CreateAdditionalFieldsError, FieldToInsert, NetworkProtocol,
    ProcessObjectTypeDefinitionOutcome, ProcessTypeSystemDocumentOutcome, RootOperationName,
    Schema, ServerEntityName, ServerObjectSelectable, ServerObjectSelectableVariant,
    ServerScalarSelectable,
};
use pico::{Database, SourceId};

use crate::{
    add_selection_sets::add_selection_sets_to_client_selectables,
    batch_compile::BatchCompileError,
    db_singletons::get_isograph_config,
    isograph_literals::{parse_iso_literal_in_source, process_iso_literals},
};

pub fn create_schema<TNetworkProtocol: NetworkProtocol>(
    db: &Database,
    sources: &TNetworkProtocol::Sources,
    iso_literals: &HashMap<RelativePathToSourceFile, SourceId<IsoLiteralsSource>>,
) -> Result<(Schema<TNetworkProtocol>, ContainsIsoStats), Box<dyn Error>> {
    let config = get_isograph_config(db);
    let ProcessTypeSystemDocumentOutcome { scalars, objects } =
        TNetworkProtocol::parse_and_process_type_system_documents(db, sources)?;

    let mut unvalidated_isograph_schema = Schema::<TNetworkProtocol>::new();
    for (server_scalar_entity, name_location) in scalars {
        unvalidated_isograph_schema
            .server_entity_data
            .insert_server_scalar_entity(server_scalar_entity, name_location)?;
    }

    let mut field_queue = HashMap::new();
    let mut expose_as_field_queue = HashMap::new();
    for (
        ProcessObjectTypeDefinitionOutcome {
            encountered_root_kind,
            server_object_entity,
            fields_to_insert,
            expose_as_fields_to_insert,
        },
        name_location,
    ) in objects
    {
        let new_object_id = unvalidated_isograph_schema
            .server_entity_data
            .insert_server_object_entity(server_object_entity, name_location)?;
        field_queue.insert(new_object_id, fields_to_insert);

        match encountered_root_kind {
            Some(RootOperationKind::Query) => {
                unvalidated_isograph_schema
                    .fetchable_types
                    .insert(new_object_id, RootOperationName("query".to_string()));
            }
            Some(RootOperationKind::Mutation) => {
                unvalidated_isograph_schema
                    .fetchable_types
                    .insert(new_object_id, RootOperationName("mutation".to_string()));
            }
            Some(RootOperationKind::Subscription) => {
                unvalidated_isograph_schema
                    .fetchable_types
                    .insert(new_object_id, RootOperationName("subscription".to_string()));
            }
            None => {}
        }

        expose_as_field_queue.insert(new_object_id, expose_as_fields_to_insert);
    }

    process_field_queue(
        &mut unvalidated_isograph_schema,
        field_queue,
        &config.options,
    )?;

    // Step one: we can create client selectables. However, we must create all
    // client selectables before being able to create their selection sets, because
    // selection sets refer to client selectables. We hold onto these selection sets
    // (both reader selection sets and refetch selection sets) in the unprocess_items
    // vec, then process it later.
    let mut unprocessed_items = vec![];

    for (parent_object_entity_name, expose_as_fields_to_insert) in expose_as_field_queue {
        for expose_as_field in expose_as_fields_to_insert {
            let unprocessed_scalar_item = unvalidated_isograph_schema
                .create_new_exposed_field(expose_as_field, parent_object_entity_name)?;

            unprocessed_items.push(SelectionType::Scalar(unprocessed_scalar_item));
        }
    }

    let contains_iso = parse_iso_literals(db, iso_literals)?;
    let contains_iso_stats = contains_iso.stats();

    let (unprocessed_client_types, unprocessed_entrypoints) =
        process_iso_literals(&mut unvalidated_isograph_schema, contains_iso)?;
    unprocessed_items.extend(unprocessed_client_types);

    unvalidated_isograph_schema.add_link_fields()?;

    unvalidated_isograph_schema.entrypoints = validate_entrypoints(
        &unvalidated_isograph_schema,
        unprocessed_entrypoints,
    )
    .map_err(|e| BatchCompileError::MultipleErrorsWithLocations {
        messages: e
            .into_iter()
            .map(|x| WithLocation::new(Box::new(x.item) as Box<dyn std::error::Error>, x.location))
            .collect(),
    })?;

    // Step two: now, we can create the selection sets. Creating a selection set involves
    // looking up client selectables, to:
    // - determine if the selectable exists,
    // - to determine if we are selecting it appropriately (e.g. client fields as scalars, etc)
    // - to validate arguments (e.g. no missing arguments, etc.)
    // - validate loadability/updatability, and
    // - to store the selectable id,
    add_selection_sets_to_client_selectables(&mut unvalidated_isograph_schema, unprocessed_items)
        .map_err(|messages| BatchCompileError::MultipleErrorsWithLocations {
        messages: messages
            .into_iter()
            .map(|x| WithLocation::new(Box::new(x.item) as Box<dyn std::error::Error>, x.location))
            .collect(),
    })?;

    Ok((unvalidated_isograph_schema, contains_iso_stats))
}

fn parse_iso_literals(
    db: &Database,
    iso_literals_sources: &HashMap<RelativePathToSourceFile, SourceId<IsoLiteralsSource>>,
) -> Result<ContainsIso, BatchCompileError> {
    let mut contains_iso = ContainsIso::default();
    let mut iso_literal_parse_errors = vec![];
    for (relative_path, iso_literals_source_id) in iso_literals_sources.iter() {
        match parse_iso_literal_in_source(db, *iso_literals_source_id).to_owned() {
            Ok(iso_literals) => {
                if !iso_literals.is_empty() {
                    contains_iso.insert(*relative_path, iso_literals);
                }
            }
            Err(e) => {
                iso_literal_parse_errors.extend(e);
            }
        };
    }
    if iso_literal_parse_errors.is_empty() {
        Ok(contains_iso)
    } else {
        Err(iso_literal_parse_errors.into())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct ContainsIso {
    pub files: HashMap<RelativePathToSourceFile, Vec<(IsoLiteralExtractionResult, TextSource)>>,
}

impl ContainsIso {
    pub fn stats(&self) -> ContainsIsoStats {
        let mut client_field_count: usize = 0;
        let mut client_pointer_count: usize = 0;
        let mut entrypoint_count: usize = 0;
        for iso_literals in self.values() {
            for (iso_literal, ..) in iso_literals {
                match iso_literal {
                    IsoLiteralExtractionResult::ClientFieldDeclaration(_) => {
                        client_field_count += 1
                    }
                    IsoLiteralExtractionResult::EntrypointDeclaration(_) => entrypoint_count += 1,
                    IsoLiteralExtractionResult::ClientPointerDeclaration(_) => {
                        client_pointer_count += 1
                    }
                }
            }
        }
        ContainsIsoStats {
            client_field_count,
            entrypoint_count,
            client_pointer_count,
        }
    }
}

impl Deref for ContainsIso {
    type Target = HashMap<RelativePathToSourceFile, Vec<(IsoLiteralExtractionResult, TextSource)>>;

    fn deref(&self) -> &Self::Target {
        &self.files
    }
}

impl DerefMut for ContainsIso {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.files
    }
}

pub struct ContainsIsoStats {
    pub client_field_count: usize,
    pub entrypoint_count: usize,
    #[allow(unused)]
    pub client_pointer_count: usize,
}

/// Now that we have processed all objects and scalars, we can process fields (i.e.
/// selectables), as we have the knowledge of whether the field points to a scalar
/// or object.
///
/// For each field:
/// - insert it into to the parent object's encountered_fields
/// - append it to schema.server_fields
/// - if it is an id field, modify the parent object
fn process_field_queue<TNetworkProtocol: NetworkProtocol>(
    schema: &mut Schema<TNetworkProtocol>,
    field_queue: HashMap<ServerObjectEntityName, Vec<WithLocation<FieldToInsert>>>,
    options: &CompilerConfigOptions,
) -> Result<(), WithLocation<CreateAdditionalFieldsError>> {
    for (parent_object_entity_name, field_definitions_to_insert) in field_queue {
        for server_field_to_insert in field_definitions_to_insert.into_iter() {
            let parent_object_entity = schema
                .server_entity_data
                .server_object_entity(parent_object_entity_name);

            let target_entity_type_name = server_field_to_insert.item.type_.inner();

            let selection_type = schema
                .server_entity_data
                .defined_entities
                .get(target_entity_type_name)
                .ok_or_else(|| {
                    WithLocation::new(
                        CreateAdditionalFieldsError::FieldTypenameDoesNotExist {
                            target_entity_type_name: *target_entity_type_name,
                        },
                        server_field_to_insert.item.name.location,
                    )
                })?;

            let arguments = server_field_to_insert
                .item
                .arguments
                // TODO don't clone
                .clone()
                .into_iter()
                .map(|input_value_definition| {
                    graphql_input_value_definition_to_variable_definition(
                        &schema.server_entity_data.defined_entities,
                        input_value_definition,
                        parent_object_entity.name,
                        server_field_to_insert.item.name.item.into(),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            let description = server_field_to_insert.item.description.map(|d| d.item);

            match selection_type {
                SelectionType::Scalar(scalar_entity_name) => {
                    schema
                        .insert_server_scalar_selectable(
                            ServerScalarSelectable {
                                description,
                                name: server_field_to_insert
                                    .item
                                    .name
                                    .map(|x| x.unchecked_conversion()),
                                target_scalar_entity: TypeAnnotation::from_graphql_type_annotation(
                                    server_field_to_insert.item.type_.clone(),
                                )
                                .map(&mut |_| *scalar_entity_name),
                                parent_object_entity_name,
                                arguments,
                                phantom_data: std::marker::PhantomData,
                            },
                            options,
                            server_field_to_insert
                                .item
                                .type_
                                .inner_non_null_named_type(),
                        )
                        .map_err(|e| WithLocation::new(e, server_field_to_insert.location))?;
                }
                SelectionType::Object(object_entity_name) => {
                    schema
                        .insert_server_object_selectable(ServerObjectSelectable {
                            description,
                            name: server_field_to_insert.item.name.map(|x| x.unchecked_conversion()),
                            target_object_entity: TypeAnnotation::from_graphql_type_annotation(
                                server_field_to_insert.item.type_.clone(),
                            )
                            .map(&mut |_| *object_entity_name),
                            parent_object_name: parent_object_entity_name,
                            arguments,
                            phantom_data: std::marker::PhantomData,
                            object_selectable_variant:
                                // TODO this is hacky
                                if server_field_to_insert.item.is_inline_fragment {
                                    ServerObjectSelectableVariant::InlineFragment
                                } else {
                                    ServerObjectSelectableVariant::LinkedField
                                }
                        })
                        .map_err(|e| WithLocation::new(e, server_field_to_insert.location))?;
                }
            }
        }
    }

    Ok(())
}

pub fn graphql_input_value_definition_to_variable_definition(
    defined_types: &HashMap<UnvalidatedTypeName, ServerEntityName>,
    input_value_definition: WithLocation<GraphQLInputValueDefinition>,
    parent_type_name: ServerObjectEntityName,
    field_name: SelectableName,
) -> Result<
    WithLocation<VariableDefinition<ServerEntityName>>,
    WithLocation<CreateAdditionalFieldsError>,
> {
    let default_value = input_value_definition
        .item
        .default_value
        .map(|graphql_constant_value| {
            Ok::<_, WithLocation<CreateAdditionalFieldsError>>(WithLocation::new(
                convert_graphql_constant_value_to_isograph_constant_value(
                    graphql_constant_value.item,
                ),
                graphql_constant_value.location,
            ))
        })
        .transpose()?;

    let type_ = input_value_definition
        .item
        .type_
        .clone()
        .and_then(|input_type_name| {
            defined_types
                .get(&(*input_value_definition.item.type_.inner()).into())
                .ok_or_else(|| {
                    WithLocation::new(
                        CreateAdditionalFieldsError::FieldArgumentTypeDoesNotExist {
                            argument_type: input_type_name.into(),
                            argument_name: input_value_definition.item.name.item.into(),
                            parent_type_name,
                            field_name,
                        },
                        input_value_definition.location,
                    )
                })
                .copied()
        })?;

    Ok(WithLocation::new(
        VariableDefinition {
            name: input_value_definition.item.name.map(VariableName::from),
            type_,
            default_value,
        },
        input_value_definition.location,
    ))
}

fn convert_graphql_constant_value_to_isograph_constant_value(
    graphql_constant_value: GraphQLConstantValue,
) -> ConstantValue {
    match graphql_constant_value {
        GraphQLConstantValue::Int(i) => ConstantValue::Integer(i),
        GraphQLConstantValue::Boolean(b) => ConstantValue::Boolean(b),
        GraphQLConstantValue::String(s) => ConstantValue::String(s),
        GraphQLConstantValue::Float(f) => ConstantValue::Float(f),
        GraphQLConstantValue::Null => ConstantValue::Null,
        GraphQLConstantValue::Enum(e) => ConstantValue::Enum(e),
        GraphQLConstantValue::List(l) => {
            let converted_list = l
                .into_iter()
                .map(|x| {
                    WithLocation::new(
                        convert_graphql_constant_value_to_isograph_constant_value(x.item),
                        x.location,
                    )
                })
                .collect::<Vec<_>>();
            ConstantValue::List(converted_list)
        }
        GraphQLConstantValue::Object(o) => {
            let converted_object = o
                .into_iter()
                .map(|name_value_pair| NameValuePair {
                    name: name_value_pair.name,
                    value: WithLocation::new(
                        convert_graphql_constant_value_to_isograph_constant_value(
                            name_value_pair.value.item,
                        ),
                        name_value_pair.value.location,
                    ),
                })
                .collect::<Vec<_>>();
            ConstantValue::Object(converted_object)
        }
    }
}
