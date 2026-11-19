//! The isograph binary: TypeScript host, GraphQL protocol, React artifacts.

use std::process::ExitCode;

use graphql_network_protocol::GraphQLNetworkProtocol;
use isograph_compiler::CompilationProfile;
use isograph_extract_typescript::TypeScriptHostLanguage;

struct TypeScriptGraphQL;

impl CompilationProfile for TypeScriptGraphQL {
    type HostLanguage = TypeScriptHostLanguage;
    type NetworkProtocol = GraphQLNetworkProtocol;
}

fn main() -> ExitCode {
    isograph_cli::run::<TypeScriptGraphQL>()
}
