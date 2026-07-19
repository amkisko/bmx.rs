//! Shell completion generation.

use clap::{CommandFactory, ValueEnum};
use clap_complete::{Shell, generate};
use std::io;

use crate::cli::Cli;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CompletionShell {
    Bash,
    Elvish,
    Fish,
    PowerShell,
    Zsh,
}

impl From<CompletionShell> for Shell {
    fn from(value: CompletionShell) -> Self {
        match value {
            CompletionShell::Bash => Shell::Bash,
            CompletionShell::Elvish => Shell::Elvish,
            CompletionShell::Fish => Shell::Fish,
            CompletionShell::PowerShell => Shell::PowerShell,
            CompletionShell::Zsh => Shell::Zsh,
        }
    }
}

pub fn run(shell: CompletionShell) -> Result<(), String> {
    let mut command = Cli::command();
    let shell: Shell = shell.into();
    generate(shell, &mut command, "bmx", &mut io::stdout());
    Ok(())
}
