use common_lang_types::ArtifactPathAndContent;
use intern::string_key::Intern;

pub fn generate_ts_config() -> ArtifactPathAndContent {
    ArtifactPathAndContent {
        type_and_field: None,
        file_name: "tsconfig.json".intern().into(),
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
    }
}
