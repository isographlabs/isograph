use std::ops::Deref;

use common_lang_types::WithLocation;
use isograph_schema::{
    validate_use_of_arguments, IsographDatabase, NetworkProtocol, Schema,
    ValidateUseOfArgumentsError,
};
use pico_macros::memo;
use thiserror::Error;

use crate::{
    create_schema, process_iso_literals_for_schema, ContainsIsoStats, CreateSchemaError,
    ProcessIsoLiteralsForSchemaError,
};

#[memo]
pub fn get_validated_schema<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<(Schema<TNetworkProtocol>, ContainsIsoStats), GetValidatedSchemaError<TNetworkProtocol>>
{
    let (unvalidated_isograph_schema, unprocessed_items) =
        create_schema::<TNetworkProtocol>(db).deref().clone()?;
    let (isograph_schema, stats) =
        process_iso_literals_for_schema(db, unvalidated_isograph_schema, unprocessed_items)?;
    validate_use_of_arguments(&isograph_schema)?;
    Ok((isograph_schema, stats))
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum GetValidatedSchemaError<TNetworkProtocol: NetworkProtocol + 'static> {
    #[error(
        "{}",
        messages.iter().fold(String::new(), |mut output, x| {
            output.push_str(&format!("\n\n{x}"));
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
        error: ProcessIsoLiteralsForSchemaError,
    },
}

impl<TNetworkProtocol: NetworkProtocol + 'static>
    From<Vec<WithLocation<ValidateUseOfArgumentsError>>>
    for GetValidatedSchemaError<TNetworkProtocol>
{
    fn from(messages: Vec<WithLocation<ValidateUseOfArgumentsError>>) -> Self {
        GetValidatedSchemaError::ValidateUseOfArguments { messages }
    }
}
