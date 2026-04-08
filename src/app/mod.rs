mod subdispatch;

use std::path::Path;

use anyhow::{Result, bail};

use crate::cli::{Cli, Commands, ShimCommands};
use crate::history::{list_recent_audit, undo, undo_one_app};
use crate::layout::resolve_installed_layout;
use crate::ops::{
    TrustOptions, clean_temp_artifacts, doctor, install_app, rebuild_app, reinstall_app, run_app,
    self_update, show_package, uninstall_app, update_all,
};

use subdispatch::{
    dispatch_checkout, dispatch_isolation, dispatch_source, dispatch_trust,
    require_trust_for_global,
};

pub(crate) fn dispatch(cli: Cli, home: &Path) -> Result<()> {
    let record = !cli.rm;
    let run_isolation_override = if cli.isolate_run {
        Some(crate::types::BuildIsolation::Auto)
    } else if cli.no_isolate_run {
        Some(crate::types::BuildIsolation::Off)
    } else {
        None
    };
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
            run_app(
                home,
                &spec,
                &args,
                cli.verbose,
                cli.pin,
                run_isolation_override,
            )
        }
        Some(Commands::Install { app, install_as }) => {
            require_trust_for_global(cli.trust_global, cli.trust)?;
            install_app(
                home,
                &app,
                install_as.as_deref(),
                cli.verbose,
                record,
                cli.pin,
                TrustOptions {
                    prompt: cli.trust,
                    global: cli.trust_global,
                },
            )?;
            println!("installed {app}");
            Ok(())
        }
        Some(Commands::Rebuild { install, app }) => {
            require_trust_for_global(cli.trust_global, cli.trust)?;
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
                TrustOptions {
                    prompt: cli.trust,
                    global: cli.trust_global,
                },
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
            require_trust_for_global(cli.trust_global, cli.trust)?;
            reinstall_app(home, &app, cli.verbose, record, cli.trust, cli.trust_global)?;
            println!("reinstalled {app}");
            Ok(())
        }
        Some(Commands::SelfUpdate) => {
            require_trust_for_global(cli.trust_global, cli.trust)?;
            self_update(home, cli.verbose, record, cli.trust, cli.trust_global)?;
            println!("self-updated bmx");
            Ok(())
        }
        Some(Commands::Update { app }) => match app {
            Some(app_name) => {
                require_trust_for_global(cli.trust_global, cli.trust)?;
                resolve_installed_layout(home, &app_name)?;
                install_app(
                    home,
                    &app_name,
                    None,
                    cli.verbose,
                    record,
                    cli.pin,
                    TrustOptions {
                        prompt: cli.trust,
                        global: cli.trust_global,
                    },
                )?;
                println!("updated {app_name}");
                Ok(())
            }
            None => {
                require_trust_for_global(cli.trust_global, cli.trust)?;
                update_all(home, cli.verbose, record, cli.trust, cli.trust_global)?;
                println!("updated installed apps");
                Ok(())
            }
        },
        Some(Commands::Source { command }) => dispatch_source(home, command),
        Some(Commands::Isolation { command }) => dispatch_isolation(home, command),
        Some(Commands::Checkout { command }) => dispatch_checkout(home, command),
        Some(Commands::Trust { command }) => dispatch_trust(home, command, cli.trust_global),
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
        Some(Commands::Clean { dry_run }) => clean_temp_artifacts(dry_run, cli.verbose),
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
                        run_app(
                            home,
                            a,
                            &cli.args,
                            cli.verbose,
                            cli.pin,
                            run_isolation_override,
                        )
                    }
                    None => {
                        let spec = crate::pin::resolve_implicit_pin_spec(&cwd)?;
                        run_app(
                            home,
                            &spec,
                            &cli.args,
                            cli.verbose,
                            cli.pin,
                            run_isolation_override,
                        )
                    }
                }
            } else if let Some(app) = cli.app {
                run_app(
                    home,
                    &app,
                    &cli.args,
                    cli.verbose,
                    cli.pin,
                    run_isolation_override,
                )
            } else {
                bail!(
                    "provide a command, an app name (e.g. `bmx ripgrep`), or use --pin with `.bmx/pins.toml` / `.bmx/pin`"
                )
            }
        }
    }
}
