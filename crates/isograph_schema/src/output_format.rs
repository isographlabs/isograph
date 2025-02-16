use std::{fmt::Debug, hash::Hash};

use common_lang_types::ArtifactPathAndContent;
use isograph_config::CompilerConfig;

use crate::ValidatedSchema;

pub trait OutputFormat:
    Debug + Clone + Copy + Eq + PartialEq + Ord + PartialOrd + Hash + Default
where
    Self: Sized,
{
    // These two types should be combined into a single type, this is a
    // GraphQL-ism leaking in
    type TypeSystemDocument: Debug;
    type TypeSystemExtensionDocument: Debug;

    fn generate_artifact_path_and_content(
        schema: &ValidatedSchema<Self>,
        config: &CompilerConfig,
    ) -> Vec<ArtifactPathAndContent>;
}
