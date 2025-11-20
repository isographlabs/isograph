use common_lang_types::{
    SelectableName, ServerObjectEntityName, ServerSelectableName, StringLiteralValue, VariableName,
    WithLocation,
};
use graphql_lang_types::GraphQLTypeAnnotation;
use intern::Lookup;
use isograph_lang_types::{SelectionType, VariableDefinition};
use prelude::Postfix;
use std::collections::HashMap;

use crate::{
    IsographDatabase, NetworkProtocol, ServerEntityName, ServerSelectableId,
    ValidatedVariableDefinition, server_selectables_vec_for_entity,
};

use super::create_additional_fields_error::{
    CreateAdditionalFieldsError, FieldMapItem, ProcessTypeDefinitionResult, ProcessedFieldMapItem,
};

#[derive(Debug)]
pub(crate) struct ArgumentMap {
    arguments: Vec<WithLocation<PotentiallyModifiedArgument>>,
}

impl ArgumentMap {
    pub(crate) fn new(arguments: Vec<WithLocation<VariableDefinition<ServerEntityName>>>) -> Self {
        Self {
            arguments: arguments
                .into_iter()
                .map(|with_location| with_location.map(PotentiallyModifiedArgument::Unmodified))
                .collect(),
        }
    }
}

pub(crate) fn remove_field_map_item<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    argument_map: &mut ArgumentMap,
    field_map_item: FieldMapItem,
    primary_object_entity_name: ServerObjectEntityName,
    mutation_object_entity_name: ServerObjectEntityName,
    mutation_selectable_name: SelectableName,
) -> ProcessTypeDefinitionResult<ProcessedFieldMapItem, TNetworkProtocol> {
    let split_to_arg = field_map_item.split_to_arg();
    let (index_of_argument, argument) = argument_map
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
            name == split_to_arg.to_argument_name
        })
        .ok_or_else(|| {
            CreateAdditionalFieldsError::PrimaryDirectiveArgumentDoesNotExistOnField {
                primary_object_entity_name,
                mutation_object_entity_name,
                mutation_selectable_name,
                field_name: split_to_arg.to_argument_name,
            }
        })?;

    // TODO avoid matching twice?
    let location = argument.location;

    match &mut argument.item {
        PotentiallyModifiedArgument::Unmodified(unmodified_argument) => {
            match split_to_arg.to_field_names.split_first() {
                None => {
                    if unmodified_argument.type_.inner().as_object().is_some() {
                        return CreateAdditionalFieldsError::PrimaryDirectiveCannotRemapObject {
                            primary_object_entity_name,
                            field_name: split_to_arg.to_argument_name.lookup().to_string(),
                        }
                        .err();
                    }

                    argument_map.arguments.swap_remove(index_of_argument);

                    ProcessedFieldMapItem(field_map_item.clone())
                }
                Some((first, rest)) => {
                    let mut arg = ModifiedArgument::from_unmodified(db, unmodified_argument);

                    arg.remove_to_field::<TNetworkProtocol>(
                        *first,
                        rest,
                        primary_object_entity_name,
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
            let to_field_names = &split_to_arg.to_field_names;
            match to_field_names.split_first() {
                None => {
                    // TODO encode this in the type system.
                    // A modified argument will always have an object type, and cannot be remapped
                    // at the object level.
                    return CreateAdditionalFieldsError::PrimaryDirectiveCannotRemapObject {
                        primary_object_entity_name,
                        field_name: split_to_arg.to_argument_name.lookup().to_string(),
                    }
                    .err();
                }
                Some((first, rest)) => {
                    modified.remove_to_field::<TNetworkProtocol>(
                        *first,
                        rest,
                        primary_object_entity_name,
                    )?;
                    // TODO WAT
                    ProcessedFieldMapItem(field_map_item.clone())
                }
            }
        }
    }
    .ok()
}

#[derive(Debug)]
enum PotentiallyModifiedArgument {
    Unmodified(ValidatedVariableDefinition),
    Modified(ModifiedArgument),
}

/// An object which has fields that are unmodified, deleted,
/// or modified (indicating that a new object should be created
/// for them to point to.) Scalar fields cannot be modified,
/// only deleted.
#[derive(Debug)]
pub(crate) struct ModifiedObject {
    field_map: HashMap<ServerSelectableName, PotentiallyModifiedField>,
}

#[derive(Debug)]
pub(crate) enum PotentiallyModifiedField {
    Unmodified(ServerSelectableId),
    // This is exercised in the case of 3+ segments, e.g. input.foo.id.
    // For now, we support only up to two segments.
    #[expect(dead_code)]
    Modified(ModifiedField),
}

/// A modified field's type must be an object. A scalar field that
/// is modified is just removed.
#[derive(Debug)]
pub(crate) struct ModifiedField {
    #[expect(dead_code)]
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
    fn from_unmodified<TNetworkProtocol: NetworkProtocol>(
        db: &IsographDatabase<TNetworkProtocol>,
        unmodified: &VariableDefinition<ServerEntityName>,
    ) -> Self {
        // TODO I think we have validated that the item exists already.
        // But we should double check that, and return an error if necessary
        let object = unmodified.type_.clone().map(|input_type_name| {
            match input_type_name {
                ServerEntityName::Object(object_entity_name) => {
                    let field_map = server_selectables_vec_for_entity(db, object_entity_name)
                        .as_ref()
                        .expect(
                            "Expected parsing to have worked. \
                            This is indicative of a bug in Isograph.",
                        )
                        .iter()
                        .flat_map(|(name, result)| {
                            if let Ok(v) = result {
                                (
                                    *name,
                                    PotentiallyModifiedField::Unmodified(match v {
                                        SelectionType::Scalar(s) => SelectionType::Scalar((
                                            s.parent_object_entity_name,
                                            s.name.item,
                                        )),
                                        SelectionType::Object(o) => SelectionType::Object((
                                            o.parent_object_entity_name,
                                            o.name.item,
                                        )),
                                    }),
                                )
                                    .some()
                            } else {
                                None
                            }
                        })
                        .collect();
                    ModifiedObject { field_map }
                }
                ServerEntityName::Scalar(_scalar_entity_name) => {
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

    fn remove_to_field<TNetworkProtocol: NetworkProtocol>(
        &mut self,
        first: StringLiteralValue,
        rest: &[StringLiteralValue],
        primary_object_entity_name: ServerObjectEntityName,
    ) -> ProcessTypeDefinitionResult<(), TNetworkProtocol> {
        let argument_object = self.object.inner_mut();

        let key = first.unchecked_conversion();
        match argument_object
            .field_map
            // TODO make this a no-op
            .get_mut(&key)
        {
            Some(field) => {
                match rest.split_first() {
                    Some((_, _)) => {
                        unimplemented!("Removing to fields from PotentiallyModifiedField");
                    }
                    None => {
                        // We ran out of path segments, so we remove this item.
                        // It must have a scalar type.

                        match field {
                            PotentiallyModifiedField::Unmodified(field_id) => {
                                if field_id.as_object().is_some() {
                                    return CreateAdditionalFieldsError::PrimaryDirectiveCannotRemapObject {
                                        primary_object_entity_name,
                                        field_name: key.lookup().to_string(),
                                    }.err();
                                }

                                // Cool! We found a scalar, we can remove it.
                                argument_object.field_map.remove(&key).expect(
                                    "Expected to be able to remove item. \
                                        This is indicative of a bug in Isograph.",
                                );
                            }
                            PotentiallyModifiedField::Modified(_) => {
                                // A field can only be modified if it has an object type
                                return CreateAdditionalFieldsError::PrimaryDirectiveCannotRemapObject {
                                    primary_object_entity_name,
                                    field_name: key.to_string(),
                                }.err();
                            }
                        }
                    }
                }
            }
            None => {
                return CreateAdditionalFieldsError::PrimaryDirectiveFieldNotFound {
                    primary_object_entity_name,
                    field_name: first,
                }
                .err();
            }
        };
        Ok(())
    }
}

#[expect(dead_code)]
enum IsEmpty {
    IsEmpty,
    NotEmpty,
}
