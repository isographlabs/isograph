mod location;
mod span;
mod string_key_types;
mod text_with_carats;
mod traits;

pub use location::*;
pub use span::*;
pub use string_key_types::*;
pub use traits::*;

// TODO this doesn't belong here
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum GraphQLArtifactGenerationInfo {
    ServerField,
    TypeRefinement(IsographObjectTypeName),
}
