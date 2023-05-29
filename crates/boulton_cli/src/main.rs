mod batch_compile;
mod boulton_literals;
mod print;
mod schema;

use batch_compile::{handle_compile_command, BatchCompileCliOptions};
use structopt::StructOpt;

fn main() {
    let opt = BatchCompileCliOptions::from_args();
    let result = handle_compile_command(opt);

    match result {
        Ok(_) => eprintln!("Done."),
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    }
}
