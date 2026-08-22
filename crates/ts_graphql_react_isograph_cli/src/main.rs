//! The isograph binary: TypeScript host, GraphQL protocol, React artifacts.

use std::process::ExitCode;

use isograph_extract_typescript::TypeScriptHostLanguage;

fn main() -> ExitCode {
    isograph_cli::run::<TypeScriptHostLanguage>()
}
