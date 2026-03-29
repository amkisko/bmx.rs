use std::path::Path;

use anyhow::{Result, anyhow};

use crate::build::{build_with_strategy, detect_strategy};
use crate::config::load_config;
use crate::history::{
    AuditEntry, UndoFrame, UndoItem, append_audit, audit_unix_ts, capture_install_snapshot,
    new_action_id, push_undo_frame,
};
use crate::install_flow::reinstall_from_meta;
use crate::io::read_toml;
use crate::layout::resolve_installed_layout;
use crate::repo::sync_repo;
use crate::revision::checkout_requested_ref;
use crate::runtime::subprocess_output_visible;
use crate::trust::{
    enforce_source_trust, prompt_import_signing_keys_for_source, prompt_untrusted_source_consent,
};
use crate::types::InstallMetadata;

use crate::app_spec::parse_app_spec;

use super::TrustOptions;
use super::install::{install_app, install_refers_to_existing};

pub(crate) fn reinstall_app(
    home: &Path,
    app: &str,
    cli_verbose: bool,
    record_history: bool,
    trust_prompt: bool,
    trust_global: bool,
) -> Result<()> {
    let layout = resolve_installed_layout(home, app)?;
    let meta: InstallMetadata = read_toml(&layout.meta_file)?;
    let app_id = meta.app.clone();
    let source_url = meta.source_url.clone();
    let snapshot_dir = capture_install_snapshot(home, &app_id)?;
    reinstall_from_meta(home, meta, cli_verbose, trust_prompt, trust_global)?;
    let id = new_action_id();
    append_audit(
        home,
        &AuditEntry {
            id: id.clone(),
            ts: audit_unix_ts(),
            kind: "reinstall".into(),
            summary: format!("reinstall {app}"),
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
            summary: format!("reinstall {app}"),
            items: vec![UndoItem::RestoreSnapshot {
                app_id,
                snapshot_dir,
            }],
        },
        record_history,
    )?;
    Ok(())
}

pub(crate) fn rebuild_app(
    home: &Path,
    app: &str,
    install: bool,
    cli_verbose: bool,
    record_history: bool,
    persist_requested_ref: bool,
    trust: TrustOptions,
) -> Result<()> {
    if install {
        return install_app(
            home,
            app,
            None,
            cli_verbose,
            record_history,
            persist_requested_ref,
            trust,
        );
    }

    let cfg = load_config(home)?;
    let spec = parse_app_spec(app);
    let layout = resolve_installed_layout(home, app)?;
    let existing: InstallMetadata = read_toml(&layout.meta_file)?;
    if !install_refers_to_existing(&cfg, &spec, &existing)? {
        anyhow::bail!(
            "requested rebuild source does not match installed source for `{}`",
            existing.app
        );
    }

    let requested_ref = spec
        .requested_ref
        .clone()
        .or(existing.requested_ref.clone());
    let rust_package = spec.cargo_package.clone().or(existing.rust_package.clone());
    let source_url = existing.source_url.clone();

    sync_repo(&source_url, &layout.repo_dir, &cfg, cli_verbose)?;
    checkout_requested_ref(&layout.repo_dir, requested_ref.as_deref(), cli_verbose)?;
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
    let show_output = subprocess_output_visible(cli_verbose);
    build_with_strategy(
        &strategy,
        &layout.repo_dir,
        cfg.build_isolation,
        show_output,
        rust_package.as_deref(),
    )?;
    Ok(())
}
