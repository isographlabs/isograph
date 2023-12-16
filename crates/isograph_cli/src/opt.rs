use std::path::PathBuf;

use structopt::StructOpt;

/// Options if we're doing a batch compilation
#[derive(Debug, StructOpt)]
pub(crate) struct CliOptions {
    #[structopt(long)]
    pub watch: bool,

    /// Compile using this config file. If not provided, searches for a config in
    /// package.json under the `isograph` key.
    #[structopt(long)]
    pub config: Option<PathBuf>,
}
