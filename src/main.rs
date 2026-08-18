use std::collections::HashSet;

use crawfish::cli::arg_parser::Command;
use crawfish::cli::driver;

fn main() {
    match Command::parse() {
        Command::Build(args) => {
            driver::build(args.path, &args.emit.into_iter().collect::<HashSet<_>>());
        }
        Command::Run(args) => {
            driver::run(args.path, &args.emit.into_iter().collect::<HashSet<_>>());
        }
        Command::Check(args) => {
            driver::check(args.path, &args.emit.into_iter().collect::<HashSet<_>>());
        }
    }
}
