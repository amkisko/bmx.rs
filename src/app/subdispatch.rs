use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, bail};

use crate::cli::{CheckoutCommands, IsolationCommands, SourceCommands, TrustCommands};
use crate::config::{load_config, save_config};
use crate::source::normalize_source_base;
use crate::types::{BuildIsolation, CheckoutBackend};

pub(super) fn dispatch_isolation(home: &Path, command: IsolationCommands) -> Result<()> {
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

pub(super) fn dispatch_source(home: &Path, command: SourceCommands) -> Result<()> {
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

pub(super) fn dispatch_checkout(home: &Path, command: CheckoutCommands) -> Result<()> {
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

pub(super) fn require_trust_for_global(trust_global: bool, trust: bool) -> Result<()> {
    if trust_global && !trust {
        bail!("`--global` requires `--trust`");
    }
    Ok(())
}

pub(super) fn dispatch_trust(
    home: &Path,
    command: TrustCommands,
    global_filter: bool,
) -> Result<()> {
    match command {
        TrustCommands::List { app, local } => {
            if local && global_filter {
                bail!("use either `--global` or `--local` with `bmx trust list`, not both");
            }
            let scope = if global_filter {
                crate::trust::TrustListScope::Global
            } else if local {
                crate::trust::TrustListScope::Local
            } else {
                crate::trust::TrustListScope::All
            };
            let out = crate::trust::list_policy(home, scope, app.as_deref())?;
            println!("{out}");
            Ok(())
        }
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
        TrustCommands::RemoveKey { key, match_prefix } => {
            crate::trust::remove_allowed_signing_key(home, &key, match_prefix.as_deref())?;
            println!(
                "removed signing key from {}",
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
            let (n, from) = import_signing_keys_from_any(home, &app, match_prefix.as_deref())?;
            if n == 0 {
                println!("no new signing key values to import from {from}");
            } else {
                println!("imported {n} signing key value(s) from {from}");
            }
            Ok(())
        }
        TrustCommands::Check { source } => {
            crate::trust_check::run_trust_check(home, source.as_deref())
        }
    }
}

fn import_signing_keys_from_any(
    home: &Path,
    app_or_source: &str,
    match_prefix: Option<&str>,
) -> Result<(usize, String)> {
    if let Ok(layout) = crate::layout::resolve_installed_layout(home, app_or_source) {
        let meta: crate::types::InstallMetadata = crate::io::read_toml(&layout.meta_file)?;
        let n = crate::trust::import_signing_keys_from_repo(
            home,
            &meta.source_url,
            &layout.repo_dir,
            match_prefix,
        )?;
        return Ok((n, layout.repo_dir.display().to_string()));
    }

    let cfg = load_config(home)?;
    let spec = crate::app_spec::parse_app_spec(app_or_source);
    let source_url = crate::source::resolve_source(&cfg, &spec.source)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_root = std::env::temp_dir().join(format!(
        "{}{}-{}",
        crate::workspace::TEMP_DIR_PREFIX_TRUST_IMPORT,
        std::process::id(),
        stamp
    ));
    let repo_dir: PathBuf = tmp_root.join("repo");
    std::fs::create_dir_all(&repo_dir)?;

    let result = (|| -> Result<usize> {
        crate::repo::sync_repo(&source_url, &repo_dir, &cfg, false)?;
        crate::revision::checkout_requested_ref(&repo_dir, spec.requested_ref.as_deref(), false)?;
        crate::trust::import_signing_keys_from_repo(home, &source_url, &repo_dir, match_prefix)
    })();
    let _ = std::fs::remove_dir_all(&tmp_root);

    result.map(|n| (n, format!("{source_url} (temporary checkout)")))
}
