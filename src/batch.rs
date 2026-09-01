pub mod arg_parser;
pub mod driver;
pub mod llvm_codegen;

use std::collections::HashSet;

use arg_parser::Command;

pub fn main() {
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
