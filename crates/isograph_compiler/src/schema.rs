use common_lang_types::{
    DescriptionValue, Diagnostic, DiagnosticResult, EntityName, RelativePathToSourceFile,
    SelectableName,
};
use pico_macros::memo;
use prelude::Postfix;
use span::Span;

use crate::{ClientId, CompilationProfile, IsographDb};

/// A node of the schema graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entity {
    pub name: EntityName,
    pub definition: DefinitionLocation,
}

/// A named pointer from an entity to an entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selectable {
    pub name: SelectableName,
    pub parent_entity: EntityName,
    pub description: Option<DescriptionValue>,
    pub target_entity: EntityName,
    pub definition: DefinitionLocation,
}

/// Where an entity or selectable is defined: a span of a file, in that file's byte offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DefinitionLocation {
    pub path: RelativePathToSourceFile,
    pub span: Span,
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
