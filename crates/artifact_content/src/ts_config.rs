use common_lang_types::{ArtifactPath, ArtifactPathAndContent};
use intern::string_key::Intern;

pub fn generate_ts_config() -> ArtifactPathAndContent {
    ArtifactPathAndContent {
        file_content: "{
    \"compilerOptions\": {
        \"noUnusedLocals\": false,
        \"noUnusedParameters\": false,
        \"jsx\": \"preserve\",
        \"esModuleInterop\": true
    }
}"
        .to_string()
        .into(),
        artifact_path: ArtifactPath {
            file_name: "tsconfig.json".intern().into(),
            type_and_field: None,
        },
    }
}
