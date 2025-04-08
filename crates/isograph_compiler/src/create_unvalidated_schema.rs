use std::{
    collections::HashMap,
    error::Error,
    ops::{Deref, DerefMut},
};

use common_lang_types::{
    CurrentWorkingDirectory, IsographObjectTypeName, Location, RelativePathToSourceFile,
    SelectableName, TextSource, UnvalidatedTypeName, VariableName, WithLocation,
};
use graphql_lang_types::{
    GraphQLFieldDefinition, GraphQLInputValueDefinition, NameValuePair, RootOperationKind,
};
use isograph_config::{CompilerConfig, CompilerConfigOptions};
use isograph_lang_parser::IsoLiteralExtractionResult;
use isograph_lang_types::{
    IsoLiteralsSource, SelectionType, ServerEntityId, ServerObjectEntityId, TypeAnnotation,
    VariableDefinition,
};
use isograph_schema::{
    validate_entrypoints, InsertFieldsError, NetworkProtocol, ProcessObjectTypeDefinitionOutcome,
    ProcessTypeSystemDocumentOutcome, RootOperationName, Schema,
    SchemaServerObjectSelectableVariant, ServerObjectSelectable, ServerScalarSelectable,
    TypeRefinementMaps, UnprocessedItem,
};
use pico::{Database, SourceId};

use crate::{
    add_selection_sets::add_selection_sets_to_client_selectables,
    batch_compile::BatchCompileError,
    isograph_literals::{parse_iso_literal_in_source, process_iso_literals},
    refetch_fields::add_refetch_fields_to_objects,
    source_files::SourceFiles,
};

pub fn create_unvalidated_schema<TNetworkProtocol: NetworkProtocol>(
    db: &Database,
    source_files: &SourceFiles,
    config: &CompilerConfig,
) -> Result<(Schema<TNetworkProtocol>, ContainsIsoStats), Box<dyn Error>> {
    let ProcessTypeSystemDocumentOutcome {
        scalars,
        objects,
        // TODO don't return these; instead, creating asConcreteType fields is a responsibility
        // of the NetworkProtocol
        //
        // (I think this means we should not transfer client fields from abstract
        // types to concrete types! That's probably broken anyway.)
        unvalidated_subtype_to_supertype_map,
        unvalidated_supertype_to_subtype_map,
    } = TNetworkProtocol::parse_and_process_type_system_documents(
        db,
        source_files.schema,
        &source_files.schema_extensions,
    )?;

    let mut unvalidated_isograph_schema = Schema::<TNetworkProtocol>::new();
    for (server_scalar_entity, name_location) in scalars {
        unvalidated_isograph_schema
            .server_entity_data
            .insert_server_scalar_entity(server_scalar_entity, name_location)?;
    }

    let mut field_queue = HashMap::new();
    for (
        ProcessObjectTypeDefinitionOutcome {
            encountered_root_kind,
            directives,
            server_object_entity,
            fields_to_insert,
        },
        name_location,
    ) in objects
    {
        let new_object_id = unvalidated_isograph_schema
            .server_entity_data
            .insert_server_object_entity(server_object_entity, name_location)?;
        field_queue.insert(new_object_id, fields_to_insert);

        unvalidated_isograph_schema
            .server_entity_data
            .server_object_entity_available_selectables
            .entry(new_object_id)
            .or_default()
            .2
            .extend(directives);

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
            // TODO handle Subscription
            _ => {}
        }
    }

    process_field_queue(
        &mut unvalidated_isograph_schema,
        field_queue,
        &config.options,
    )?;

    let type_refinement_maps = get_type_refinement_map(
        &mut unvalidated_isograph_schema,
        unvalidated_supertype_to_subtype_map,
        unvalidated_subtype_to_supertype_map,
    )?;

    let contains_iso = parse_iso_literals(
        db,
        &source_files.iso_literals,
        config.current_working_directory,
    )?;
    let contains_iso_stats = contains_iso.stats();

    // Step one: we can create client selectables. However, we must create all
    // client selectables before being able to create their selection sets, because
    // selection sets refer to client selectables. We hold onto these selection sets
    // (both reader selection sets and refetch selection sets) in the unprocess_items
    // vec, then process it later.
    let mut unprocessed_items = vec![];

    let (unprocessed_client_types, unprocessed_entrypoints) =
        process_iso_literals(&mut unvalidated_isograph_schema, contains_iso)?;
    unprocessed_items.extend(unprocessed_client_types);

    unprocessed_items.extend(process_exposed_fields(&mut unvalidated_isograph_schema)?);

    unvalidated_isograph_schema.transfer_supertype_client_selectables_to_subtypes(
        &type_refinement_maps.supertype_to_subtype_map,
    )?;
    unvalidated_isograph_schema.add_link_fields()?;
    unvalidated_isograph_schema.add_object_selectable_to_subtype_on_supertypes(
        &type_refinement_maps.subtype_to_supertype_map,
    )?;

    unprocessed_items.extend(add_refetch_fields_to_objects(
        &mut unvalidated_isograph_schema,
    )?);

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
    current_working_directory: CurrentWorkingDirectory,
) -> Result<ContainsIso, BatchCompileError> {
    let mut contains_iso = ContainsIso::default();
    let mut iso_literal_parse_errors = vec![];
    for (relative_path, iso_literals_source_id) in iso_literals_sources.iter() {
        match parse_iso_literal_in_source(db, *iso_literals_source_id, current_working_directory)
            .to_owned()
        {
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

/// Here, we are processing exposeAs fields. Note that we only process these
/// directives on root objects (Query, Mutation, Subscription) and we should
/// validate that no other types have exposeAs directives.
fn process_exposed_fields<TNetworkProtocol: NetworkProtocol>(
    schema: &mut Schema<TNetworkProtocol>,
) -> Result<Vec<UnprocessedItem>, BatchCompileError> {
    let fetchable_types: Vec<_> = schema.fetchable_types.keys().copied().collect();
    let mut unprocessed_items = vec![];
    for fetchable_object_entity_id in fetchable_types.into_iter() {
        let unprocessed_client_field_item =
            schema.add_exposed_fields_to_parent_object_types(fetchable_object_entity_id)?;
        unprocessed_items.extend(
            unprocessed_client_field_item
                .into_iter()
                .map(UnprocessedItem::Scalar),
        );
    }
    Ok(unprocessed_items)
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
    field_queue: HashMap<ServerObjectEntityId, Vec<WithLocation<GraphQLFieldDefinition>>>,
    options: &CompilerConfigOptions,
) -> Result<(), WithLocation<InsertFieldsError>> {
    for (parent_object_entity_id, field_definitions_to_insert) in field_queue {
        for field_definition in field_definitions_to_insert.into_iter() {
            let parent_object_entity = schema
                .server_entity_data
                .server_object_entity(parent_object_entity_id);

            let target_entity_type_name = field_definition.item.type_.inner();

            let selection_type = schema
                .server_entity_data
                .defined_entities
                .get(target_entity_type_name)
                .ok_or_else(|| {
                    WithLocation::new(
                        InsertFieldsError::FieldTypenameDoesNotExist {
                            target_entity_type_name: *target_entity_type_name,
                        },
                        field_definition.item.name.location,
                    )
                })?;

            let arguments = field_definition
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
                        field_definition.item.name.item.into(),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            let description = field_definition.item.description.map(|d| d.item);

            match selection_type {
                SelectionType::Scalar(scalar_entity_id) => {
                    schema
                        .insert_server_scalar_selectable(
                            ServerScalarSelectable {
                                description,
                                name: field_definition.item.name.map(|x| x.unchecked_conversion()),
                                target_scalar_entity: TypeAnnotation::from_graphql_type_annotation(
                                    field_definition.item.type_.clone(),
                                )
                                .map(&mut |_| *scalar_entity_id),
                                parent_type_id: parent_object_entity_id,
                                arguments,
                                phantom_data: std::marker::PhantomData,
                            },
                            options,
                            field_definition.item.type_.inner_non_null_named_type(),
                        )
                        .map_err(|e| WithLocation::new(e, field_definition.location))?;
                }
                SelectionType::Object(object_entity_id) => {
                    schema
                        .insert_server_object_selectable(ServerObjectSelectable {
                            description,
                            name: field_definition.item.name.map(|x| x.unchecked_conversion()),
                            target_object_entity: TypeAnnotation::from_graphql_type_annotation(
                                field_definition.item.type_.clone(),
                            )
                            .map(&mut |_| *object_entity_id),
                            parent_type_id: parent_object_entity_id,
                            arguments,
                            phantom_data: std::marker::PhantomData,
                            object_selectable_variant:
                                SchemaServerObjectSelectableVariant::LinkedField,
                        })
                        .map_err(|e| WithLocation::new(e, field_definition.location))?;
                }
            }
        }
    }

    Ok(())
}

pub fn graphql_input_value_definition_to_variable_definition(
    defined_types: &HashMap<UnvalidatedTypeName, ServerEntityId>,
    input_value_definition: WithLocation<GraphQLInputValueDefinition>,
    parent_type_name: IsographObjectTypeName,
    field_name: SelectableName,
) -> Result<WithLocation<VariableDefinition<ServerEntityId>>, WithLocation<InsertFieldsError>> {
    let default_value = input_value_definition
        .item
        .default_value
        .map(|graphql_constant_value| {
            Ok::<_, WithLocation<InsertFieldsError>>(WithLocation::new(
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
                        InsertFieldsError::FieldArgumentTypeDoesNotExist {
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
    graphql_constant_value: graphql_lang_types::GraphQLConstantValue,
) -> isograph_lang_types::ConstantValue {
    match graphql_constant_value {
        graphql_lang_types::GraphQLConstantValue::Int(i) => {
            isograph_lang_types::ConstantValue::Integer(i)
        }
        graphql_lang_types::GraphQLConstantValue::Boolean(b) => {
            isograph_lang_types::ConstantValue::Boolean(b)
        }
        graphql_lang_types::GraphQLConstantValue::String(s) => {
            isograph_lang_types::ConstantValue::String(s)
        }
        graphql_lang_types::GraphQLConstantValue::Float(f) => {
            isograph_lang_types::ConstantValue::Float(f)
        }
        graphql_lang_types::GraphQLConstantValue::Null => isograph_lang_types::ConstantValue::Null,
        graphql_lang_types::GraphQLConstantValue::Enum(e) => {
            isograph_lang_types::ConstantValue::Enum(e)
        }
        graphql_lang_types::GraphQLConstantValue::List(l) => {
            let converted_list = l
                .into_iter()
                .map(|x| {
                    WithLocation::new(
                        convert_graphql_constant_value_to_isograph_constant_value(x.item),
                        x.location,
                    )
                })
                .collect::<Vec<_>>();
            isograph_lang_types::ConstantValue::List(converted_list)
        }
        graphql_lang_types::GraphQLConstantValue::Object(o) => {
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
            isograph_lang_types::ConstantValue::Object(converted_object)
        }
    }
}

// TODO This is currently a completely useless function, serving only to surface
// some validation errors. It might be necessary once we handle __asNode etc.
// style fields.
fn get_type_refinement_map<TNetworkProtocol: NetworkProtocol>(
    schema: &mut Schema<TNetworkProtocol>,
    unvalidated_supertype_to_subtype_map: UnvalidatedTypeRefinementMap,
    unvalidated_subtype_to_supertype_map: UnvalidatedTypeRefinementMap,
) -> Result<TypeRefinementMaps, WithLocation<InsertFieldsError>> {
    let supertype_to_subtype_map =
        validate_type_refinement_map(schema, unvalidated_supertype_to_subtype_map)?;
    let subtype_to_supertype_map =
        validate_type_refinement_map(schema, unvalidated_subtype_to_supertype_map)?;

    Ok(TypeRefinementMaps {
        subtype_to_supertype_map,
        supertype_to_subtype_map,
    })
}

fn validate_type_refinement_map<TNetworkProtocol: NetworkProtocol>(
    schema: &mut Schema<TNetworkProtocol>,
    unvalidated_type_refinement_map: UnvalidatedTypeRefinementMap,
) -> Result<ValidatedTypeRefinementMap, WithLocation<InsertFieldsError>> {
    let supertype_to_subtype_map = unvalidated_type_refinement_map
        .into_iter()
        .map(|(key_type_name, values_type_names)| {
            let key_id = lookup_object_in_schema(schema, key_type_name)?;

            let value_type_ids = values_type_names
                .into_iter()
                .map(|value_type_name| lookup_object_in_schema(schema, value_type_name))
                .collect::<Result<Vec<_>, _>>()?;

            Ok((key_id, value_type_ids))
        })
        .collect::<Result<HashMap<_, _>, WithLocation<InsertFieldsError>>>()?;
    Ok(supertype_to_subtype_map)
}

fn lookup_object_in_schema<TNetworkProtocol: NetworkProtocol>(
    schema: &mut Schema<TNetworkProtocol>,
    unvalidated_type_name: UnvalidatedTypeName,
) -> Result<ServerObjectEntityId, WithLocation<InsertFieldsError>> {
    let result = (*schema
        .server_entity_data
        .defined_entities
        .get(&unvalidated_type_name)
        .ok_or_else(|| {
            WithLocation::new(
                InsertFieldsError::FieldTypenameDoesNotExist {
                    target_entity_type_name: unvalidated_type_name,
                },
                // TODO don't do this
                Location::Generated,
            )
        })?)
    .as_object_result()
    .map_err(|_| {
        WithLocation::new(
            InsertFieldsError::GenericObjectIsScalar {
                type_name: unvalidated_type_name,
            },
            // TODO don't do this
            Location::Generated,
        )
    })?;

    Ok(*result)
}

type UnvalidatedTypeRefinementMap = HashMap<UnvalidatedTypeName, Vec<UnvalidatedTypeName>>;
// When constructing the final map, we can replace object type names with ids.
pub type ValidatedTypeRefinementMap = HashMap<ServerObjectEntityId, Vec<ServerObjectEntityId>>;
