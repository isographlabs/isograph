use std::path::PathBuf;
use clap::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Opt {
    #[structopt(subcommand)]
    pub command: Option<Command>,

    #[structopt(flatten)]
    pub compile: CompileCommand,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    Compile(CompileCommand),
    Lsp(LspCommand),
}
/// Compile
#[derive(Debug, StructOpt)]
pub(crate) struct CompileCommand {
    #[structopt(long)]
    pub watch: bool,

    /// Compile using this config file. If not provided, searches for a config in
    /// package.json under the `isograph` key.
    #[structopt(long)]
    pub config: Option<PathBuf>,
}

/// LSP
#[derive(Debug, StructOpt)]
pub(crate) struct LspCommand {
    /// Compile using this config file. If not provided, searches for a config in
    /// package.json under the `isograph` key.
    #[structopt(long)]
    pub config: Option<PathBuf>,
}
