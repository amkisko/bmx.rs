mod app;
mod app_spec;
mod build;
mod cli;
mod cli_error;
mod cli_suggest;
mod completions;
mod config;
mod executable;
mod exit;
mod git_cmd;
mod history;
mod hooks;
mod http_client;
mod input_mode;
mod install_flow;
mod integrity;
mod io;
mod isolation;
mod layout;
mod man;
mod ops;
mod pin;
mod process;
mod repo;
mod revision;
mod runtime;
mod search;
mod shim;
mod source;
mod trust;
mod trust_check;
mod trust_feed;
mod types;
mod workspace;

use std::process::ExitCode;

use clap::Parser;

use crate::app::dispatch;
use crate::cli::{Cli, Commands};
use crate::cli_error::{ErrorContext, print_error};
use crate::config::{bmx_home, ensure_base_dirs};
use crate::exit::AppExit;

pub fn run() -> ExitCode {
    let cli = Cli::parse();
    input_mode::set_no_input(cli.no_input);
    let error_context = ErrorContext {
        verbose: cli.verbose,
    };

    if let Some(Commands::Completions { shell }) = &cli.command {
        return match completions::run(*shell) {
            Ok(()) => AppExit::Success.into(),
            Err(message) => print_error(&anyhow::anyhow!(message), &error_context).into(),
        };
    }
    if let Some(Commands::Man) = &cli.command {
        return match man::run() {
            Ok(()) => AppExit::Success.into(),
            Err(message) => print_error(&anyhow::anyhow!(message), &error_context).into(),
        };
    }

    let persistent_home = match bmx_home() {
        Ok(home) => home,
        Err(error) => return print_error(&error, &error_context).into(),
    };
    let ephemeral = if cli.rm {
        match workspace::ephemeral_bmx_home(&persistent_home) {
            Ok(dir) => Some(dir),
            Err(error) => return print_error(&error, &error_context).into(),
        }
    } else {
        None
    };
    let home = ephemeral.as_deref().unwrap_or(&persistent_home);
    if !cli.rm
        && let Err(error) = ensure_base_dirs(home)
    {
        return print_error(&error, &error_context).into();
    }

    let result = dispatch(cli, home);
    if let Some(dir) = ephemeral {
        workspace::remove_dir_all_best_effort(&dir);
    }
    match result {
        Ok(()) => AppExit::Success.into(),
        Err(error) => print_error(&error, &error_context).into(),
    }
}

#[cfg(test)]
mod tests_build_exec;
#[cfg(test)]
mod tests_core;
#[cfg(test)]
mod tests_repo;
