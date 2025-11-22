pub mod batch_compile;
mod compiler_state;
mod read_files;
mod source_files;
pub mod watch;
mod with_duration;
mod write_artifacts;

pub use batch_compile::create_state_and_compile_once;
pub use compiler_state::*;
pub use source_files::*;
pub use watch::handle_watch_command;
pub use with_duration::*;
