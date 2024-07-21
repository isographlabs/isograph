use std::path::PathBuf;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub enum CliOptions {
    Compile(CompilerCLIOptions),
    LSP(LSPCLIOptions),
}
/// Compile
#[derive(Debug, StructOpt)]
pub(crate) struct CompilerCLIOptions {
    #[structopt(long)]
    pub watch: bool,

    /// Compile using this config file. If not provided, searches for a config in
    /// package.json under the `isograph` key.
    #[structopt(long)]
    pub config: Option<PathBuf>,
}

/// LSP
#[derive(Debug, StructOpt)]
pub(crate) struct LSPCLIOptions {
    /// Compile using this config file. If not provided, searches for a config in
    /// package.json under the `isograph` key.
    #[structopt(long)]
    pub config: Option<PathBuf>,
}
