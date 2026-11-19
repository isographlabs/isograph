use std::path::PathBuf;

use serde::Deserialize;

pub static ISOGRAPH_FOLDER: &str = "__isograph";

/// This struct is deserialized from an isograph.config.json file.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct IsographProjectConfig {
    /// The user may hard-code the JSON Schema for their version of the config.
    #[serde(rename = "$schema")]
    pub json_schema: Option<String>,
    /// Glob patterns relative to the config file's directory.
    pub source_files: Vec<String>,
    /// Glob patterns relative to the config file's directory.
    #[serde(default)]
    pub schema: Vec<String>,
    /// The directory, relative to the config file's directory, whose `__isograph` folder holds
    /// the generated artifacts. Absent: the compiler writes no artifacts, and the transforms
    /// look in the config file's directory.
    #[serde(default)]
    pub output_directory: Option<PathBuf>,
    /// Options the GraphQL network protocol reads.
    #[serde(default)]
    pub graphql: GraphQLConfig,
}

/// The `graphql` section of the config.
#[derive(Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct GraphQLConfig {
    /// How each entrypoint's query text is laid out.
    pub query_text_format: QueryTextFormat,
}

/// How an entrypoint's query text is laid out. Both formats lex to the same tokens.
#[derive(Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryTextFormat {
    /// One selection per line, two spaces of indentation per depth.
    Expanded,
    /// One line, with a space only between two tokens that would otherwise lex as one.
    #[default]
    Collapsed,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use prelude::Postfix;

    use super::{ISOGRAPH_FOLDER, IsographProjectConfig, QueryTextFormat};

    #[test]
    fn isograph_folder_is_the_generated_directory_name() {
        assert_eq!(ISOGRAPH_FOLDER, "__isograph");
    }

    #[test]
    fn a_config_without_schema_has_no_schema_globs() {
        let config: IsographProjectConfig =
            serde_json::from_str(r#"{"source_files":["src/**"]}"#).expect("the config is valid");
        assert_eq!(config.source_files, ["src/**"]);
        assert_eq!(config.schema, Vec::<String>::new());
    }

    #[test]
    fn a_config_with_schema_has_those_globs() {
        let config: IsographProjectConfig =
            serde_json::from_str(r#"{"source_files":[],"schema":["schema/**","!schema/old/**"]}"#)
                .expect("the config is valid");
        assert_eq!(config.schema, ["schema/**", "!schema/old/**"]);
    }

    #[test]
    fn a_config_with_an_output_directory_has_it() {
        let config: IsographProjectConfig =
            serde_json::from_str(r#"{"source_files":[],"output_directory":"src"}"#)
                .expect("the config is valid");
        assert_eq!(config.output_directory, PathBuf::from("src").wrap_some());
    }

    #[test]
    fn a_config_without_an_output_directory_has_none() {
        let config: IsographProjectConfig =
            serde_json::from_str(r#"{"source_files":[]}"#).expect("the config is valid");
        assert_eq!(config.output_directory, None);
    }

    fn query_text_format(json: &str) -> QueryTextFormat {
        serde_json::from_str::<IsographProjectConfig>(json)
            .expect("the config is valid")
            .graphql
            .query_text_format
    }

    #[test]
    fn a_config_without_a_query_text_format_is_collapsed() {
        assert_eq!(
            query_text_format(r#"{"source_files":[]}"#),
            QueryTextFormat::Collapsed
        );
        assert_eq!(
            query_text_format(r#"{"source_files":[],"graphql":{}}"#),
            QueryTextFormat::Collapsed
        );
    }

    #[test]
    fn a_config_with_a_query_text_format_has_it() {
        assert_eq!(
            query_text_format(r#"{"source_files":[],"graphql":{"query_text_format":"expanded"}}"#),
            QueryTextFormat::Expanded
        );
        assert_eq!(
            query_text_format(r#"{"source_files":[],"graphql":{"query_text_format":"collapsed"}}"#),
            QueryTextFormat::Collapsed
        );
    }

    #[test]
    fn an_unknown_query_text_format_is_an_error_naming_the_formats() {
        let error = serde_json::from_str::<IsographProjectConfig>(
            r#"{"source_files":[],"graphql":{"query_text_format":"compact"}}"#,
        )
        .expect_err("compact is not a format");
        let message = error.to_string();
        assert!(message.contains("compact"), "{message}");
        assert!(message.contains("expanded"), "{message}");
        assert!(message.contains("collapsed"), "{message}");
    }

    #[test]
    fn an_unknown_key_in_graphql_is_an_error() {
        let error = serde_json::from_str::<IsographProjectConfig>(
            r#"{"source_files":[],"graphql":{"query_text_formats":"expanded"}}"#,
        )
        .expect_err("deny_unknown_fields rejects query_text_formats");
        assert!(error.to_string().contains("query_text_formats"), "{error}");
    }

    #[test]
    fn a_config_with_an_unknown_key_is_an_error() {
        let error =
            serde_json::from_str::<IsographProjectConfig>(r#"{"source_files":[],"schemas":[]}"#)
                .expect_err("deny_unknown_fields rejects schemas");
        assert!(error.to_string().contains("schemas"), "{error}");
    }
}
