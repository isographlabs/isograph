use common_lang_types::ArtifactPathAndContent;
use isograph_config::CompilerConfig;

use crate::ValidatedSchema;

pub trait OutputFormat: std::fmt::Debug
where
    Self: Sized,
{
    fn generate_artifact_path_and_content(
        schema: &ValidatedSchema<Self>,
        config: &CompilerConfig,
    ) -> Vec<ArtifactPathAndContent>;
}
