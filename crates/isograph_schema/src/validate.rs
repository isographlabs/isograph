use common_lang_types::WithLocation;
use isograph_lang_parser::IsographLiteralParseError;
use pico_macros::memo;
use prelude::Postfix;
use thiserror::Error;

use crate::{
    ContainsIsoStats, CreateAdditionalFieldsError, CreateSchemaError,
    FieldToInsertToServerSelectableError, IsographDatabase, NetworkProtocol,
    ProcessClientFieldDeclarationError, ValidateUseOfArgumentsError, ValidatedEntrypointError,
    create_new_exposed_field, create_type_system_schema_with_server_selectables,
    parse_iso_literals, process_iso_literals, server_selectables_map, validate_use_of_arguments,
    validated_entrypoints,
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
///
/// TODO we return early in a few places, and throw away already-accumulated
/// errors, which we should not do.
#[memo]
pub fn validate_entire_schema<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<ContainsIsoStats, Vec<ValidationError<TNetworkProtocol>>> {
    let mut errors = vec![];

    if let Err(e) = validate_use_of_arguments(db) {
        errors.extend(e.iter().map(|e| ValidationError::from(e.item.clone())))
    }

    if let Err(e) = validate_all_server_selectables_point_to_defined_types(db).to_owned() {
        errors.extend(e);
    }

    errors.extend(validated_entrypoints(db).values().flat_map(|result| {
        ValidationError::ValidatedEntrypointError(result.as_ref().err()?.clone()).some()
    }));

    // validate that each expose field is defined correctly
    let expose_as_field_queue = create_type_system_schema_with_server_selectables(db)
        .as_ref()
        .map_err(|e| vec![e.clone().into()])?;

    for (parent_object_entity_name, expose_as_fields_to_insert) in expose_as_field_queue {
        for expose_as_field in expose_as_fields_to_insert {
            if let Err(e) =
                create_new_exposed_field(db, expose_as_field, *parent_object_entity_name)
            {
                errors.push(e.into());
            }
        }
    }

    // Process all iso literals
    let contains_iso = parse_iso_literals(db).to_owned().map_err(|errors| {
        errors
            .into_iter()
            .map(|e| ValidationError::IsographLiteralParseError { message: e })
            .collect::<Vec<_>>()
    })?;
    let contains_iso_stats = contains_iso.stats();

    if let Err(e) = process_iso_literals(db, contains_iso) {
        errors.extend(
            e.into_iter()
                .map(|e| ValidationError::ProcessClientFieldDeclarationError { error: e }),
        )
    }

    if errors.is_empty() {
        Ok(contains_iso_stats)
    } else {
        Err(errors)
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Error)]
pub enum ValidationError<TNetworkProtocol: NetworkProtocol> {
    #[error("{0}")]
    ValidateUseOfArgumentsError(#[from] ValidateUseOfArgumentsError<TNetworkProtocol>),

    #[error("{0}")]
    ParseTypeSystemDocumentsError(TNetworkProtocol::ParseTypeSystemDocumentsError),

    #[error("{0}")]
    FieldToInsertToServerSelectableError(#[from] FieldToInsertToServerSelectableError),

    #[error("{0}")]
    ValidatedEntrypointError(#[from] ValidatedEntrypointError<TNetworkProtocol>),

    #[error("{0}")]
    CreateAdditionalFieldsError(#[from] CreateAdditionalFieldsError<TNetworkProtocol>),

    #[error("{0}")]
    CreateSchemaError(#[from] CreateSchemaError<TNetworkProtocol>),

    #[error("{}", message.for_display())]
    IsographLiteralParseError {
        message: WithLocation<IsographLiteralParseError>,
    },

    #[error("{}", error.for_display())]
    ProcessClientFieldDeclarationError {
        error: WithLocation<ProcessClientFieldDeclarationError<TNetworkProtocol>>,
    },
}

#[memo]
fn validate_all_server_selectables_point_to_defined_types<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<(), Vec<ValidationError<TNetworkProtocol>>> {
    // Note: server_selectables_map is a HashMap<_, Vec<(_, Result)>
    // That result encodes whether the field exists. So, basically, we are collecting
    // each error from that result.
    //
    // This can and should be rethought! Namely, just because the referenced entity doesn't exist
    // doesn't mean that the selectable can't be materialized. Instead, the result should be
    // materialized when we actually need to look at the referenced entity.
    let server_selectables = server_selectables_map(db)
        .as_ref()
        .map_err(|e| ValidationError::ParseTypeSystemDocumentsError(e.clone()))
        .map_err(|e| vec![e])?;

    let mut errors = vec![];

    for selectables in server_selectables.values() {
        for (_, selectable_result) in selectables {
            if let Err(e) = selectable_result {
                errors.push(e.clone().into());
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
