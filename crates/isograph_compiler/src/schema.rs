use common_lang_types::{Diagnostic, DiagnosticResult, EntityName, SelectableName};
use pico_macros::memo;
use prelude::Postfix;

use crate::{ClientId, CompilationProfile, IsographDb};

/// A node of the schema graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entity {
    pub name: EntityName,
}

/// A named pointer from an entity to an entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selectable {
    pub name: SelectableName,
    pub target_entity: EntityName,
}

/// The entity named `entity_name` in the schema as `client` reads it.
#[memo]
pub fn entity_named<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    _client: ClientId,
    entity_name: EntityName,
) -> DiagnosticResult<Entity> {
    Diagnostic::new(
        format!("The schema is not available yet, so entity `{entity_name}` cannot be looked up."),
        None,
    )
    .wrap_err()
}

/// The selectable named `selectable_name` on the entity named `entity_name` in the schema as
/// `client` reads it.
#[memo]
pub fn selectable_named<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    _client: ClientId,
    entity_name: EntityName,
    selectable_name: SelectableName,
) -> DiagnosticResult<Selectable> {
    Diagnostic::new(
        format!(
            "The schema is not available yet, so selectable \
            `{entity_name}.{selectable_name}` cannot be looked up."
        ),
        None,
    )
    .wrap_err()
}
