use std::path::Path;

use anyhow::{Result, bail};

use crate::cli::{
    CheckoutCommands, Cli, Commands, IsolationCommands, ShimCommands, SourceCommands,
};
use crate::config::{load_config, save_config};
use crate::ops::{
    doctor, install_app, run_app, self_update, uninstall_app, update_all,
};
use crate::source::normalize_source_base;
use crate::types::{BuildIsolation, CheckoutBackend};

pub(crate) fn dispatch(cli: Cli, home: &Path) -> Result<()> {
    match cli.command {
        Some(Commands::Exec { app, args }) => {
            let spec = if cli.pin {
                let cwd = std::env::current_dir()?;
                crate::pin::read_pin_from_ancestors(&cwd)?
            } else {
                app.ok_or_else(|| {
                    anyhow::anyhow!("`bmx exec` requires APP, or use --pin with a `.bmx/pin` file")
                })?
            };
            run_app(home, &spec, &args, cli.verbose)
        }
        Some(Commands::Install { app }) => {
            install_app(home, &app, cli.verbose)?;
            println!("installed {app}");
            Ok(())
        }
        Some(Commands::Uninstall { app }) => {
            uninstall_app(home, &app)?;
            println!("uninstalled {app}");
            Ok(())
        }
        Some(Commands::Reinstall { app }) => {
            uninstall_app(home, &app)?;
            install_app(home, &app, cli.verbose)?;
            println!("reinstalled {app}");
            Ok(())
        }
        Some(Commands::SelfUpdate { app }) => {
            self_update(home, &app, cli.verbose)?;
            println!("self-updated {app}");
            Ok(())
        }
        Some(Commands::Update { app }) => match app {
            Some(app_name) => {
                install_app(home, &app_name, cli.verbose)?;
                println!("updated {app_name}");
                Ok(())
            }
            None => {
                update_all(home, cli.verbose)?;
                println!("updated installed apps");
                Ok(())
            }
        },
        Some(Commands::Source { command }) => dispatch_source(home, command),
        Some(Commands::Isolation { command }) => dispatch_isolation(home, command),
        Some(Commands::Checkout { command }) => dispatch_checkout(home, command),
        Some(Commands::Shim { command }) => {
            let ph = crate::config::bmx_home()?;
            crate::config::ensure_base_dirs(&ph)?;
            match command {
                ShimCommands::Init => crate::shim::shim_init(&ph),
                ShimCommands::Path => {
                    crate::shim::shim_path_line(&ph);
                    Ok(())
                }
                ShimCommands::Add { app, name } => {
                    crate::shim::shim_add(&ph, &app, name.as_deref())
                }
            }
        }
        Some(Commands::Doctor) => doctor(home),
        None => {
            let cwd = std::env::current_dir()?;
            if cli.pin {
                let spec = crate::pin::read_pin_from_ancestors(&cwd)?;
                run_app(home, &spec, &cli.args, cli.verbose)
            } else if let Some(app) = cli.app {
                run_app(home, &app, &cli.args, cli.verbose)
            } else {
                bail!("provide a command, an app name (e.g. `bmx ripgrep`), or use --pin with `.bmx/pin`")
            }
        }
    }
}

fn dispatch_isolation(home: &Path, command: IsolationCommands) -> Result<()> {
    match command {
        IsolationCommands::SetDefault { mode } => {
            let mode = BuildIsolation::parse(&mode).ok_or_else(|| {
                anyhow::anyhow!(
                    "invalid isolation mode `{mode}`; expected one of: off, auto, docker, podman, nerdctl"
                )
            })?;
            let mut cfg = load_config(home)?;
            cfg.build_isolation = mode;
            save_config(home, &cfg)?;
            println!(
                "default build isolation set to {}",
                cfg.build_isolation.as_str()
            );
            Ok(())
        }
        IsolationCommands::Show => {
            let cfg = load_config(home)?;
            println!("{}", cfg.build_isolation.as_str());
            Ok(())
        }
    }
}

fn dispatch_source(home: &Path, command: SourceCommands) -> Result<()> {
    match command {
        SourceCommands::SetDefault { url } => {
            let mut cfg = load_config(home)?;
            cfg.default_source = Some(normalize_source_base(&url));
            save_config(home, &cfg)?;
            println!(
                "default source set to {}",
                cfg.default_source.unwrap_or_default()
            );
            Ok(())
        }
        SourceCommands::Show => {
            let cfg = load_config(home)?;
            println!(
                "{}",
                cfg.default_source
                    .unwrap_or_else(|| "(not set)".to_string())
            );
            Ok(())
        }
    }
}

fn dispatch_checkout(home: &Path, command: CheckoutCommands) -> Result<()> {
    match command {
        CheckoutCommands::SetDefault { backend } => {
            let backend = CheckoutBackend::parse(&backend).ok_or_else(|| {
                anyhow::anyhow!(
                    "invalid checkout backend `{backend}`; expected git, gh, or custom"
                )
            })?;
            let mut cfg = load_config(home)?;
            cfg.checkout_backend = backend;
            save_config(home, &cfg)?;
            println!(
                "default checkout backend set to {}",
                cfg.checkout_backend.as_str()
            );
            Ok(())
        }
        CheckoutCommands::Show => {
            let cfg = load_config(home)?;
            println!("{}", cfg.checkout_backend.as_str());
            Ok(())
        }
    }
}
