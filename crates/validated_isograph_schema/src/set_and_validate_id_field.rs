use common_lang_types::{
    ServerObjectEntityName, ServerScalarIdSelectableName, ServerScalarSelectableName,
    UnvalidatedTypeName,
};
use graphql_lang_types::GraphQLNamedTypeAnnotation;
use isograph_schema::{
    CreateAdditionalFieldsError, CreateAdditionalFieldsResult, ID_ENTITY_NAME, IsographDatabase,
    NetworkProtocol,
};

/// If we have encountered an id field, we can:
/// - validate that the id field is properly defined, i.e. has type ID!
/// - set the id field
pub(crate) fn set_and_validate_id_field<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    id_field: &mut Option<ServerScalarIdSelectableName>,
    current_field_selectable_name: ServerScalarSelectableName,
    parent_object_entity_name: ServerObjectEntityName,
    inner_non_null_named_type: Option<&GraphQLNamedTypeAnnotation<UnvalidatedTypeName>>,
) -> CreateAdditionalFieldsResult<(), TNetworkProtocol> {
    let options = &db.get_isograph_config().options;
    // N.B. id_field is guaranteed to be None; otherwise field_names_to_type_name would
    // have contained this field name already.
    debug_assert!(id_field.is_none(), "id field should not be defined twice");

    // We should change the type here! It should not be ID! It should be a
    // type specific to the concrete type, e.g. UserID.
    *id_field = Some(current_field_selectable_name.unchecked_conversion());

    match inner_non_null_named_type {
        Some(type_) => {
            if type_.0.item != *ID_ENTITY_NAME {
                options.on_invalid_id_type.on_failure(|| {
                    CreateAdditionalFieldsError::IdFieldMustBeNonNullIdType {
                        strong_field_name: "id",
                        parent_object_entity_name,
                    }
                })?;
            }
            Ok(())
        }
        None => {
            options.on_invalid_id_type.on_failure(|| {
                CreateAdditionalFieldsError::IdFieldMustBeNonNullIdType {
                    strong_field_name: "id",
                    parent_object_entity_name,
                }
            })?;
            Ok(())
        }
    }
}
