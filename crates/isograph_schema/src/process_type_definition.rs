use std::collections::{hash_map::Entry, BTreeMap, HashMap};

use crate::{
    EncounteredRootTypes, FieldDefinitionLocation, IsographObjectTypeDefinition,
    ProcessedRootTypes, RootOperationName, RootTypes, Schema, SchemaObject, SchemaScalar,
    SchemaServerField, UnvalidatedObjectFieldInfo, UnvalidatedSchema, UnvalidatedSchemaSchemaField,
    ID_GRAPHQL_TYPE, STRING_JAVASCRIPT_TYPE,
};
use common_lang_types::{
    GraphQLObjectTypeName, GraphQLScalarTypeName, IsographObjectTypeName, Location,
    SelectableFieldName, Span, StringLiteralValue, UnvalidatedTypeName, VariableName, WithLocation,
    WithSpan,
};
use graphql_lang_types::{
    GraphQLFieldDefinition, GraphQLInputValueDefinition, GraphQLNamedTypeAnnotation,
    GraphQLNonNullTypeAnnotation, GraphQLScalarTypeDefinition, GraphQLTypeAnnotation,
    GraphQLTypeSystemDefinition, GraphQLTypeSystemDocument, GraphQLTypeSystemExtension,
    GraphQLTypeSystemExtensionDocument, GraphQLTypeSystemExtensionOrDefinition, NameValuePair,
    RootOperationKind,
};
use intern::{string_key::Intern, Lookup};
use isograph_config::ConfigOptions;
use isograph_lang_types::{
    SelectableServerFieldId, ServerObjectId, ServerStrongIdFieldId, VariableDefinition,
};
use lazy_static::lazy_static;
use serde::Deserialize;
use thiserror::Error;

lazy_static! {
    static ref QUERY_TYPE: IsographObjectTypeName = "Query".intern().into();
    static ref MUTATION_TYPE: IsographObjectTypeName = "Mutation".intern().into();
}

// When parsing, we have the subtype's ObjectId, but only the Supertype's name
type UnvalidatedSupertypeToSubtypeMap = HashMap<IsographObjectTypeName, Vec<ServerObjectId>>;
type UnvalidatedSubtypeToSupertypeMap =
    HashMap<ServerObjectId, Vec<WithLocation<IsographObjectTypeName>>>;
// When constructing the final map, we have both!
pub type TypeRefinementMap = HashMap<ServerObjectId, Vec<ServerObjectId>>;

#[allow(unused)]
#[derive(Debug)]
pub struct TypeRefinementMaps {
    pub subtype_to_supertype_map: TypeRefinementMap,
    pub supertype_to_subtype_map: TypeRefinementMap,
}

pub struct ProcessGraphQLDocumentOutcome {
    pub type_refinement_maps: TypeRefinementMaps,
    pub root_types: EncounteredRootTypes,
}

pub struct ProcessObjectTypeDefinitionOutcome {
    pub object_id: ServerObjectId,
    pub encountered_root_kind: Option<RootOperationKind>,
}

impl UnvalidatedSchema {
    pub fn process_graphql_type_system_document(
        &mut self,
        type_system_document: GraphQLTypeSystemDocument,
        options: ConfigOptions,
    ) -> ProcessTypeDefinitionResult<ProcessGraphQLDocumentOutcome> {
        // In the schema, interfaces, unions and objects are the same type of object (SchemaType),
        // with e.g. interfaces "simply" being objects that can be refined to other
        // concrete objects.
        //
        // Processing type system documents is done in two passes:
        // - First, create types for interfaces, objects, scalars, etc.
        // - Then, validate that all implemented interfaces exist, and add refinements
        //   to the found interface.
        let mut supertype_to_subtype_map = HashMap::new();
        let mut subtype_to_supertype_map = HashMap::new();

        let mut encountered_root_types = RootTypes {
            query: None,
            mutation: None,
            subscription: None,
        };
        let mut processed_root_types = None;

        for with_location in type_system_document.0 {
            let WithLocation {
                location,
                item: type_system_definition,
            } = with_location;
            match type_system_definition {
                GraphQLTypeSystemDefinition::ObjectTypeDefinition(object_type_definition) => {
                    let concrete_type = Some(object_type_definition.name.item.into());
                    let object_type_definition = object_type_definition.into();
                    let outcome = self.process_object_type_definition(
                        object_type_definition,
                        &mut supertype_to_subtype_map,
                        &mut subtype_to_supertype_map,
                        true,
                        options,
                        concrete_type,
                    )?;
                    if let Some(encountered_root_kind) = outcome.encountered_root_kind {
                        encountered_root_types
                            .set_root_type(encountered_root_kind, outcome.object_id);
                    }
                }
                GraphQLTypeSystemDefinition::ScalarTypeDefinition(scalar_type_definition) => {
                    self.process_scalar_definition(scalar_type_definition)?;
                    // N.B. we assume that Mutation will be an object, not a scalar
                }
                GraphQLTypeSystemDefinition::InterfaceTypeDefinition(interface_type_definition) => {
                    self.process_object_type_definition(
                        interface_type_definition.into(),
                        &mut supertype_to_subtype_map,
                        &mut subtype_to_supertype_map,
                        true,
                        options,
                        None,
                    )?;
                    // N.B. we assume that Mutation will be an object, not an interface
                }
                GraphQLTypeSystemDefinition::InputObjectTypeDefinition(
                    input_object_type_definition,
                ) => {
                    let concrete_type = Some(input_object_type_definition.name.item.into());
                    self.process_object_type_definition(
                        input_object_type_definition.into(),
                        &mut supertype_to_subtype_map,
                        &mut subtype_to_supertype_map,
                        false,
                        options,
                        // Shouldn't really matter what we pass here
                        concrete_type,
                    )?;
                }
                GraphQLTypeSystemDefinition::DirectiveDefinition(_) => {
                    // For now, Isograph ignores directive definitions,
                    // but it might choose to allow-list them.
                }
                GraphQLTypeSystemDefinition::EnumDefinition(enum_definition) => {
                    // TODO Do not do this
                    self.process_scalar_definition(GraphQLScalarTypeDefinition {
                        description: enum_definition.description,
                        name: enum_definition.name.map(|x| x.lookup().intern().into()),
                        directives: enum_definition.directives,
                    })?;
                }
                GraphQLTypeSystemDefinition::UnionTypeDefinition(union_definition) => {
                    // TODO do something reasonable here, once we add support for type refinements.
                    self.process_object_type_definition(
                        IsographObjectTypeDefinition {
                            description: union_definition.description,
                            name: union_definition.name.map(|x| x.into()),
                            interfaces: vec![],
                            directives: union_definition.directives,
                            fields: vec![],
                        },
                        &mut supertype_to_subtype_map,
                        &mut subtype_to_supertype_map,
                        true,
                        options,
                        None,
                    )?;
                }
                GraphQLTypeSystemDefinition::SchemaDefinition(schema_definition) => {
                    if processed_root_types.is_some() {
                        return Err(WithLocation::new(
                            ProcessTypeDefinitionError::DuplicateSchemaDefinition,
                            location,
                        ));
                    }
                    processed_root_types = Some(RootTypes {
                        query: schema_definition.query,
                        mutation: schema_definition.mutation,
                        subscription: schema_definition.subscription,
                    })
                }
            }
        }

        let type_refinement_map =
            self.get_type_refinement_map(supertype_to_subtype_map, subtype_to_supertype_map)?;

        let root_types = self.process_root_types(processed_root_types, encountered_root_types)?;

        if let Some(query_type_id) = root_types.query {
            self.fetchable_types
                .insert(query_type_id, RootOperationName("query".to_string()));
        }
        if let Some(mutation_type_id) = root_types.mutation {
            self.fetchable_types
                .insert(mutation_type_id, RootOperationName("mutation".to_string()));
        }
        // TODO add support for subscriptions

        Ok(ProcessGraphQLDocumentOutcome {
            root_types,
            type_refinement_maps: type_refinement_map,
        })
    }

    // TODO This is currently a completely useless function, serving only to surface
    // some validation errors. It might be necessary once we handle __asNode etc.
    // style fields.
    fn get_type_refinement_map(
        &mut self,
        unvalidated_supertype_to_subtype_map: UnvalidatedSupertypeToSubtypeMap,
        unvalidated_subtype_to_supertype_map: UnvalidatedSubtypeToSupertypeMap,
    ) -> ProcessTypeDefinitionResult<TypeRefinementMaps> {
        let mut subtype_to_supertype_map = HashMap::new();
        for (subtype_id, supertype_names) in unvalidated_subtype_to_supertype_map {
            match subtype_to_supertype_map.entry(subtype_id) {
                Entry::Occupied(_) => {
                    panic!(
                        "Expected type_refinement_map to be empty at {} \
                        ({}). This is indicative of a bug in Isograph.",
                        self.server_field_data.object(subtype_id).name,
                        subtype_id
                    );
                }
                Entry::Vacant(vacant) => {
                    let entries = supertype_names
                        .into_iter()
                        .map(|supertype_name| {
                            let supertype_id = self
                                .server_field_data
                                .defined_types
                                .get(&supertype_name.item.into())
                                .ok_or(WithLocation::new(
                                    ProcessTypeDefinitionError::IsographObjectTypeNameNotDefined {
                                        type_name: supertype_name.item,
                                    },
                                    supertype_name.location,
                                ))?;
                            match supertype_id {
                                SelectableServerFieldId::Scalar(_) => {
                                    let subtype_name =
                                        self.server_field_data.object(subtype_id).name;

                                    Err(WithLocation::new(
                                        ProcessTypeDefinitionError::ObjectIsScalar {
                                            type_name: supertype_name.item,
                                            implementing_object: subtype_name,
                                        },
                                        supertype_name.location,
                                    ))
                                }
                                SelectableServerFieldId::Object(supertype_object_id) => {
                                    Ok(*supertype_object_id)
                                }
                            }
                        })
                        .collect::<Result<Vec<_>, _>>()?;

                    vacant.insert(entries);
                }
            }
        }

        let mut supertype_to_subtype_map = HashMap::new();
        for (supertype_name, subtypes) in unvalidated_supertype_to_subtype_map {
            let supertype_id = self
                .server_field_data
                .defined_types
                .get(&supertype_name.into())
                .expect("Expected Interface to be found. This indicates a bug in Isograph.");

            match supertype_id {
                SelectableServerFieldId::Scalar(_) => {
                    panic!(
                        "Expected an object id; this is indicative of a bug in Isograph and \
                        should have already been validated."
                    );
                }
                SelectableServerFieldId::Object(supertype_object_id) => {
                    // TODO validate that supertype was defined as an interface, perhaps by
                    // including references to the original definition (i.e. as a type parameter)
                    // and having the schema be able to validate this. (i.e. this should be
                    // a way to execute GraphQL-specific code in isograph-land without actually
                    // putting the code here.)

                    let value = subtypes.into_iter().collect();
                    match supertype_to_subtype_map.entry(*supertype_object_id) {
                        Entry::Occupied(_) => {
                            panic!("Encountered duplicate supertype. This is indicative of a bug in Isograph.")
                        }
                        Entry::Vacant(vacant) => {
                            vacant.insert(value);
                        }
                    }
                }
            };
        }

        Ok(TypeRefinementMaps {
            subtype_to_supertype_map,
            supertype_to_subtype_map,
        })
    }

    pub fn process_graphql_type_extension_document(
        &mut self,
        extension_document: GraphQLTypeSystemExtensionDocument,
        options: ConfigOptions,
    ) -> ProcessTypeDefinitionResult<ProcessGraphQLDocumentOutcome> {
        let mut definitions = Vec::with_capacity(extension_document.0.len());
        let mut extensions = Vec::with_capacity(extension_document.0.len());

        for extension_or_definition in extension_document.0 {
            let WithLocation { location, item } = extension_or_definition;
            match item {
                GraphQLTypeSystemExtensionOrDefinition::Definition(definition) => {
                    definitions.push(WithLocation::new(definition, location));
                }
                GraphQLTypeSystemExtensionOrDefinition::Extension(extension) => {
                    extensions.push(WithLocation::new(extension, location))
                }
            }
        }

        // N.B. we should probably restructure this...?
        // Like, we could discover the mutation type right now!
        let outcome = self.process_graphql_type_system_document(
            GraphQLTypeSystemDocument(definitions),
            options,
        )?;

        for extension in extensions.into_iter() {
            // TODO collect errors into vec
            // TODO we can encounter new interface implementations; we should account for that
            self.process_graphql_type_system_extension(extension)?;
        }

        Ok(outcome)
    }

    fn process_graphql_type_system_extension(
        &mut self,
        extension: WithLocation<GraphQLTypeSystemExtension>,
    ) -> ProcessTypeDefinitionResult<()> {
        match extension.item {
            GraphQLTypeSystemExtension::ObjectTypeExtension(object_extension) => {
                let name = object_extension.name.item;

                let id = self
                    .server_field_data
                    .defined_types
                    .get(&name.into())
                    .expect(
                    "TODO why does this id not exist. This probably indicates a bug in Isograph.",
                );

                match *id {
                    SelectableServerFieldId::Object(object_id) => {
                        let schema_object = self.server_field_data.object_mut(object_id);

                        if !object_extension.fields.is_empty() {
                            panic!("Adding fields in schema extensions is not allowed, yet.");
                        }
                        if !object_extension.interfaces.is_empty() {
                            panic!("Adding interfaces in schema extensions is not allowed, yet.");
                        }

                        schema_object.directives.extend(object_extension.directives);

                        Ok(())
                    }
                    SelectableServerFieldId::Scalar(_) => Err(WithLocation::new(
                        ProcessTypeDefinitionError::TypeExtensionMismatch {
                            type_name: name.into(),
                            is_type: "a scalar",
                            extended_as_type: "an object",
                        },
                        object_extension.name.location,
                    )),
                }
            }
        }
    }

    pub(crate) fn process_object_type_definition(
        &mut self,
        object_type_definition: IsographObjectTypeDefinition,
        supertype_to_subtype_map: &mut UnvalidatedSupertypeToSubtypeMap,
        subtype_to_supertype_map: &mut UnvalidatedSubtypeToSupertypeMap,
        // TODO this smells! We should probably pass Option<ServerIdFieldId>
        may_have_id_field: bool,
        options: ConfigOptions,
        concrete_type: Option<IsographObjectTypeName>,
    ) -> ProcessTypeDefinitionResult<ProcessObjectTypeDefinitionOutcome> {
        let &mut Schema {
            server_fields: ref mut schema_fields,
            server_field_data: ref mut schema_data,
            ..
        } = self;
        let next_object_id = schema_data.server_objects.len().into();
        let string_type_for_typename = schema_data.scalar(self.string_type_id).name;
        let type_names = &mut schema_data.defined_types;
        let objects = &mut schema_data.server_objects;
        let encountered_root_kind = match type_names.entry(object_type_definition.name.item.into())
        {
            Entry::Occupied(_) => {
                return Err(WithLocation::new(
                    ProcessTypeDefinitionError::DuplicateTypeDefinition {
                        // BUG: this could be an interface, actually
                        type_definition_type: "object",
                        type_name: object_type_definition.name.item.into(),
                    },
                    object_type_definition.name.location,
                ));
            }
            Entry::Vacant(vacant) => {
                // TODO avoid this
                let type_def_2 = object_type_definition.clone();
                let FieldObjectIdsEtc {
                    unvalidated_schema_fields,
                    encountered_fields,
                    id_field,
                } = get_field_objects_ids_and_names(
                    type_def_2.fields,
                    schema_fields.len(),
                    next_object_id,
                    type_def_2.name.item,
                    get_typename_type(string_type_for_typename.item),
                    may_have_id_field,
                    options,
                )?;

                objects.push(SchemaObject {
                    description: object_type_definition.description.map(|d| d.item),
                    name: object_type_definition.name.item,
                    id: next_object_id,
                    encountered_fields,
                    id_field,
                    directives: object_type_definition.directives,
                    concrete_type,
                });

                schema_fields.extend(unvalidated_schema_fields);
                vacant.insert(SelectableServerFieldId::Object(next_object_id));

                // TODO default types are a GraphQL-land concept, but this is Isograph-land
                if object_type_definition.name.item == *QUERY_TYPE {
                    Some(RootOperationKind::Query)
                } else if object_type_definition.name.item == *MUTATION_TYPE {
                    Some(RootOperationKind::Mutation)
                } else {
                    // TODO subscription
                    None
                }
            }
        };

        for interface in &object_type_definition.interfaces {
            // type_definition implements interface
            let definitions = supertype_to_subtype_map
                .entry(interface.item.into())
                .or_default();
            definitions.push(next_object_id);
        }
        // This check isn't necessary, but it keeps the data structure smaller
        if !object_type_definition.interfaces.is_empty() {
            match subtype_to_supertype_map.entry(next_object_id) {
                Entry::Occupied(_) => panic!(
                    "Expected object id to not have been encountered before.\
                    This is indicative of a bug in Isograph."
                ),
                Entry::Vacant(vacant) => {
                    vacant.insert(
                        object_type_definition
                            .interfaces
                            .into_iter()
                            .map(|with_location| with_location.map(|x| x.into()))
                            .collect(),
                    );
                }
            }
        }

        Ok(ProcessObjectTypeDefinitionOutcome {
            object_id: next_object_id,
            encountered_root_kind,
        })
    }

    // TODO this should accept an IsographScalarTypeDefinition
    fn process_scalar_definition(
        &mut self,
        scalar_type_definition: GraphQLScalarTypeDefinition,
    ) -> ProcessTypeDefinitionResult<()> {
        let &mut Schema {
            server_field_data: ref mut schema_data,
            ..
        } = self;
        let next_scalar_id = schema_data.server_scalars.len().into();
        let type_names = &mut schema_data.defined_types;
        let scalars = &mut schema_data.server_scalars;
        match type_names.entry(scalar_type_definition.name.item.into()) {
            Entry::Occupied(_) => {
                return Err(WithLocation::new(
                    ProcessTypeDefinitionError::DuplicateTypeDefinition {
                        type_definition_type: "scalar",
                        type_name: scalar_type_definition.name.item.into(),
                    },
                    scalar_type_definition.name.location,
                ));
            }
            Entry::Vacant(vacant) => {
                scalars.push(SchemaScalar {
                    description: scalar_type_definition.description,
                    name: scalar_type_definition.name,
                    id: next_scalar_id,
                    javascript_name: *STRING_JAVASCRIPT_TYPE,
                });

                vacant.insert(SelectableServerFieldId::Scalar(next_scalar_id));
            }
        }
        Ok(())
    }

    fn process_root_types(
        &self,
        processed_root_types: Option<ProcessedRootTypes>,
        encountered_root_types: EncounteredRootTypes,
    ) -> ProcessTypeDefinitionResult<EncounteredRootTypes> {
        match processed_root_types {
            Some(processed_root_types) => {
                let RootTypes {
                    query: query_type_name,
                    mutation: mutation_type_name,
                    subscription: subscription_type_name,
                } = processed_root_types;

                let query_id = query_type_name
                    .map(|query_type_name| self.look_up_root_type(query_type_name))
                    .transpose()?;
                let mutation_id = mutation_type_name
                    .map(|mutation_type_name| self.look_up_root_type(mutation_type_name))
                    .transpose()?;
                let subscription_id = subscription_type_name
                    .map(|subscription_type_name| self.look_up_root_type(subscription_type_name))
                    .transpose()?;

                Ok(RootTypes {
                    query: query_id,
                    mutation: mutation_id,
                    subscription: subscription_id,
                })
            }
            None => Ok(encountered_root_types),
        }
    }

    fn look_up_root_type(
        &self,
        type_name: WithLocation<GraphQLObjectTypeName>,
    ) -> ProcessTypeDefinitionResult<ServerObjectId> {
        match self
            .server_field_data
            .defined_types
            .get(&type_name.item.into())
        {
            Some(SelectableServerFieldId::Object(object_id)) => Ok(*object_id),
            Some(SelectableServerFieldId::Scalar(_)) => Err(WithLocation::new(
                ProcessTypeDefinitionError::RootTypeMustBeObject,
                type_name.location,
            )),
            None => Err(WithLocation::new(
                ProcessTypeDefinitionError::IsographObjectTypeNameNotDefined {
                    type_name: type_name.item.into(),
                },
                type_name.location,
            )),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[serde(deny_unknown_fields)]
pub struct FieldMapItem {
    // TODO eventually, we want to support . syntax here, too
    pub from: StringLiteralValue,
    pub to: StringLiteralValue,
}

pub struct SplitToArg {
    pub to_argument_name: StringLiteralValue,
    pub to_field_names: Vec<StringLiteralValue>,
}

impl FieldMapItem {
    pub fn split_to_arg(&self) -> SplitToArg {
        let mut split = self.to.lookup().split('.');
        let to_argument_name = split.next().expect(
            "Expected at least one item returned \
                by split. This is indicative of a bug in Isograph.",
        );

        SplitToArg {
            to_argument_name: to_argument_name.intern().into(),
            to_field_names: split.map(|x| x.intern().into()).collect(),
        }
    }
}

fn get_typename_type(
    string_type_for_typename: GraphQLScalarTypeName,
) -> GraphQLTypeAnnotation<UnvalidatedTypeName> {
    GraphQLTypeAnnotation::NonNull(Box::new(GraphQLNonNullTypeAnnotation::Named(
        GraphQLNamedTypeAnnotation(WithSpan::new(
            string_type_for_typename.into(),
            // TODO we probably need a generated or built-in span type
            Span::todo_generated(),
        )),
    )))
}

struct FieldObjectIdsEtc {
    unvalidated_schema_fields: Vec<UnvalidatedSchemaSchemaField>,
    // TODO this should be BTreeMap<_, WithLocation<_>> or something
    encountered_fields: BTreeMap<SelectableFieldName, UnvalidatedObjectFieldInfo>,
    // TODO this should not be a ServerFieldId, but a special type
    id_field: Option<ServerStrongIdFieldId>,
}

/// Given a vector of fields from the schema AST all belonging to the same object/interface,
/// return a vector of unvalidated fields and a set of field names.
fn get_field_objects_ids_and_names(
    new_fields: Vec<WithLocation<GraphQLFieldDefinition>>,
    next_field_id: usize,
    parent_type_id: ServerObjectId,
    parent_type_name: IsographObjectTypeName,
    typename_type: GraphQLTypeAnnotation<UnvalidatedTypeName>,
    // TODO this is hacky
    may_have_field_id: bool,
    options: ConfigOptions,
) -> ProcessTypeDefinitionResult<FieldObjectIdsEtc> {
    let new_field_count = new_fields.len();
    let mut encountered_fields = BTreeMap::new();
    let mut unvalidated_fields = Vec::with_capacity(new_field_count);
    let mut server_field_ids = Vec::with_capacity(new_field_count + 1); // +1 for the typename
    let mut id_field = None;
    let id_name = "id".intern().into();
    for (current_field_index, field) in new_fields.into_iter().enumerate() {
        let next_server_field_id_usize = next_field_id + current_field_index;
        let next_server_field_id = next_server_field_id_usize.into();

        match encountered_fields.insert(
            field.item.name.item,
            FieldDefinitionLocation::Server(next_server_field_id),
        ) {
            None => {
                // TODO check for @strong directive instead!
                if may_have_field_id && field.item.name.item == id_name {
                    set_and_validate_id_field(
                        &mut id_field,
                        next_server_field_id_usize,
                        &field,
                        parent_type_name,
                        options,
                    )?;
                }

                unvalidated_fields.push(SchemaServerField {
                    description: field.item.description.map(|d| d.item),
                    name: field.item.name,
                    id: next_server_field_id,
                    associated_data: field.item.type_,
                    parent_type_id,
                    arguments: field
                        .item
                        .arguments
                        .into_iter()
                        .map(graphql_input_value_definition_to_variable_definition)
                        .collect::<Result<Vec<_>, _>>()?,
                    is_discriminator: false,
                });
                server_field_ids.push(next_server_field_id);
            }
            Some(_) => {
                return Err(WithLocation::new(
                    ProcessTypeDefinitionError::DuplicateField {
                        field_name: field.item.name.item,
                        parent_type: parent_type_name,
                    },
                    field.item.name.location,
                ));
            }
        }
    }

    // ------- HACK -------
    // Magic __typename field
    // TODO: find a way to do this that is less tied to GraphQL
    // TODO: the only way to determine that a field is a magic __typename field is
    // to check the name! That's a bit unfortunate. We should model these differently,
    // perhaps fields should contain an enum (IdField, TypenameField, ActualField)
    let typename_field_id = (next_field_id + server_field_ids.len()).into();
    let typename_name = WithLocation::new("__typename".intern().into(), Location::generated());
    server_field_ids.push(typename_field_id);
    unvalidated_fields.push(SchemaServerField {
        description: None,
        name: typename_name,
        id: typename_field_id,
        associated_data: typename_type.clone(),
        parent_type_id,
        arguments: vec![],
        is_discriminator: true,
    });

    if encountered_fields
        .insert(
            typename_name.item,
            FieldDefinitionLocation::Server(typename_field_id),
        )
        .is_some()
    {
        return Err(WithLocation::new(
            ProcessTypeDefinitionError::TypenameCannotBeDefined {
                parent_type: parent_type_name,
            },
            // This is blatantly incorrect, we should have the location
            // of the previously defined typename
            Location::generated(),
        ));
    }
    // ----- END HACK -----

    Ok(FieldObjectIdsEtc {
        unvalidated_schema_fields: unvalidated_fields,
        encountered_fields,
        id_field,
    })
}

pub fn graphql_input_value_definition_to_variable_definition(
    input_value_definition: WithLocation<GraphQLInputValueDefinition>,
) -> ProcessTypeDefinitionResult<WithLocation<VariableDefinition<UnvalidatedTypeName>>> {
    let default_value = input_value_definition
        .item
        .default_value
        .map(|graphql_constant_value| {
            Ok::<_, WithLocation<ProcessTypeDefinitionError>>(WithLocation::new(
                convert_graphql_constant_value_to_isograph_constant_value(
                    graphql_constant_value.item,
                ),
                graphql_constant_value.location,
            ))
        })
        .transpose()?;
    Ok(WithLocation::new(
        VariableDefinition {
            name: input_value_definition.item.name.map(VariableName::from),
            type_: input_value_definition
                .item
                .type_
                .map(UnvalidatedTypeName::from),
            default_value,
        },
        input_value_definition.location,
    ))
}

/// If we have encountered an id field, we can:
/// - validate that the id field is properly defined, i.e. has type ID!
/// - set the id field
fn set_and_validate_id_field(
    id_field: &mut Option<ServerStrongIdFieldId>,
    current_field_id: usize,
    field: &WithLocation<GraphQLFieldDefinition>,
    parent_type_name: IsographObjectTypeName,
    options: ConfigOptions,
) -> ProcessTypeDefinitionResult<()> {
    // N.B. id_field is guaranteed to be None; otherwise field_names_to_type_name would
    // have contained this field name already.
    debug_assert!(id_field.is_none(), "id field should not be defined twice");

    // We should change the type here! It should not be ID! It should be a
    // type specific to the concrete type, e.g. UserID.
    *id_field = Some(current_field_id.into());

    match field.item.type_.inner_non_null_named_type() {
        Some(type_) => {
            if type_.0.item.lookup() != ID_GRAPHQL_TYPE.lookup() {
                options.on_invalid_id_type.on_failure(|| {
                    WithLocation::new(
                        ProcessTypeDefinitionError::IdFieldMustBeNonNullIdType {
                            strong_field_name: "id",
                            parent_type: parent_type_name,
                        },
                        // TODO this shows the wrong span?
                        field.location,
                    )
                })?;
            }
            Ok(())
        }
        None => {
            options.on_invalid_id_type.on_failure(|| {
                WithLocation::new(
                    ProcessTypeDefinitionError::IdFieldMustBeNonNullIdType {
                        strong_field_name: "id",
                        parent_type: parent_type_name,
                    },
                    // TODO this shows the wrong span?
                    field.location,
                )
            })?;
            Ok(())
        }
    }
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

// TODO this should be a different type.
pub(crate) struct ProcessedFieldMapItem(pub FieldMapItem);

pub(crate) type ProcessTypeDefinitionResult<T> =
    Result<T, WithLocation<ProcessTypeDefinitionError>>;

/// Errors that make semantic sense when referring to creating a GraphQL schema in-memory representation
#[derive(Error, Debug)]
pub enum ProcessTypeDefinitionError {
    // TODO include info about where the type was previously defined
    // TODO the type_definition_name refers to the second object being defined, which isn't
    // all that helpful
    #[error("Duplicate type definition ({type_definition_type}) named \"{type_name}\"")]
    DuplicateTypeDefinition {
        type_definition_type: &'static str,
        type_name: UnvalidatedTypeName,
    },

    // TODO include info about where the field was previously defined
    #[error("Duplicate field named \"{field_name}\" on type \"{parent_type}\"")]
    DuplicateField {
        field_name: SelectableFieldName,
        parent_type: IsographObjectTypeName,
    },

    #[error(
        "The Isograph compiler attempted to create a field named \
    \"{field_name}\" on type \"{parent_type}\", but a field with that name already exists."
    )]
    FieldExistsOnSubtype {
        field_name: SelectableFieldName,
        parent_type: IsographObjectTypeName,
    },

    // TODO
    // This is held in a span pointing to one place the non-existent type was referenced.
    // We should perhaps include info about all the places it was referenced.
    //
    // When type Foo implements Bar and Bar is not defined:
    #[error("Type \"{type_name}\" is never defined.")]
    IsographObjectTypeNameNotDefined { type_name: IsographObjectTypeName },

    // When type Foo implements Bar and Bar is scalar
    #[error("\"{implementing_object}\" attempted to implement \"{type_name}\". However, \"{type_name}\" is a scalar, but only other object types can be implemented.")]
    ObjectIsScalar {
        type_name: IsographObjectTypeName,
        implementing_object: IsographObjectTypeName,
    },

    #[error(
        "You cannot manually defined the \"__typename\" field, which is defined in \"{parent_type}\"."
    )]
    TypenameCannotBeDefined { parent_type: IsographObjectTypeName },

    #[error(
        "The {strong_field_name} field on \"{parent_type}\" must have type \"ID!\".\n\
    This error can be suppressed using the \"on_invalid_id_type\" config parameter."
    )]
    IdFieldMustBeNonNullIdType {
        parent_type: IsographObjectTypeName,
        strong_field_name: &'static str,
    },

    #[error("The @exposeField directive should have three arguments")]
    InvalidPrimaryDirectiveArgumentCount,

    #[error("The @exposeField directive requires a path argument")]
    MissingPathArg,

    #[error("The @exposeField directive requires a field_map argument")]
    MissingFieldMapArg,

    #[error("The @exposeField directive path argument value should be a string")]
    PathValueShouldBeString,

    #[error("Invalid field_map in @exposeField directive")]
    InvalidFieldMap,

    #[error("Invalid field in @exposeField directive")]
    InvalidField,

    #[error("Invalid mutation field")]
    InvalidMutationField,

    // TODO include which fields were unused
    #[error("Not all fields specified as 'to' fields in the @exposeField directive field_map were found \
        on the mutation field. Unused fields: {}",
        unused_field_map_items.iter().map(|x| format!("'{}'", x.split_to_arg().to_argument_name)).collect::<Vec<_>>().join(", ")
    )]
    NotAllToFieldsUsed {
        unused_field_map_items: Vec<FieldMapItem>,
    },

    #[error("In a @exposeField directive's field_map, the to field cannot be just a dot.")]
    FieldMapToCannotJustBeADot,

    #[error(
        "Error when processing @exposeField directive on type `{primary_type_name}`. \
        The field `{mutation_object_name}.{mutation_field_name}` does not have argument `{field_name}`, \
        or it was previously processed by another field_map item."
    )]
    PrimaryDirectiveArgumentDoesNotExistOnField {
        primary_type_name: IsographObjectTypeName,
        mutation_object_name: IsographObjectTypeName,
        mutation_field_name: SelectableFieldName,
        field_name: String,
    },

    #[error(
        "Error when processing @exposeField directive on type `{primary_type_name}`. \
        The field `{field_name}` is an object, and cannot be remapped. Remap scalars only."
    )]
    PrimaryDirectiveCannotRemapObject {
        primary_type_name: IsographObjectTypeName,
        field_name: String,
    },

    #[error(
        "Error when processing @exposeField directive on type `{primary_type_name}`. \
        The field `{field_name}` is not found."
    )]
    PrimaryDirectiveFieldNotFound {
        primary_type_name: IsographObjectTypeName,
        field_name: StringLiteralValue,
    },

    #[error(
        "The type `{type_name}` is {is_type}, but it is being extended as {extended_as_type}."
    )]
    TypeExtensionMismatch {
        type_name: UnvalidatedTypeName,
        is_type: &'static str,
        extended_as_type: &'static str,
    },

    #[error("Duplicate schema definition")]
    DuplicateSchemaDefinition,

    #[error("Root types must be objects. This type is a scalar.")]
    RootTypeMustBeObject,

    #[error("Failed to deserialize {0}")]
    FailedToDeserialize(String),
}
