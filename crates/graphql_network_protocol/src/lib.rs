use std::path::Path;

use isograph_compiler::{IncludeFile, NetworkProtocol};

pub struct GraphQLNetworkProtocol;

impl NetworkProtocol for GraphQLNetworkProtocol {
    type EntityData = ();
    type SelectableData = ();

    fn should_include_schema_file(relative_path: &Path) -> IncludeFile {
        match relative_path.extension().and_then(|e| e.to_str()) {
            Some("graphql" | "graphqls") => IncludeFile::Include,
            _ => IncludeFile::Skip,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use isograph_compiler::{IncludeFile, NetworkProtocol};

    use super::GraphQLNetworkProtocol;

    #[test]
    fn should_include_schema_file_includes_graphql_and_graphqls() {
        for path in ["a.graphql", "schema/a.graphqls"] {
            assert_eq!(
                GraphQLNetworkProtocol::should_include_schema_file(Path::new(path)),
                IncludeFile::Include,
                "{path}"
            );
        }
        for path in ["a.gql", "a.ts", "a.json", "graphql", ""] {
            assert_eq!(
                GraphQLNetworkProtocol::should_include_schema_file(Path::new(path)),
                IncludeFile::Skip,
                "{path}"
            );
        }
    }
}
