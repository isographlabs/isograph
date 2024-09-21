use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct Opt {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[command(flatten)]
    pub compile: CompileCommand,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Compile(CompileCommand),
    Lsp(LspCommand),
}

/// Compile
#[derive(Debug, Args)]
pub(crate) struct CompileCommand {
    #[arg(long)]
    pub watch: bool,

    /// Compile using this config file. If not provided, searches for a config in
    /// package.json under the `isograph` key.
    #[arg(long)]
    pub config: Option<PathBuf>,
}

/// LSP
#[derive(Debug, Args)]
pub(crate) struct LspCommand {
    /// Compile using this config file. If not provided, searches for a config in
    /// package.json under the `isograph` key.
    #[arg(long)]
    pub config: Option<PathBuf>,
}
