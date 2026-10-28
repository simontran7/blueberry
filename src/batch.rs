pub mod arg_parser;
pub mod diagnostic_render;
pub mod driver;

use arg_parser::Command;

pub fn main() {
    match Command::parse() {
        Command::Build(flags) => {
            driver::build(&flags.path, &flags.emit);
        }
        Command::Run(flags) => {
            driver::run(&flags.path, &flags.emit);
        }
        Command::Check(flags) => {
            driver::check(&flags.path, &flags.emit);
        }
    }
}
