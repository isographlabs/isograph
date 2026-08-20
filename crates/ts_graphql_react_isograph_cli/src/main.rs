//! The isograph binary: TypeScript host, GraphQL protocol, React artifacts.

use std::process::ExitCode;

fn main() -> ExitCode {
    isograph_cli::run()
}
