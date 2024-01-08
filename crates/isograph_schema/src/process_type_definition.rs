use std::collections::{hash_map::Entry, HashMap};

use common_lang_types::{
    DescriptionValue, InputValueName, IsographObjectTypeName, Location, ScalarTypeName,
    SelectableFieldName, Span, StringLiteralValue, TextSource, UnvalidatedTypeName, WithLocation,
    WithSpan,
};
use graphql_lang_types::{
    ConstantValue, GraphQLDirective, GraphQLInputValueDefinition, GraphQLOutputFieldDefinition,
    GraphQLScalarTypeDefinition, GraphQLTypeSystemDefinition, GraphQLTypeSystemDocument,
    GraphQLTypeSystemExtension, GraphQLTypeSystemExtensionDocument,
    GraphQLTypeSystemExtensionOrDefinition, NamedTypeAnnotation, NonNullTypeAnnotation,
    TypeAnnotation,
};
use intern::{string_key::Intern, Lookup};
use isograph_lang_types::{
    DefinedTypeId, ObjectId, ResolverFieldId, ScalarFieldSelection, Selection, ServerFieldId,
    ServerFieldSelection, ServerIdFieldId,
};
use lazy_static::lazy_static;
use thiserror::Error;

use crate::{
    ConfigOptions, DefinedField, IsographObjectTypeDefinition, ResolverActionKind,
    ResolverTypeAndField, ResolverVariant, Schema, SchemaObject, SchemaResolver, SchemaScalar,
    SchemaServerField, UnvalidatedObjectFieldInfo, UnvalidatedSchema, UnvalidatedSchemaField,
    UnvalidatedSchemaResolver, ValidRefinement, ID_GRAPHQL_TYPE, STRING_JAVASCRIPT_TYPE,
};

lazy_static! {
    static ref QUERY_TYPE: IsographObjectTypeName = "Query".intern().into();
    static ref MUTATION_TYPE: IsographObjectTypeName = "Mutation".intern().into();
}

type TypeRefinementMap = HashMap<IsographObjectTypeName, Vec<WithLocation<ObjectId>>>;

pub struct ProcessGraphQLDocumentOutcome {
    pub mutation_id: Option<ObjectId>,
}

struct ProcessObjectTypeDefinitionOutcome {
    object_id: ObjectId,
    mutation_object_id: Option<ObjectId>,
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
        let mut valid_type_refinement_map = HashMap::new();

        let mut mutation_type_id = None;
        for type_system_definition in type_system_document.0 {
            match type_system_definition {
                GraphQLTypeSystemDefinition::ObjectTypeDefinition(object_type_definition) => {
                    let object_type_definition = object_type_definition.into();

                    let outcome = self.process_object_type_definition(
                        object_type_definition,
                        &mut valid_type_refinement_map,
                        true,
                        options,
                    )?;
                    if let Some(mutation_id) = outcome.mutation_object_id {
                        mutation_type_id = Some(mutation_id);
                    };
                }
                GraphQLTypeSystemDefinition::ScalarTypeDefinition(scalar_type_definition) => {
                    self.process_scalar_definition(scalar_type_definition)?;
                    // N.B. we assume that Mutation will be an object, not a scalar
                }
                GraphQLTypeSystemDefinition::InterfaceTypeDefinition(interface_type_definition) => {
                    self.process_object_type_definition(
                        interface_type_definition.into(),
                        &mut valid_type_refinement_map,
                        true,
                        options,
                    )?;
                    // N.B. we assume that Mutation will be an object, not an interface
                }
                GraphQLTypeSystemDefinition::InputObjectTypeDefinition(
                    input_object_type_definition,
                ) => {
                    self.process_object_type_definition(
                        input_object_type_definition.into(),
                        &mut valid_type_refinement_map,
                        false,
                        options,
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
                        &mut valid_type_refinement_map,
                        true,
                        options,
                    )?;
                }
            }
        }

        for (supertype_name, subtypes) in valid_type_refinement_map {
            // TODO perhaps encode this in the type system
            let first_item = subtypes
                .first()
                .expect("subtypes should not be empty. This indicates a bug in Isograph");

            // supertype, if it exists, can be refined to each subtype
            let supertype_id = self
                .schema_data
                .defined_types
                .get(&supertype_name.into())
                .ok_or(WithLocation::new(
                    ProcessTypeDefinitionError::IsographObjectTypeNameNotDefined {
                        type_name: supertype_name,
                    },
                    // TODO look up the first_item, get the matching implementing object, and
                    // use that instead.
                    first_item.location,
                ))?;

            match supertype_id {
                DefinedTypeId::Scalar(scalar_id) => {
                    let scalar = self.schema_data.scalar(*scalar_id);
                    let first_implementing_object = self.schema_data.object(first_item.item);

                    return Err(WithLocation::new(
                        ProcessTypeDefinitionError::IsographObjectTypeNameIsScalar {
                            type_name: supertype_name,
                            implementing_object: first_implementing_object.name,
                        },
                        scalar.name.location,
                    ));
                }
                DefinedTypeId::Object(object_id) => {
                    let supertype = self.schema_data.object_mut(*object_id);
                    // TODO validate that supertype was defined as an interface, perhaps by
                    // including references to the original definition (i.e. as a type parameter)
                    // and having the schema be able to validate this. (i.e. this should be
                    // a way to execute GraphQL-specific code in isograph-land without actually
                    // putting the code here.)

                    for subtype_id in subtypes {
                        supertype.valid_refinements.push(ValidRefinement {
                            target: subtype_id.item,
                        });
                    }
                }
            }
        }

        Ok(ProcessGraphQLDocumentOutcome {
            mutation_id: mutation_type_id,
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
            match extension_or_definition {
                GraphQLTypeSystemExtensionOrDefinition::Definition(definition) => {
                    definitions.push(definition);
                }
                GraphQLTypeSystemExtensionOrDefinition::Extension(extension) => {
                    extensions.push(extension)
                }
            }
        }

        // N.B. we should probably restructure this...?
        // Like, we could discover the mutation type right now!
        self.process_graphql_type_system_document(GraphQLTypeSystemDocument(definitions), options)?;

        for extension in extensions.into_iter() {
            // TODO collect errors into vec
            self.process_graphql_type_system_extension(extension)?;
        }

        Ok(ProcessGraphQLDocumentOutcome {
            // TODO process schema and return mutation id
            mutation_id: None,
        })
    }

    fn process_graphql_type_system_extension(
        &mut self,
        extension: GraphQLTypeSystemExtension,
    ) -> ProcessTypeDefinitionResult<()> {
        match extension {
            GraphQLTypeSystemExtension::ObjectTypeExtension(object_extension) => {
                let name = object_extension.name.item;

                let id = self
                    .schema_data
                    .defined_types
                    .get(&name.into())
                    .ok_or_else(|| panic!("TODO why does this id not exist?"))?;

                match *id {
                    DefinedTypeId::Object(object_id) => {
                        let schema_object = self.schema_data.object_mut(object_id);

                        if !object_extension.fields.is_empty() {
                            panic!("Adding fields in schema extensions is not allowed, yet.");
                        }
                        if !object_extension.interfaces.is_empty() {
                            panic!("Adding interfaces in schema extensions is not allowed, yet.");
                        }

                        schema_object
                            .directives
                            .extend(object_extension.directives.into_iter());

                        Ok(())
                    }
                    DefinedTypeId::Scalar(_) => Err(WithLocation::new(
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

    fn process_object_type_definition(
        &mut self,
        object_type_definition: IsographObjectTypeDefinition,
        valid_type_refinement_map: &mut TypeRefinementMap,
        // TODO this smells! We should probably pass Option<ServerIdFieldId>
        may_have_id_field: bool,
        options: ConfigOptions,
    ) -> ProcessTypeDefinitionResult<ProcessObjectTypeDefinitionOutcome> {
        let &mut Schema {
            fields: ref mut schema_fields,
            ref mut schema_data,
            resolvers: ref mut schema_resolvers,
            ..
        } = self;
        let next_object_id = schema_data.objects.len().into();
        let string_type_for_typename = schema_data.scalar(self.string_type_id).name;
        let ref mut type_names = schema_data.defined_types;
        let ref mut objects = schema_data.objects;
        let mut mutation_id = None;
        match type_names.entry(object_type_definition.name.item.into()) {
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
                    server_fields,
                    mut encountered_fields,
                    id_field,
                } = get_field_objects_ids_and_names(
                    type_def_2.fields,
                    schema_fields.len(),
                    next_object_id,
                    type_def_2.name.item.into(),
                    get_typename_type(string_type_for_typename.item),
                    may_have_id_field,
                    options,
                )?;

                let object_resolvers = get_resolvers_for_schema_object(
                    &id_field,
                    &mut encountered_fields,
                    schema_resolvers,
                    next_object_id,
                    &object_type_definition,
                );

                objects.push(SchemaObject {
                    description: object_type_definition.description.map(|d| d.item),
                    name: object_type_definition.name.item,
                    id: next_object_id,
                    server_fields,
                    resolvers: object_resolvers,
                    encountered_fields,
                    valid_refinements: vec![],
                    id_field,
                    directives: object_type_definition.directives,
                });

                // ----- HACK -----
                // This should mutate a default query object; only if no schema declaration is ultimately
                // encountered should we use the default query object.
                //
                // Also, this is a GraphQL concept, but it's leaking into Isograph land :/ (is it?)
                if object_type_definition.name.item == *QUERY_TYPE {
                    self.query_type_id = Some(next_object_id);
                }
                // --- END HACK ---

                // ----- HACK -----
                // It's unclear to me that this is the best way to add magic mutation fields.
                if object_type_definition.name.item == *MUTATION_TYPE {
                    mutation_id = Some(next_object_id)
                }
                // --- END HACK ---

                schema_fields.extend(unvalidated_schema_fields);
                vacant.insert(DefinedTypeId::Object(next_object_id));
            }
        }

        for interface in object_type_definition.interfaces {
            // type_definition implements interface
            let definitions = valid_type_refinement_map
                .entry(interface.item.into())
                .or_default();
            definitions.push(WithLocation::new(
                next_object_id,
                object_type_definition.name.location,
            ));
        }

        Ok(ProcessObjectTypeDefinitionOutcome {
            object_id: next_object_id,
            mutation_object_id: mutation_id,
        })
    }

    // TODO this should accept an IsographScalarTypeDefinition
    fn process_scalar_definition(
        &mut self,
        scalar_type_definition: GraphQLScalarTypeDefinition,
    ) -> ProcessTypeDefinitionResult<()> {
        let &mut Schema {
            ref mut schema_data,
            ..
        } = self;
        let next_scalar_id = schema_data.scalars.len().into();
        let ref mut type_names = schema_data.defined_types;
        let ref mut scalars = schema_data.scalars;
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

                vacant.insert(DefinedTypeId::Scalar(next_scalar_id));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct FieldMapItem {
    // TODO eventually, we want to support . syntax here, too
    pub from: StringLiteralValue,
    /// Everything that is before the first . in the to field
    pub to_argument_name: WithSpan<StringLiteralValue>,
    /// Everything after the first ., split on .
    pub to_field_names: Vec<WithSpan<StringLiteralValue>>,
}

#[derive(Debug)]
struct ModifiedArgument {
    description: Option<WithSpan<DescriptionValue>>,
    name: WithLocation<InputValueName>,
    object: TypeAnnotation<ModifiedObject>,
    default_value: Option<WithLocation<ConstantValue>>,
    directives: Vec<GraphQLDirective<ConstantValue>>,
}

/// An object which has fields that are unmodified, deleted,
/// or modified (indicating that a new object should be created
/// for them to point to.) Scalar fields cannot be modified,
/// only deleted.
#[derive(Debug)]
struct ModifiedObject {
    object_id: ObjectId,
    field_map: HashMap<SelectableFieldName, PotentiallyModifiedField>,
}

impl ModifiedObject {
    fn create_and_get_name(
        self,
        schema: &mut UnvalidatedSchema,
        options: ConfigOptions,
    ) -> IsographObjectTypeName {
        let original_object = schema.schema_data.object(self.object_id);

        let fields = original_object
            .server_fields
            .iter()
            .flat_map(|field_id| {
                let field = schema.field(*field_id);

                // HACK alert
                if field.name.item == "__typename".intern().into() {
                    return None;
                }

                let potentially_modified_field = self.field_map.get(&field.name.item)?;

                let new_field = match potentially_modified_field {
                    PotentiallyModifiedField::Unmodified(_) => WithLocation::new(
                        GraphQLOutputFieldDefinition {
                            description: field.description.map(|description| {
                                WithSpan::new(description, Span::todo_generated())
                            }),
                            name: field.name,
                            type_: field.associated_data.clone(),
                            arguments: field.arguments.clone(),
                            directives: vec![],
                        },
                        Location::generated(),
                    ),
                    PotentiallyModifiedField::Modified(_) => todo!("modified"),
                };

                Some(new_field)
            })
            .collect();

        let item = IsographObjectTypeDefinition {
            // TODO it looks like we throw away span info for descriptions, which makes sense?
            description: original_object
                .description
                .clone()
                .map(|description| WithSpan::new(description, Span::todo_generated())),
            // TODO confirm that names don't have to be unique
            name: WithLocation::new(
                format!("{}__generated", original_object.name.lookup())
                    .intern()
                    .into(),
                Location::generated(),
            ),
            // Very unclear what to do here
            interfaces: vec![],
            directives: vec![],
            fields,
        };

        let ProcessObjectTypeDefinitionOutcome { object_id, .. } = schema
            .process_object_type_definition(item, &mut HashMap::new(), true, options)
            // This is not (yet) true. If you reference a non-existent type in
            // a @exposeField directive, the compiler panics here. The solution is to
            // process these directives after the definitions have been validated,
            // but before resolvers hvae been validated.
            .expect(
                "Expected object creation to work. This is \
                indicative of a bug in Isograph.",
            );

        schema.schema_data.object(object_id).name
    }
}

#[derive(Debug)]
enum PotentiallyModifiedField {
    Unmodified(ServerFieldId),
    // This is exercised in the case of 3+ segments, e.g. input.foo.id.
    // For now, we support only up to two segments.
    #[allow(dead_code)]
    Modified(ModifiedField),
}

#[allow(dead_code)]
enum IsEmpty {
    IsEmpty,
    NotEmpty,
}

impl PotentiallyModifiedField {
    fn remove_to_field(
        &mut self,
        _first: &WithSpan<StringLiteralValue>,
        _rest: &[WithSpan<StringLiteralValue>],
    ) -> ProcessTypeDefinitionResult<IsEmpty> {
        unimplemented!("Removing to fields from PotentiallyModifiedField")
    }
}

/// A modified field's type must be an object. A scalar field that
/// is modified is just removed.
#[derive(Debug)]
struct ModifiedField {
    #[allow(dead_code)]
    modified_object: ModifiedObject,
}

impl ModifiedArgument {
    /// N.B. this kinda-sorta creates a ModifiedArgument in an invalid state,
    /// in that if we didn't immediately call remove_to_field, we would have
    /// a modified argument with a modified object containing no modified fields.
    ///
    /// Thus, we would unnecessarily create a new object that is identical to
    /// an existing object.
    ///
    /// This panics if unmodified's type is a scalar.
    pub fn from_unmodified(
        unmodified: &GraphQLInputValueDefinition,
        schema: &UnvalidatedSchema,
    ) -> Self {
        // TODO I think we have validated that the item exists already.
        // But we should double check that, and return an error if necessary
        let object = unmodified.type_.clone().map(|x| {
            let defined_type_id = *schema.schema_data.defined_types.get(&x.into()).expect(
                "Expected type to be defined by now. This is indicative of a bug in Isograph.",
            );
            match defined_type_id {
                DefinedTypeId::Object(object_id) => {
                    let object = schema.schema_data.object(object_id);

                    ModifiedObject {
                        object_id,
                        field_map: object
                            .server_fields
                            .iter()
                            .map(|server_field_id| {
                                (
                                    schema.field(*server_field_id).name.item,
                                    PotentiallyModifiedField::Unmodified(*server_field_id),
                                )
                            })
                            .collect(),
                    }
                }
                DefinedTypeId::Scalar(_scalar_id) => {
                    // TODO don't be lazy, return an error
                    panic!("Cannot modify a scalar")
                }
            }
        });

        // TODO We can probably avoid cloning here
        Self {
            name: unmodified.name,
            description: unmodified.description,
            default_value: unmodified.default_value.clone(),
            directives: unmodified.directives.clone(),
            object,
        }
    }

    pub fn remove_to_field(
        &mut self,
        schema: &UnvalidatedSchema,
        first: &WithSpan<StringLiteralValue>,
        rest: &[WithSpan<StringLiteralValue>],
        primary_type_name: IsographObjectTypeName,
        text_source: TextSource,
    ) -> ProcessTypeDefinitionResult<()> {
        let argument_object = self.object.inner_mut();

        let key = first.item.lookup().intern().into();
        let _ = match argument_object
            .field_map
            // TODO make this a no-op
            .get_mut(&key)
        {
            Some(field) => {
                match rest.split_first() {
                    Some((first, rest)) => {
                        match field.remove_to_field(first, rest)? {
                            IsEmpty::IsEmpty => {
                                // The field's object has no remaining fields (except for __typename),
                                // so we remove the item from the parent.
                                argument_object.field_map.remove(&key).expect(
                                    "Expected to be able to remove item. \
                                    This is indicative of a bug in Isograph",
                                );
                            }
                            IsEmpty::NotEmpty => {}
                        }
                    }
                    None => {
                        // We ran out of path segments, so we remove this item.
                        // It must have a scalar type.

                        match field {
                            PotentiallyModifiedField::Unmodified(field_id) => {
                                let field_object = schema.field(*field_id);
                                let field_object_type = field_object.associated_data.inner();

                                // N.B. this should be done via a validation pass.
                                match schema.schema_data.defined_types.get(field_object_type) {
                                    Some(type_) => match type_ {
                                        DefinedTypeId::Object(_) => {
                                            // Otherwise, formatting breaks :(
                                            use ProcessTypeDefinitionError::PrimaryDirectiveCannotRemapObject;
                                            return Err(WithLocation::new(
                                                PrimaryDirectiveCannotRemapObject {
                                                    primary_type_name,
                                                    field_name: key.to_string(),
                                                },
                                                Location::new(text_source, first.span),
                                            ));
                                        }
                                        DefinedTypeId::Scalar(_scalar_id) => {
                                            // Cool! We found a scalar, we can remove it.
                                            argument_object.field_map.remove(&key).expect(
                                                "Expected to be able to remove item. \
                                                This is indicative of a bug in Isograph.",
                                            );
                                        }
                                    },

                                    None => panic!("Encountered a non-existent type."),
                                }
                            }
                            PotentiallyModifiedField::Modified(_) => {
                                // A field can only be modified if it has an object type
                                return Err(WithLocation::new(
                                    ProcessTypeDefinitionError::PrimaryDirectiveCannotRemapObject {
                                        primary_type_name,
                                        field_name: key.to_string(),
                                    },
                                    Location::new(text_source, first.span),
                                ));
                            }
                        }
                    }
                }
            }
            None => {
                return Err(WithLocation::new(
                    ProcessTypeDefinitionError::PrimaryDirectiveFieldNotFound {
                        primary_type_name,
                        field_name: first.item,
                    },
                    Location::new(text_source, first.span),
                ))
            }
        };
        Ok(())
    }
}

enum PotentiallyModifiedArgument {
    Unmodified(GraphQLInputValueDefinition),
    Modified(ModifiedArgument),
}

pub(crate) struct ArgumentMap {
    arguments: Vec<WithLocation<PotentiallyModifiedArgument>>,
}

impl ArgumentMap {
    pub(crate) fn new(arguments: Vec<WithLocation<GraphQLInputValueDefinition>>) -> Self {
        Self {
            arguments: arguments
                .into_iter()
                .map(|with_location| {
                    with_location.map(|argument| PotentiallyModifiedArgument::Unmodified(argument))
                })
                .collect(),
        }
    }

    pub(crate) fn remove_field_map_item(
        &mut self,
        field_map_item: FieldMapItem,
        primary_type_name: IsographObjectTypeName,
        mutation_object_name: IsographObjectTypeName,
        mutation_field_name: SelectableFieldName,
        text_source: TextSource,
        schema: &mut UnvalidatedSchema,
    ) -> ProcessTypeDefinitionResult<ProcessedFieldMapItem> {
        let (index_of_argument, argument) = self
            .arguments
            .iter_mut()
            .enumerate()
            .find(|(_, argument)| {
                let name = match &argument.item {
                    PotentiallyModifiedArgument::Unmodified(argument) => argument.name.item,
                    PotentiallyModifiedArgument::Modified(modified_argument) => {
                        modified_argument.name.item
                    }
                };
                name.lookup() == field_map_item.to_argument_name.item.lookup()
            })
            .ok_or_else(|| {
                WithLocation::new(
                    ProcessTypeDefinitionError::PrimaryDirectiveArgumentDoesNotExistOnField {
                        primary_type_name,
                        mutation_object_name,
                        mutation_field_name,
                        field_name: field_map_item.to_argument_name.item.lookup().to_string(),
                    },
                    Location::new(text_source, field_map_item.to_argument_name.span),
                )
            })?;

        // TODO avoid matching twice?
        let location = argument.location;

        let to_field_names = &field_map_item.to_field_names;
        let to_argument_name = &field_map_item.to_argument_name;

        let processed_field_map_item = match &mut argument.item {
            PotentiallyModifiedArgument::Unmodified(unmodified_argument) => {
                match to_field_names.split_first() {
                    None => {
                        match schema
                            .schema_data
                            .defined_types
                            .get(&unmodified_argument.type_.inner().lookup().intern().into())
                        {
                            Some(defined_type) => match defined_type {
                                DefinedTypeId::Object(_) => return Err(WithLocation::new(
                                    ProcessTypeDefinitionError::PrimaryDirectiveCannotRemapObject {
                                        primary_type_name,
                                        field_name: to_argument_name.item.lookup().to_string(),
                                    },
                                    Location::new(text_source, to_argument_name.span),
                                )),
                                DefinedTypeId::Scalar(_) => {}
                            },
                            None => panic!(
                                "Type is not found. This is indicative \
                                of a bug in Isograph, and will be solved by validating first."
                            ),
                        }

                        self.arguments.swap_remove(index_of_argument);
                        let processed_field_map_item: ProcessedFieldMapItem =
                            field_map_item.clone();
                        processed_field_map_item
                    }
                    Some((first, rest)) => {
                        let mut arg =
                            ModifiedArgument::from_unmodified(unmodified_argument, schema);

                        let _processed_field_map_item = arg.remove_to_field(
                            schema,
                            first,
                            rest,
                            primary_type_name,
                            text_source,
                        )?;

                        *argument =
                            WithLocation::new(PotentiallyModifiedArgument::Modified(arg), location);
                        // processed_field_map_item
                        // TODO wat
                        field_map_item.clone()
                    }
                }
            }
            PotentiallyModifiedArgument::Modified(modified) => {
                let to_field_names = &field_map_item.to_field_names;
                match to_field_names.split_first() {
                    None => {
                        // TODO encode this in the type system.
                        // A modified argument will always have an object type, and cannot be remapped
                        // at the object level.
                        return Err(WithLocation::new(
                            ProcessTypeDefinitionError::PrimaryDirectiveCannotRemapObject {
                                primary_type_name,
                                field_name: to_argument_name.item.to_string(),
                            },
                            Location::new(text_source, field_map_item.to_argument_name.span),
                        ));
                    }
                    Some((first, rest)) => {
                        modified.remove_to_field(
                            schema,
                            first,
                            rest,
                            primary_type_name,
                            text_source,
                        )?;
                        // TODO WAT
                        field_map_item.clone()
                    }
                }
            }
        };

        Ok(processed_field_map_item)
    }

    pub(crate) fn into_arguments(
        self,
        schema: &mut UnvalidatedSchema,
        options: ConfigOptions,
    ) -> Vec<WithLocation<GraphQLInputValueDefinition>> {
        self.arguments
            .into_iter()
            .map(|with_location| {
                with_location.map(|potentially_modified_argument| {
                    match potentially_modified_argument {
                        PotentiallyModifiedArgument::Unmodified(unmodified) => unmodified,
                        PotentiallyModifiedArgument::Modified(modified) => {
                            let ModifiedArgument {
                                description,
                                name,
                                object,
                                default_value,
                                directives,
                            } = modified;

                            GraphQLInputValueDefinition {
                                description,
                                name,
                                type_: object.map(|modified_object| {
                                    modified_object
                                        .create_and_get_name(schema, options)
                                        .lookup()
                                        .intern()
                                        .into()
                                }),
                                default_value,
                                directives,
                            }
                        }
                    }
                })
            })
            .collect()
    }
}

/// Returns the resolvers for a schema object that we know up-front (before processing
/// iso literals.) This is either a refetch field (if the object is refetchable), or
/// nothing.
fn get_resolvers_for_schema_object(
    id_field_id: &Option<ServerIdFieldId>,
    encountered_fields: &mut HashMap<SelectableFieldName, UnvalidatedObjectFieldInfo>,
    schema_resolvers: &mut Vec<UnvalidatedSchemaResolver>,
    parent_object_id: ObjectId,
    type_definition: &IsographObjectTypeDefinition,
) -> Vec<ResolverFieldId> {
    if let Some(_id_field_id) = id_field_id {
        let next_resolver_id = schema_resolvers.len().into();
        let id_field_selection = WithSpan::new(
            Selection::ServerField(ServerFieldSelection::ScalarField(ScalarFieldSelection {
                name: WithLocation::new("id".intern().into(), Location::generated()),
                reader_alias: None,
                normalization_alias: None,
                associated_data: (),
                unwraps: vec![],
                arguments: vec![],
            })),
            Span::todo_generated(),
        );
        schema_resolvers.push(SchemaResolver {
            description: Some("A refetch field for this object.".intern().into()),
            name: "__refetch".intern().into(),
            id: next_resolver_id,
            selection_set_and_unwraps: Some((vec![id_field_selection], vec![])),
            variant: ResolverVariant::RefetchField,
            variable_definitions: vec![],
            type_and_field: ResolverTypeAndField {
                type_name: type_definition.name.item,
                field_name: "__refetch".intern().into(),
            },
            parent_object_id,
            // N.B. __refetch fields are non-fetchable, but they do execute queries which
            // have fetchable artifacts (i.e. normalization ASTs).
            action_kind: ResolverActionKind::RefetchField,
        });
        encountered_fields.insert(
            "__refetch".intern().into(),
            DefinedField::ResolverField(next_resolver_id),
        );
        vec![next_resolver_id]
    } else {
        vec![]
    }
}

fn get_typename_type(
    string_type_for_typename: ScalarTypeName,
) -> TypeAnnotation<UnvalidatedTypeName> {
    TypeAnnotation::NonNull(Box::new(NonNullTypeAnnotation::Named(NamedTypeAnnotation(
        WithSpan::new(
            string_type_for_typename.into(),
            // TODO we probably need a generated or built-in span type
            Span::todo_generated(),
        ),
    ))))
}

struct FieldObjectIdsEtc {
    unvalidated_schema_fields: Vec<UnvalidatedSchemaField>,
    server_fields: Vec<ServerFieldId>,
    // TODO this should be HashMap<_, WithLocation<_>> or something
    encountered_fields: HashMap<SelectableFieldName, UnvalidatedObjectFieldInfo>,
    // TODO this should not be a ServerFieldId, but a special type
    id_field: Option<ServerIdFieldId>,
}

/// Given a vector of fields from the schema AST all belonging to the same object/interface,
/// return a vector of unvalidated fields and a set of field names.
fn get_field_objects_ids_and_names(
    new_fields: Vec<WithLocation<GraphQLOutputFieldDefinition>>,
    next_field_id: usize,
    parent_type_id: ObjectId,
    parent_type_name: IsographObjectTypeName,
    typename_type: TypeAnnotation<UnvalidatedTypeName>,
    // TODO this is hacky
    may_have_field_id: bool,
    options: ConfigOptions,
) -> ProcessTypeDefinitionResult<FieldObjectIdsEtc> {
    let new_field_count = new_fields.len();
    let mut encountered_fields = HashMap::with_capacity(new_field_count);
    let mut unvalidated_fields = Vec::with_capacity(new_field_count);
    let mut field_ids = Vec::with_capacity(new_field_count + 1); // +1 for the typename
    let mut id_field = None;
    let id_name = "id".intern().into();
    for (current_field_index, field) in new_fields.into_iter().enumerate() {
        // TODO use entry
        match encountered_fields.insert(
            field.item.name.item,
            DefinedField::ServerField(field.item.type_.clone()),
        ) {
            None => {
                let current_field_id = next_field_id + current_field_index;

                // TODO check for @strong directive instead!
                if may_have_field_id && field.item.name.item == id_name {
                    set_and_validate_id_field(
                        &mut id_field,
                        current_field_id,
                        &field,
                        parent_type_name,
                        options,
                    )?;
                }

                unvalidated_fields.push(SchemaServerField {
                    description: field.item.description.map(|d| d.item),
                    name: field.item.name,
                    id: current_field_id.into(),
                    associated_data: field.item.type_,
                    parent_type_id,
                    arguments: field.item.arguments,
                });
                field_ids.push(current_field_id.into());
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
    let typename_field_id = (next_field_id + field_ids.len()).into();
    let typename_name = WithLocation::new("__typename".intern().into(), Location::generated());
    field_ids.push(typename_field_id);
    unvalidated_fields.push(SchemaServerField {
        description: None,
        name: typename_name,
        id: typename_field_id,
        associated_data: typename_type.clone(),
        parent_type_id,
        arguments: vec![],
    });

    if encountered_fields
        .insert(typename_name.item, DefinedField::ServerField(typename_type))
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
        server_fields: field_ids,
        encountered_fields,
        id_field,
    })
}

/// If we have encountered an id field, we can:
/// - validate that the id field is properly defined, i.e. has type ID!
/// - set the id field
fn set_and_validate_id_field(
    id_field: &mut Option<ServerIdFieldId>,
    current_field_id: usize,
    field: &WithLocation<GraphQLOutputFieldDefinition>,
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
            if (*type_).0.item.lookup() != ID_GRAPHQL_TYPE.lookup() {
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
        None => Err(WithLocation::new(
            ProcessTypeDefinitionError::IdFieldMustBeNonNullIdType {
                strong_field_name: "id",
                parent_type: parent_type_name,
            },
            // TODO this might show the wrong span?
            field.location,
        )),
    }
}

// TODO this should be a different type.
pub(crate) type ProcessedFieldMapItem = FieldMapItem;

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

    #[error("Due to a mutation, Isograph attempted to create a field named \"{field_name}\" on type \"{parent_type}\", but a field with that name already exists.")]
    MutationFieldIsDuplicate {
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
    IsographObjectTypeNameIsScalar {
        type_name: IsographObjectTypeName,
        implementing_object: IsographObjectTypeName,
    },

    #[error(
        "You cannot manually defined the \"__typename\" field, which is defined in \"{parent_type}\"."
    )]
    TypenameCannotBeDefined { parent_type: IsographObjectTypeName },

    #[error("The {strong_field_name} field on \"{parent_type}\" must have type \"ID!\".")]
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
        unused_field_map_items.iter().map(|x| format!("'{}'", x.to_argument_name)).collect::<Vec<_>>().join(", ")
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
}
