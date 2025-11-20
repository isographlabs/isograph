use pico_macros::memo;
use thiserror::Error;

use crate::{
    IsographDatabase, NetworkProtocol, ValidateUseOfArgumentsError, validate_use_of_arguments,
};

/// In the world of pico, we minimally validate. For example, if the
/// schema contains a field `foo: Bar`, and `Bar` is undefined and
/// unreferenced, then we will never actually ensure that `Bar` is
/// actually defined!
///
/// So, we need to define a function where we do all of the validation.
///
/// This is opt-in, but it makes sense to call this before we generate
/// artifacts. However, whether we do these strictly-unnecessary
/// validations should be controllable by the user.
#[memo]
pub fn validate_entire_schema<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<(), Vec<ValidationError<TNetworkProtocol>>> {
    let mut errors = vec![];

    if let Err(e) = validate_use_of_arguments(db) {
        errors.extend(e.iter().map(|e| ValidationError::from(e.item.clone())))
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Error)]
pub enum ValidationError<TNetworkProtocol: NetworkProtocol> {
    #[error("{0}")]
    ValidateUseOfArgumentsError(#[from] ValidateUseOfArgumentsError<TNetworkProtocol>),
}
