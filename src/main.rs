mod cli_args;

use crate::cli_args::CliArgs;
use clap::Parser;

fn main() {
    CliArgs::parse().run();
}
