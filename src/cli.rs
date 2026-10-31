pub mod arg_parser;

use arg_parser::{Cli, Command};
use clap::Parser;

use crate::batch::driver;

pub fn main() {
    match Cli::parse().command {
        Command::Build(flags) => {
            driver::build(&flags);
        }
        Command::Run(flags) => {
            driver::run(&flags);
        }
        Command::Check(flags) => {
            driver::check(&flags);
        }
    }
}
