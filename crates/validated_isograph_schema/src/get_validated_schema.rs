use std::ops::Deref;

use common_lang_types::WithLocation;
use isograph_schema::{
    CreateAdditionalFieldsError, IsographDatabase, NetworkProtocol, Schema,
    ValidateUseOfArgumentsError, create_new_exposed_field, validate_use_of_arguments,
};
use pico_macros::legacy_memo;
use thiserror::Error;

use crate::{
    CreateSchemaError,
    add_link_fields::add_link_fields_to_schema,
    create_type_system_schema::process_field_queue,
    process_iso_literals::{
        ContainsIsoStats, ProcessIsoLiteralsForSchemaError, process_iso_literals_for_schema,
    },
};

#[legacy_memo]
pub fn get_validated_schema<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<(Schema<TNetworkProtocol>, ContainsIsoStats), GetValidatedSchemaError<TNetworkProtocol>>
{
    let memo_ref =
        crate::create_type_system_schema::create_type_system_schema_with_server_selectables(db);
    let (expose_as_field_queue, field_queue) = memo_ref.deref().as_ref().map_err(|e| e.clone())?;

    let mut unvalidated_isograph_schema = Schema::new();

    process_field_queue(db, &mut unvalidated_isograph_schema, field_queue)?;

    let mut unprocessed_selection_set = std::vec![];

    for (parent_object_entity_name, expose_as_fields_to_insert) in expose_as_field_queue {
        for expose_as_field in expose_as_fields_to_insert {
            let (
                unprocessed_client_scalar_selection_set,
                exposed_field_client_scalar_selectable,
                payload_object_entity_name,
            ) = create_new_exposed_field(db, expose_as_field, *parent_object_entity_name)?;

            let client_scalar_selectable_name = exposed_field_client_scalar_selectable.name.item;
            let parent_object_entity_name =
                exposed_field_client_scalar_selectable.parent_object_entity_name;

            unvalidated_isograph_schema
                .client_scalar_selectables
                .insert(
                    (
                        exposed_field_client_scalar_selectable.parent_object_entity_name,
                        client_scalar_selectable_name,
                    ),
                    exposed_field_client_scalar_selectable,
                );

            unvalidated_isograph_schema.insert_client_field_on_object(
                parent_object_entity_name,
                client_scalar_selectable_name,
                payload_object_entity_name,
            )?;

            unprocessed_selection_set.push(isograph_lang_types::SelectionType::Scalar(
                unprocessed_client_scalar_selection_set,
            ));
        }
    }

    add_link_fields_to_schema(db, &mut unvalidated_isograph_schema)?;

    let (isograph_schema, stats) = process_iso_literals_for_schema(
        db,
        unvalidated_isograph_schema,
        unprocessed_selection_set,
    )?;
    validate_use_of_arguments(db, &isograph_schema)?;
    Ok((isograph_schema, stats))
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum GetValidatedSchemaError<TNetworkProtocol: NetworkProtocol> {
    #[error(
        "{}",
        messages.iter().fold(String::new(), |mut output, x| {
            output.push_str(&format!("\n\n{}", x.for_display()));
            output
        })
    )]
    ValidateUseOfArguments {
        messages: Vec<WithLocation<ValidateUseOfArgumentsError>>,
    },

    #[error("{error}")]
    CreateSchemaError {
        #[from]
        error: CreateSchemaError<TNetworkProtocol>,
    },

    #[error("{error}")]
    ProcessIsoLiteralsForSchemaError {
        #[from]
        error: ProcessIsoLiteralsForSchemaError<TNetworkProtocol>,
    },

    #[error("{0}")]
    CreateAdditionalFieldsError(#[from] CreateAdditionalFieldsError<TNetworkProtocol>),
}

impl<TNetworkProtocol: NetworkProtocol> From<Vec<WithLocation<ValidateUseOfArgumentsError>>>
    for GetValidatedSchemaError<TNetworkProtocol>
{
    fn from(messages: Vec<WithLocation<ValidateUseOfArgumentsError>>) -> Self {
        GetValidatedSchemaError::ValidateUseOfArguments { messages }
    }
}
