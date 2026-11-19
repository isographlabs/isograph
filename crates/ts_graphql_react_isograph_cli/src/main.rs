//! The isograph binary: TypeScript host, GraphQL protocol, React artifacts.

use std::collections::BTreeMap;
use std::process::ExitCode;

use common_lang_types::EntityName;

use graphql_network_protocol::GraphQLNetworkProtocol;
use isograph_compiler::CompilationProfile;
use isograph_extract_typescript::{TypeScriptCustomScalarType, TypeScriptHostLanguage};

struct TypeScriptGraphQL;

impl CompilationProfile for TypeScriptGraphQL {
    type HostLanguage = TypeScriptHostLanguage;
    type NetworkProtocol = GraphQLNetworkProtocol;

    fn custom_scalar_types() -> BTreeMap<EntityName, TypeScriptCustomScalarType> {
        BTreeMap::new()
    }
}

fn main() -> ExitCode {
    isograph_cli::run::<TypeScriptGraphQL>()
}
