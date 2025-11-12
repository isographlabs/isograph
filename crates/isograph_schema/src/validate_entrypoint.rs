use std::{
    collections::{HashMap, hash_map::Entry},
    ops::Deref,
};

use common_lang_types::{
    ClientScalarSelectableName, IsoLiteralText, Location, ServerObjectEntityName, TextSource,
    UnvalidatedTypeName, WithLocation, WithSpan,
};
use intern::Lookup;
use isograph_lang_types::{
    ClientScalarSelectableNameWrapper, EntrypointDeclaration, EntrypointDirectiveSet,
    ServerObjectEntityNameWrapper,
};
use thiserror::Error;

use crate::{
    IsographDatabase, MemoizedIsoLiteralError, NetworkProtocol, SelectableNamedError,
    ServerEntityName, client_scalar_selectable_named, defined_entity, fetchable_types,
    selectable_named,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EntrypointDeclarationInfo {
    pub iso_literal_text: IsoLiteralText,
    pub directive_set: EntrypointDirectiveSet,
}

#[expect(clippy::type_complexity)]
pub fn validate_entrypoints<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    entrypoint_declarations: Vec<(TextSource, WithSpan<EntrypointDeclaration>)>,
) -> Result<
    HashMap<(ServerObjectEntityName, ClientScalarSelectableName), EntrypointDeclarationInfo>,
    Vec<WithLocation<ValidateEntrypointDeclarationError<TNetworkProtocol>>>,
> {
    let mut errors = vec![];
    let mut entrypoints: HashMap<
        (ServerObjectEntityName, ClientScalarSelectableName),
        EntrypointDeclarationInfo,
    > = HashMap::new();
    for (text_source, entrypoint_declaration) in entrypoint_declarations {
        match validate_entrypoint_type_and_field(db, text_source, &entrypoint_declaration) {
            Ok(client_field_id) => {
                let new_entrypoint = EntrypointDeclarationInfo {
                    iso_literal_text: entrypoint_declaration.item.iso_literal_text,
                    directive_set: entrypoint_declaration.item.entrypoint_directive_set,
                };
                match entrypoints.entry((
                    entrypoint_declaration
                        .item
                        .parent_type
                        .item
                        .0
                        .unchecked_conversion(),
                    client_field_id,
                )) {
                    Entry::Occupied(occupied_entry) => {
                        if occupied_entry.get().directive_set != new_entrypoint.directive_set {
                            errors.push(WithLocation::new(
                                ValidateEntrypointDeclarationError::LazyLoadInconsistentEntrypoint,
                                Location::new(
                                    text_source,
                                    entrypoint_declaration.item.entrypoint_keyword.span,
                                ),
                            ));
                        }
                    }
                    Entry::Vacant(vacant) => {
                        vacant.insert(new_entrypoint);
                    }
                }
            }
            Err(e) => {
                errors.push(e);
            }
        }
    }

    if errors.is_empty() {
        Ok(entrypoints)
    } else {
        Err(errors)
    }
}

fn validate_entrypoint_type_and_field<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    text_source: TextSource,
    entrypoint_declaration: &WithSpan<EntrypointDeclaration>,
) -> Result<
    ClientScalarSelectableName,
    WithLocation<ValidateEntrypointDeclarationError<TNetworkProtocol>>,
> {
    let parent_object_entity_name = validate_parent_object_entity_name(
        db,
        entrypoint_declaration.item.parent_type,
        text_source,
    )?;

    validate_client_field_exists(
        db,
        entrypoint_declaration.item.client_field_name,
        text_source,
        parent_object_entity_name,
    )?;

    Ok(entrypoint_declaration.item.client_field_name.item.0)
}

fn validate_parent_object_entity_name<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: WithSpan<ServerObjectEntityNameWrapper>,
    text_source: TextSource,
) -> Result<
    ServerObjectEntityName,
    WithLocation<ValidateEntrypointDeclarationError<TNetworkProtocol>>,
> {
    let parent_type_id = defined_entity(db, parent_object_entity_name.item.0.into())
        .to_owned()
        .expect(
            "Expected parsing to have succeeded. \
            This is indicative of a bug in Isograph.",
        )
        .ok_or(WithLocation::new(
            ValidateEntrypointDeclarationError::ParentTypeNotDefined {
                parent_object_entity_name: parent_object_entity_name.item,
            },
            Location::new(text_source, parent_object_entity_name.span),
        ))?;

    match parent_type_id {
        ServerEntityName::Object(object_entity_name) => {
            let is_fetchable = fetchable_types(db)
                .deref()
                .as_ref()
                .expect(
                    "Expected parsing to have succeeded. \
                This is indicative of a bug in Isograph.",
                )
                .contains_key(&object_entity_name);

            if !is_fetchable {
                let fetchable_types = fetchable_types(db)
                    .deref()
                    .as_ref()
                    .expect(
                        "Expected parsing to have succeeded. \
                        This is indicative of a bug in Isograph.",
                    )
                    .keys()
                    .map(|object_entity_name| object_entity_name.lookup())
                    .collect::<Vec<_>>()
                    .join(", ");

                Err(WithLocation::new(
                    ValidateEntrypointDeclarationError::NonFetchableParentType {
                        parent_object_entity_name: parent_object_entity_name.item,
                        fetchable_types,
                    },
                    Location::new(text_source, parent_object_entity_name.span),
                ))
            } else {
                Ok(object_entity_name)
            }
        }
        ServerEntityName::Scalar(scalar_entity_name) => Err(WithLocation::new(
            ValidateEntrypointDeclarationError::InvalidParentType {
                parent_type: "scalar",
                parent_object_entity_name: scalar_entity_name.into(),
            },
            Location::new(text_source, parent_object_entity_name.span),
        )),
    }
}

fn validate_client_field_exists<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    field_name: WithSpan<ClientScalarSelectableNameWrapper>,
    text_source: TextSource,
    parent_object_entity_name: ServerObjectEntityName,
) -> Result<(), WithLocation<ValidateEntrypointDeclarationError<TNetworkProtocol>>> {
    let memo_ref = client_scalar_selectable_named(db, parent_object_entity_name, field_name.item.0);

    match memo_ref
        .deref()
        .as_ref()
        .map_err(|e| WithLocation::new(e.clone().into(), Location::Generated))?
    {
        Some(_) => Ok(()),
        None => {
            // check whether it is anything else
            let other_memo_ref =
                selectable_named(db, parent_object_entity_name, field_name.item.0.into());

            match other_memo_ref
                .deref()
                .as_ref()
                .map_err(|e| WithLocation::new(e.clone().into(), Location::Generated))?
            {
                Some(_) => Err(WithLocation::new(
                    ValidateEntrypointDeclarationError::FieldMustBeClientField {
                        parent_object_entity_name,
                        client_field_name: field_name.item,
                    },
                    Location::new(text_source, field_name.span),
                )),
                None => Err(WithLocation::new(
                    ValidateEntrypointDeclarationError::ClientFieldMustExist {
                        parent_object_entity_name,
                        client_field_name: field_name.item,
                    },
                    Location::new(text_source, field_name.span),
                )),
            }
        }
    }
}

#[derive(Error, Eq, PartialEq, Debug, Clone)]
pub enum ValidateEntrypointDeclarationError<TNetworkProtocol: NetworkProtocol> {
    #[error("`{parent_object_entity_name}` is not a type that has been defined.")]
    ParentTypeNotDefined {
        parent_object_entity_name: ServerObjectEntityNameWrapper,
    },

    #[error(
        "Invalid parent type. `{parent_object_entity_name}` is a {parent_type}, but it should be an object or interface."
    )]
    InvalidParentType {
        parent_type: &'static str,
        parent_object_entity_name: UnvalidatedTypeName,
    },

    #[error(
        "The type `{parent_object_entity_name}` is not fetchable. The following types are fetchable: {fetchable_types}."
    )]
    NonFetchableParentType {
        parent_object_entity_name: ServerObjectEntityNameWrapper,
        fetchable_types: String,
    },

    #[error("The client field `{parent_object_entity_name}.{client_field_name}` is not defined.")]
    ClientFieldMustExist {
        parent_object_entity_name: ServerObjectEntityName,
        client_field_name: ClientScalarSelectableNameWrapper,
    },

    // N.B. We could conceivably support fetching server fields, though!
    #[error(
        "The field `{parent_object_entity_name}.{client_field_name}` is a server field. It must be a client defined field."
    )]
    FieldMustBeClientField {
        parent_object_entity_name: ServerObjectEntityName,
        client_field_name: ClientScalarSelectableNameWrapper,
    },

    #[error(
        "Entrypoint declared lazy in one location and declared eager in another location. Entrypoint must be either lazy or non-lazy in all instances."
    )]
    LazyLoadInconsistentEntrypoint,

    #[error("{0}")]
    MemoizedIsoLiteralError(#[from] MemoizedIsoLiteralError<TNetworkProtocol>),

    #[error("{0}")]
    SelectableNamedError(#[from] SelectableNamedError<TNetworkProtocol>),
}
