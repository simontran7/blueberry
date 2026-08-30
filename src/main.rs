use std::collections::HashSet;

use blueberry::cli::arg_parser::Command;
use blueberry::cli::driver;

fn main() {
    match Command::parse() {
        Command::Build(flags) => {
            driver::build(flags.path, &flags.emit.into_iter().collect::<HashSet<_>>());
        }
        Command::Run(flags) => {
            driver::run(flags.path, &flags.emit.into_iter().collect::<HashSet<_>>());
        }
        Command::Check(flags) => {
            driver::check(flags.path, &flags.emit.into_iter().collect::<HashSet<_>>());
        }
    }
}
