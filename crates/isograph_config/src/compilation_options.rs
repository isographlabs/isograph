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
}

#[cfg(test)]
mod tests {
    use super::ISOGRAPH_FOLDER;

    #[test]
    fn isograph_folder_is_the_generated_directory_name() {
        assert_eq!(ISOGRAPH_FOLDER, "__isograph");
    }
}
