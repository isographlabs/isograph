pub mod batch_compile;
mod field_directives;
mod isograph_literals;
mod refetch_fields;
mod schema;
pub mod watch;
mod write_artifacts;

pub use watch::handle_watch_command;
pub use batch_compile::compile_and_print;
