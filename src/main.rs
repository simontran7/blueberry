use std::collections::HashSet;

use crawfish::cli::arg_parser::Command;
use crawfish::cli::driver;

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
