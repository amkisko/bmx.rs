use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::history::{
    AuditEntry, UndoFrame, UndoItem, append_audit, audit_unix_ts, capture_install_snapshot,
    new_action_id, push_undo_frame,
};
use crate::install_flow::reinstall_from_meta;
use crate::io::read_toml;
use crate::types::InstallMetadata;

pub(crate) fn update_all(
    home: &Path,
    cli_verbose: bool,
    record_history: bool,
    trust_prompt: bool,
    trust_global: bool,
) -> Result<()> {
    let apps_root = home.join("apps");
    if !apps_root.exists() {
        return Ok(());
    }

    let mut plan: Vec<(InstallMetadata, PathBuf)> = Vec::new();
    for entry in fs::read_dir(&apps_root)? {
        let meta_path = entry?.path().join("install.toml");
        if !meta_path.exists() {
            continue;
        }

        let install: InstallMetadata = read_toml(&meta_path)?;
        let snapshot_dir = capture_install_snapshot(home, &install.app)?;
        plan.push((install, snapshot_dir));
    }
    plan.sort_by(|a, b| a.0.app.cmp(&b.0.app));

    for (install, _) in &plan {
        reinstall_from_meta(
            home,
            install.clone(),
            cli_verbose,
            trust_prompt,
            trust_global,
        )?;
    }

    if plan.is_empty() {
        return Ok(());
    }

    let n = plan.len();
    let items: Vec<UndoItem> = plan
        .into_iter()
        .map(|(i, snap)| UndoItem::RestoreSnapshot {
            app_id: i.app,
            snapshot_dir: snap,
        })
        .collect();
    let id = new_action_id();
    append_audit(
        home,
        &AuditEntry {
            id: id.clone(),
            ts: audit_unix_ts(),
            kind: "update-all".into(),
            summary: format!("update all ({n} apps)"),
            app: None,
            source_url: None,
            undoable: true,
        },
        record_history,
    )?;
    push_undo_frame(
        home,
        UndoFrame {
            id,
            ts: audit_unix_ts(),
            summary: format!("update all ({n} apps)"),
            items,
        },
        record_history,
    )?;
    Ok(())
}
