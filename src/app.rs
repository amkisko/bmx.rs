use std::path::Path;

use anyhow::{Result, bail};

use crate::cli::{
    CheckoutCommands, Cli, Commands, IsolationCommands, ShimCommands, SourceCommands, TrustCommands,
};
use crate::config::{load_config, save_config};
use crate::history::{list_recent_audit, undo, undo_one_app};
use crate::layout::resolve_installed_layout;
use crate::ops::{
    doctor, install_app, rebuild_app, reinstall_app, run_app, self_update, show_package,
    uninstall_app, update_all,
};
use crate::source::normalize_source_base;
use crate::types::{BuildIsolation, CheckoutBackend};

pub(crate) fn dispatch(cli: Cli, home: &Path) -> Result<()> {
    let record = !cli.rm;
    match cli.command {
        Some(Commands::Exec { app, args }) => {
            let cwd = std::env::current_dir()?;
            let spec = if cli.pin {
                match app.as_deref() {
                    Some(a) => {
                        crate::pin::upsert_pin_in_dir(&cwd, a)?;
                        a.to_string()
                    }
                    None => crate::pin::resolve_implicit_pin_spec(&cwd)?,
                }
            } else {
                app.ok_or_else(|| {
                    anyhow::anyhow!(
                        "`bmx exec` requires APP, or use --pin with `.bmx/pins.toml` / `.bmx/pin`"
                    )
                })?
            };
            run_app(home, &spec, &args, cli.verbose, cli.pin)
        }
        Some(Commands::Install { app, install_as }) => {
            install_app(
                home,
                &app,
                install_as.as_deref(),
                cli.verbose,
                record,
                cli.pin,
                cli.trust,
            )?;
            println!("installed {app}");
            Ok(())
        }
        Some(Commands::Rebuild { install, app }) => {
            let cwd = std::env::current_dir()?;
            let spec = if cli.pin {
                match app.as_deref() {
                    Some(a) => {
                        crate::pin::upsert_pin_in_dir(&cwd, a)?;
                        a.to_string()
                    }
                    None => crate::pin::resolve_implicit_pin_spec(&cwd)?,
                }
            } else {
                app.ok_or_else(|| {
                    anyhow::anyhow!(
                        "`bmx rebuild` requires APP, or use --pin with `.bmx/pins.toml` / `.bmx/pin`"
                    )
                })?
            };
            rebuild_app(
                home,
                &spec,
                install,
                cli.verbose,
                record,
                cli.pin,
                cli.trust,
            )?;
            if install {
                println!("rebuilt and installed {spec}");
            } else {
                println!("rebuilt {spec}");
            }
            Ok(())
        }
        Some(Commands::Uninstall { app }) => {
            uninstall_app(home, &app, record)?;
            println!("uninstalled {app}");
            Ok(())
        }
        Some(Commands::Reinstall { app }) => {
            reinstall_app(home, &app, cli.verbose, record, cli.trust)?;
            println!("reinstalled {app}");
            Ok(())
        }
        Some(Commands::SelfUpdate { app }) => {
            self_update(home, &app, cli.verbose, record, cli.trust)?;
            println!("self-updated {app}");
            Ok(())
        }
        Some(Commands::Update { app }) => match app {
            Some(app_name) => {
                resolve_installed_layout(home, &app_name)?;
                install_app(
                    home,
                    &app_name,
                    None,
                    cli.verbose,
                    record,
                    cli.pin,
                    cli.trust,
                )?;
                println!("updated {app_name}");
                Ok(())
            }
            None => {
                update_all(home, cli.verbose, record, cli.trust)?;
                println!("updated installed apps");
                Ok(())
            }
        },
        Some(Commands::Source { command }) => dispatch_source(home, command),
        Some(Commands::Isolation { command }) => dispatch_isolation(home, command),
        Some(Commands::Checkout { command }) => dispatch_checkout(home, command),
        Some(Commands::Trust { command }) => dispatch_trust(home, command),
        Some(Commands::Shim { command }) => {
            crate::config::ensure_base_dirs(home)?;
            match command {
                ShimCommands::Init => crate::shim::shim_init(home),
                ShimCommands::Path => {
                    crate::shim::shim_path_line(home);
                    Ok(())
                }
                ShimCommands::Add { app, name } => {
                    crate::shim::shim_add(home, &app, name.as_deref())
                }
            }
        }
        Some(Commands::Show { app, would_remove }) => show_package(home, &app, would_remove),
        Some(Commands::History { limit }) => {
            for e in list_recent_audit(home, limit)? {
                println!(
                    "{}  {}  {}  undoable={}  {}  {}",
                    e.id,
                    e.ts,
                    e.kind,
                    e.undoable,
                    e.summary,
                    e.source_url.as_deref().unwrap_or("-"),
                );
            }
            Ok(())
        }
        Some(Commands::Undo { id, only_app }) => {
            if id.is_some() && only_app.is_some() {
                bail!("use either `bmx undo [id]` or `bmx undo --only <app>`, not both");
            }
            if let Some(app) = only_app {
                undo_one_app(home, &app, cli.verbose, record)
            } else {
                undo(home, id.as_deref(), cli.verbose, record)
            }
        }
        Some(Commands::Doctor) => doctor(home),
        Some(Commands::Search {
            query,
            backend,
            limit,
            forks,
            url,
            no_probe,
        }) => crate::search::run_search(home, &query, backend, limit, forks, url, !no_probe),
        None => {
            let cwd = std::env::current_dir()?;
            if cli.pin {
                match cli.app.as_deref() {
                    Some(a) => {
                        crate::pin::upsert_pin_in_dir(&cwd, a)?;
                        run_app(home, a, &cli.args, cli.verbose, cli.pin)
                    }
                    None => {
                        let spec = crate::pin::resolve_implicit_pin_spec(&cwd)?;
                        run_app(home, &spec, &cli.args, cli.verbose, cli.pin)
                    }
                }
            } else if let Some(app) = cli.app {
                run_app(home, &app, &cli.args, cli.verbose, cli.pin)
            } else {
                bail!(
                    "provide a command, an app name (e.g. `bmx ripgrep`), or use --pin with `.bmx/pins.toml` / `.bmx/pin`"
                )
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
                anyhow::anyhow!("invalid checkout backend `{backend}`; expected git, gh, or custom")
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

fn dispatch_trust(home: &Path, command: TrustCommands) -> Result<()> {
    match command {
        TrustCommands::Show => {
            let policy = crate::trust::show_policy_toml(home)?;
            println!("{policy}");
            Ok(())
        }
        TrustCommands::AddKey { key, match_prefix } => {
            crate::trust::add_allowed_signing_key(home, &key, match_prefix.as_deref())?;
            println!(
                "added signing key for {}",
                match_prefix.as_deref().unwrap_or("<default>")
            );
            Ok(())
        }
        TrustCommands::SetSigned {
            match_prefix,
            enabled,
        } => {
            crate::trust::set_require_signed_commit(home, &match_prefix, enabled)?;
            println!("set require_signed_commit={enabled} for {match_prefix}");
            Ok(())
        }
        TrustCommands::SetAllow {
            match_prefix,
            allow,
        } => {
            crate::trust::set_allow(home, &match_prefix, allow)?;
            println!("set allow={allow} for {match_prefix}");
            Ok(())
        }
        TrustCommands::ImportRepo { app, match_prefix } => {
            let layout = crate::layout::resolve_installed_layout(home, &app)?;
            let n = crate::trust::import_signing_keys_from_repo(
                home,
                &layout.repo_dir,
                match_prefix.as_deref(),
            )?;
            println!(
                "imported {n} signing key value(s) from {}",
                layout.repo_dir.display()
            );
            Ok(())
        }
    }
}
