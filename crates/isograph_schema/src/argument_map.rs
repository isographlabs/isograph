use std::collections::HashMap;

use common_lang_types::{
    DescriptionValue, InputValueName, IsographObjectTypeName, Location, SelectableFieldName, Span,
    StringLiteralValue, TextSource, WithLocation, WithSpan,
};
use graphql_lang_types::{
    ConstantValue, GraphQLDirective, GraphQLFieldDefinition, GraphQLInputValueDefinition,
    TypeAnnotation,
};
use intern::{string_key::Intern, Lookup};
use isograph_lang_types::{DefinedTypeId, ObjectId, ServerFieldId};

use crate::{
    FieldMapItem, IsographObjectTypeDefinition, ProcessObjectTypeDefinitionOutcome,
    ProcessTypeDefinitionError, ProcessTypeDefinitionResult, ProcessedFieldMapItem,
    UnvalidatedSchema,
};
use isograph_config::ConfigOptions;

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
                        let processed_field_map_item =
                            ProcessedFieldMapItem(field_map_item.clone());
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
                        ProcessedFieldMapItem(field_map_item.clone())
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
                        ProcessedFieldMapItem(field_map_item.clone())
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

enum PotentiallyModifiedArgument {
    Unmodified(GraphQLInputValueDefinition),
    Modified(ModifiedArgument),
}

/// An object which has fields that are unmodified, deleted,
/// or modified (indicating that a new object should be created
/// for them to point to.) Scalar fields cannot be modified,
/// only deleted.
#[derive(Debug)]
pub(crate) struct ModifiedObject {
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
                        GraphQLFieldDefinition {
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
            .process_object_type_definition(
                item,
                // Obviously, this is a smell
                &mut HashMap::new(),
                &mut HashMap::new(),
                true,
                options,
            )
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
pub(crate) enum PotentiallyModifiedField {
    Unmodified(ServerFieldId),
    // This is exercised in the case of 3+ segments, e.g. input.foo.id.
    // For now, we support only up to two segments.
    #[allow(dead_code)]
    Modified(ModifiedField),
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
pub(crate) struct ModifiedField {
    #[allow(dead_code)]
    modified_object: ModifiedObject,
}

#[derive(Debug)]
struct ModifiedArgument {
    description: Option<WithSpan<DescriptionValue>>,
    name: WithLocation<InputValueName>,
    object: TypeAnnotation<ModifiedObject>,
    default_value: Option<WithLocation<ConstantValue>>,
    directives: Vec<GraphQLDirective<ConstantValue>>,
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

#[allow(dead_code)]
enum IsEmpty {
    IsEmpty,
    NotEmpty,
}
