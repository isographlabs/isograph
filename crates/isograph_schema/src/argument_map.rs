use std::collections::HashMap;

use common_lang_types::{
    IsographObjectTypeName, Location, SelectableFieldName, StringLiteralValue, VariableName,
    WithLocation,
};
use graphql_lang_types::GraphQLTypeAnnotation;
use intern::{string_key::Intern, Lookup};
use isograph_lang_types::{SelectableServerFieldId, ServerFieldId};

use crate::{
    FieldDefinitionLocation, FieldMapItem, ProcessTypeDefinitionError, ProcessTypeDefinitionResult,
    ProcessedFieldMapItem, UnvalidatedSchema, UnvalidatedVariableDefinition,
};

pub(crate) struct ArgumentMap {
    arguments: Vec<WithLocation<PotentiallyModifiedArgument>>,
}

impl ArgumentMap {
    pub(crate) fn new(arguments: Vec<WithLocation<UnvalidatedVariableDefinition>>) -> Self {
        Self {
            arguments: arguments
                .into_iter()
                .map(|with_location| with_location.map(PotentiallyModifiedArgument::Unmodified))
                .collect(),
        }
    }

    pub(crate) fn remove_field_map_item(
        &mut self,
        field_map_item: FieldMapItem,
        primary_type_name: IsographObjectTypeName,
        mutation_object_name: IsographObjectTypeName,
        mutation_field_name: SelectableFieldName,
        schema: &mut UnvalidatedSchema,
    ) -> ProcessTypeDefinitionResult<ProcessedFieldMapItem> {
        let split_to_arg = field_map_item.split_to_arg();
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
                name.lookup() == split_to_arg.to_argument_name.lookup()
            })
            .ok_or_else(|| {
                WithLocation::new(
                    ProcessTypeDefinitionError::PrimaryDirectiveArgumentDoesNotExistOnField {
                        primary_type_name,
                        mutation_object_name,
                        mutation_field_name,
                        field_name: split_to_arg.to_argument_name.lookup().to_string(),
                    },
                    Location::generated(),
                )
            })?;

        // TODO avoid matching twice?
        let location = argument.location;

        let processed_field_map_item = match &mut argument.item {
            PotentiallyModifiedArgument::Unmodified(unmodified_argument) => {
                match split_to_arg.to_field_names.split_first() {
                    None => {
                        match schema
                            .server_field_data
                            .defined_types
                            .get(&unmodified_argument.type_.inner().lookup().intern().into())
                        {
                            Some(defined_type) => match defined_type {
                                SelectableServerFieldId::Object(_) => return Err(WithLocation::new(
                                    ProcessTypeDefinitionError::PrimaryDirectiveCannotRemapObject {
                                        primary_type_name,
                                        field_name: split_to_arg
                                            .to_argument_name
                                            .lookup()
                                            .to_string(),
                                    },
                                    Location::generated(),
                                )),
                                SelectableServerFieldId::Scalar(_) => {}
                            },
                            None => panic!(
                                "Type is not found. This is indicative \
                                of a bug in Isograph, and will be solved by validating first."
                            ),
                        }

                        self.arguments.swap_remove(index_of_argument);

                        ProcessedFieldMapItem(field_map_item.clone())
                    }
                    Some((first, rest)) => {
                        let mut arg =
                            ModifiedArgument::from_unmodified(unmodified_argument, schema);

                        arg.remove_to_field(schema, *first, rest, primary_type_name)?;

                        *argument =
                            WithLocation::new(PotentiallyModifiedArgument::Modified(arg), location);
                        // processed_field_map_item
                        // TODO wat
                        ProcessedFieldMapItem(field_map_item.clone())
                    }
                }
            }
            PotentiallyModifiedArgument::Modified(modified) => {
                let to_field_names = &split_to_arg.to_field_names;
                match to_field_names.split_first() {
                    None => {
                        // TODO encode this in the type system.
                        // A modified argument will always have an object type, and cannot be remapped
                        // at the object level.
                        return Err(WithLocation::new(
                            ProcessTypeDefinitionError::PrimaryDirectiveCannotRemapObject {
                                primary_type_name,
                                field_name: split_to_arg.to_argument_name.to_string(),
                            },
                            Location::generated(),
                        ));
                    }
                    Some((first, rest)) => {
                        modified.remove_to_field(schema, *first, rest, primary_type_name)?;
                        // TODO WAT
                        ProcessedFieldMapItem(field_map_item.clone())
                    }
                }
            }
        };

        Ok(processed_field_map_item)
    }
}

enum PotentiallyModifiedArgument {
    Unmodified(UnvalidatedVariableDefinition),
    Modified(ModifiedArgument),
}

/// An object which has fields that are unmodified, deleted,
/// or modified (indicating that a new object should be created
/// for them to point to.) Scalar fields cannot be modified,
/// only deleted.
#[derive(Debug)]
pub(crate) struct ModifiedObject {
    field_map: HashMap<SelectableFieldName, PotentiallyModifiedField>,
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
        _first: StringLiteralValue,
        _rest: &[StringLiteralValue],
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
    name: WithLocation<VariableName>,
    object: GraphQLTypeAnnotation<ModifiedObject>,
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
        unmodified: &UnvalidatedVariableDefinition,
        schema: &UnvalidatedSchema,
    ) -> Self {
        // TODO I think we have validated that the item exists already.
        // But we should double check that, and return an error if necessary
        let object = unmodified.type_.clone().map(|input_type_name| {
            let defined_type_id = *schema
                .server_field_data
                .defined_types
                .get(&input_type_name)
                .expect(
                    "Expected type to be defined by now. This is indicative of a bug in Isograph.",
                );
            match defined_type_id {
                SelectableServerFieldId::Object(object_id) => {
                    let object = schema.server_field_data.object(object_id);

                    ModifiedObject {
                        field_map: object
                            .encountered_fields
                            .iter()
                            .flat_map(|(name, field_id)| match field_id {
                                FieldDefinitionLocation::Server(s) => {
                                    Some((*name, PotentiallyModifiedField::Unmodified(*s)))
                                }
                                FieldDefinitionLocation::Client(_) => None,
                            })
                            .collect(),
                    }
                }
                SelectableServerFieldId::Scalar(_scalar_id) => {
                    // TODO don't be lazy, return an error
                    panic!("Cannot modify a scalar")
                }
            }
        });

        // TODO We can probably avoid cloning here
        Self {
            name: unmodified.name,
            object,
        }
    }

    pub fn remove_to_field(
        &mut self,
        schema: &UnvalidatedSchema,
        first: StringLiteralValue,
        rest: &[StringLiteralValue],
        primary_type_name: IsographObjectTypeName,
    ) -> ProcessTypeDefinitionResult<()> {
        let argument_object = self.object.inner_mut();

        let key = first.lookup().intern().into();
        match argument_object
            .field_map
            // TODO make this a no-op
            .get_mut(&key)
        {
            Some(field) => {
                match rest.split_first() {
                    Some((first, rest)) => {
                        match field.remove_to_field(*first, rest)? {
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
                                let field_object = schema.server_field(*field_id);
                                let field_object_type = field_object.associated_data.inner();

                                // N.B. this should be done via a validation pass.
                                match schema
                                    .server_field_data
                                    .defined_types
                                    .get(field_object_type)
                                {
                                    Some(type_) => match type_ {
                                        SelectableServerFieldId::Object(_) => {
                                            // Otherwise, formatting breaks :(
                                            use ProcessTypeDefinitionError::PrimaryDirectiveCannotRemapObject;
                                            return Err(WithLocation::new(
                                                PrimaryDirectiveCannotRemapObject {
                                                    primary_type_name,
                                                    field_name: key.to_string(),
                                                },
                                                Location::generated(),
                                            ));
                                        }
                                        SelectableServerFieldId::Scalar(_scalar_id) => {
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
                                    Location::generated(),
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
                        field_name: first,
                    },
                    Location::generated(),
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
