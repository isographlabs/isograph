use crate::{
    ContainsIsoStats, CreateAdditionalFieldsError, IsographDatabase, NetworkProtocol,
    ProcessIsoLiteralsForSchemaError, Schema, ValidateUseOfArgumentsError,
    create_new_exposed_field, process_iso_literals_for_schema, validate_use_of_arguments,
    validated_isograph_schema::create_type_system_schema::{
        CreateSchemaError, create_type_system_schema_with_server_selectables, process_field_queue,
    },
};
use common_lang_types::WithLocation;
use isograph_lang_types::SelectionType;
use pico_macros::memo;
use thiserror::Error;

#[memo]
pub fn get_validated_schema<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<(Schema, ContainsIsoStats), GetValidatedSchemaError<TNetworkProtocol>> {
    let (expose_as_field_queue, field_queue) =
        create_type_system_schema_with_server_selectables(db)
            .as_ref()
            .map_err(|e| e.clone())?;

    let mut unvalidated_isograph_schema = Schema::new();

    process_field_queue(db, &mut unvalidated_isograph_schema, field_queue)?;

    let mut unprocessed_selection_sets = vec![];

    for (parent_object_entity_name, expose_as_fields_to_insert) in expose_as_field_queue {
        for expose_as_field in expose_as_fields_to_insert {
            let (unprocessed_client_scalar_selection_set, _, _) =
                create_new_exposed_field(db, expose_as_field, *parent_object_entity_name)?;

            unprocessed_selection_sets.push(SelectionType::Scalar(
                unprocessed_client_scalar_selection_set,
            ));
        }
    }

    let stats = process_iso_literals_for_schema(db, unprocessed_selection_sets)?;
    validate_use_of_arguments(db, &unvalidated_isograph_schema)?;
    Ok((unvalidated_isograph_schema, stats))
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
        messages: Vec<WithLocation<ValidateUseOfArgumentsError<TNetworkProtocol>>>,
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

impl<TNetworkProtocol: NetworkProtocol>
    From<Vec<WithLocation<ValidateUseOfArgumentsError<TNetworkProtocol>>>>
    for GetValidatedSchemaError<TNetworkProtocol>
{
    fn from(messages: Vec<WithLocation<ValidateUseOfArgumentsError<TNetworkProtocol>>>) -> Self {
        GetValidatedSchemaError::ValidateUseOfArguments { messages }
    }
}
