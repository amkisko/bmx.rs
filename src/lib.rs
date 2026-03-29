mod app;
mod app_spec;
mod build;
mod cli;
mod config;
mod executable;
mod git_cmd;
mod history;
mod hooks;
mod install_flow;
mod integrity;
mod io;
mod isolation;
mod layout;
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
mod types;
mod workspace;

use anyhow::Result;
use clap::Parser;

use crate::app::dispatch;
use crate::cli::Cli;
use crate::config::{bmx_home, ensure_base_dirs};

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let persistent_home = bmx_home()?;
    let ephemeral = if cli.rm {
        Some(workspace::ephemeral_bmx_home()?)
    } else {
        None
    };
    let home = ephemeral.as_deref().unwrap_or(&persistent_home);
    if !cli.rm {
        ensure_base_dirs(home)?;
    }

    let result = dispatch(cli, home);
    if let Some(dir) = ephemeral {
        workspace::remove_dir_all_best_effort(&dir);
    }
    result
}

#[cfg(test)]
mod tests_build_exec;
#[cfg(test)]
mod tests_core;
#[cfg(test)]
mod tests_repo;
