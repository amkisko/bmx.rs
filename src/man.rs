//! Man page generation.

use clap::CommandFactory;
use clap_mangen::Man;
use std::io;

use crate::cli::Cli;

pub fn run() -> Result<(), String> {
    let command = Cli::command();
    Man::new(command)
        .render(&mut io::stdout())
        .map_err(|error| error.to_string())
}
