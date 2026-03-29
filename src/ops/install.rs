use std::fs;
use std::path::Path;

use anyhow::{Result, anyhow};

use crate::app_spec::{AppSpec, layout_id, parse_app_spec};
use crate::build::{build_with_strategy, detect_strategy};
use crate::config::load_config;
use crate::executable::detect_executable_rel;
use crate::history::{
    AuditEntry, UndoFrame, UndoItem, append_audit, audit_unix_ts, capture_install_snapshot,
    new_action_id, push_undo_frame,
};
use crate::hooks;
use crate::install_flow::reinstall_from_meta;
use crate::io::read_toml;
use crate::layout::{app_layout_for_id, sanitize_install_as};
use crate::repo::sync_repo;
use crate::revision::checkout_requested_ref;
use crate::runtime::subprocess_output_visible;
use crate::source::resolve_source;
use crate::trust::{
    enforce_source_trust, prompt_import_signing_keys_for_source, prompt_untrusted_source_consent,
};
use crate::types::{Config, InstallMetadata};

use super::TrustOptions;

pub(super) fn install_refers_to_existing(
    cfg: &Config,
    spec: &AppSpec,
    existing: &InstallMetadata,
) -> Result<bool> {
    if spec.source == existing.app {
        return Ok(true);
    }
    let resolved = resolve_source(cfg, &spec.source)?;
    Ok(resolved == existing.source_url)
}

pub(crate) fn install_app(
    home: &Path,
    app: &str,
    install_as: Option<&str>,
    cli_verbose: bool,
    record_history: bool,
    persist_requested_ref: bool,
    trust: TrustOptions,
) -> Result<()> {
    let spec = parse_app_spec(app);
    let cfg = load_config(home)?;
    let directory_id = if let Some(a) = install_as {
        sanitize_install_as(a)?
    } else {
        layout_id(&spec)
    };
    let layout = app_layout_for_id(home, directory_id);

    if layout.meta_file.exists() {
        let existing: InstallMetadata = read_toml(&layout.meta_file)?;
        let persisted_requested_ref = existing.requested_ref.clone();
        if let Some(a) = install_as {
            let want = sanitize_install_as(a)?;
            if existing.app != want {
                anyhow::bail!(
                    "this install is registered as `{}`, not `{}`",
                    existing.app,
                    want
                );
            }
        }
        if !install_refers_to_existing(&cfg, &spec, &existing)? {
            anyhow::bail!(
                "directory `{}` already holds an install from {}\n\
                 hint: `bmx uninstall {}` or use `bmx install {} --as <unique-name>` for the other source",
                layout.id,
                existing.source_url,
                layout.id,
                app
            );
        }
        let mut merged = existing;
        if spec.requested_ref.is_some() {
            merged.requested_ref = spec.requested_ref.clone();
        }
        if spec.cargo_package.is_some() {
            merged.rust_package = spec.cargo_package.clone();
        }
        let app_id = merged.app.clone();
        let source_url = merged.source_url.clone();
        let snapshot_dir = capture_install_snapshot(home, &app_id)?;
        reinstall_from_meta(home, merged, cli_verbose, trust.prompt, trust.global)?;
        if !persist_requested_ref {
            let mut refreshed: InstallMetadata = read_toml(&layout.meta_file)?;
            refreshed.requested_ref = persisted_requested_ref;
            crate::io::write_toml(&layout.meta_file, &refreshed)?;
        }
        let id = new_action_id();
        append_audit(
            home,
            &AuditEntry {
                id: id.clone(),
                ts: audit_unix_ts(),
                kind: "update".into(),
                summary: format!("update {app}"),
                app: Some(app_id.clone()),
                source_url: Some(source_url),
                undoable: true,
            },
            record_history,
        )?;
        push_undo_frame(
            home,
            UndoFrame {
                id,
                ts: audit_unix_ts(),
                summary: format!("update {app}"),
                items: vec![UndoItem::RestoreSnapshot {
                    app_id,
                    snapshot_dir,
                }],
            },
            record_history,
        )?;
        return Ok(());
    }

    fs::create_dir_all(&layout.repo_dir)?;

    let show_output = subprocess_output_visible(cli_verbose);
    let source_url = resolve_source(&cfg, &spec.source)?;
    sync_repo(&source_url, &layout.repo_dir, &cfg, cli_verbose)?;
    let resolved_commit =
        checkout_requested_ref(&layout.repo_dir, spec.requested_ref.as_deref(), cli_verbose)?;
    prompt_untrusted_source_consent(home, &source_url, &layout.repo_dir)?;
    if trust.prompt {
        prompt_import_signing_keys_for_source(home, &source_url, &layout.repo_dir, trust.global)?;
    }
    enforce_source_trust(home, &source_url, &layout.repo_dir)?;

    let strategy = detect_strategy(&layout.repo_dir).ok_or_else(|| {
        anyhow!(
            "unable to detect build strategy for {}",
            layout.repo_dir.display()
        )
    })?;
    build_with_strategy(
        &strategy,
        &layout.repo_dir,
        cfg.build_isolation,
        show_output,
        spec.cargo_package.as_deref(),
    )?;

    let executable_rel = detect_executable_rel(&layout.repo_dir, &spec)?;
    let app_id = layout.id.clone();
    let metadata = InstallMetadata {
        app: app_id.clone(),
        source_url,
        executable_rel,
        strategy: strategy.as_str().to_string(),
        requested_ref: if persist_requested_ref {
            spec.requested_ref
        } else {
            None
        },
        resolved_commit,
        rust_package: spec.cargo_package.clone(),
    };
    crate::io::write_toml(&layout.meta_file, &metadata)?;
    hooks::run_post_install_hook(&layout.repo_dir)?;
    let id = new_action_id();
    append_audit(
        home,
        &AuditEntry {
            id: id.clone(),
            ts: audit_unix_ts(),
            kind: "install".into(),
            summary: format!("install {app}"),
            app: Some(app_id.clone()),
            source_url: Some(metadata.source_url.clone()),
            undoable: true,
        },
        record_history,
    )?;
    push_undo_frame(
        home,
        UndoFrame {
            id,
            ts: audit_unix_ts(),
            summary: format!("install {app}"),
            items: vec![UndoItem::FreshInstall { app_id }],
        },
        record_history,
    )?;
    Ok(())
}
