use crate::{
    ContainsIsoStats, CreateAdditionalFieldsError, IsographDatabase, NetworkProtocol,
    ProcessIsoLiteralsForSchemaError, ValidateUseOfArgumentsError, process_iso_literals_for_schema,
    validated_isograph_schema::create_type_system_schema::CreateSchemaError,
};
use common_lang_types::WithLocation;
use pico_macros::memo;
use thiserror::Error;

#[memo]
pub fn get_validated_schema<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<ContainsIsoStats, GetValidatedSchemaError<TNetworkProtocol>> {
    let stats = process_iso_literals_for_schema(db)?;
    Ok(stats)
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
