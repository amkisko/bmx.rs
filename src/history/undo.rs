use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::git_cmd::git_run;
use crate::install_flow::rebuild_worktree_from_saved_metadata;
use crate::io::read_toml;
use crate::layout::{app_layout_for_id, resolve_installed_layout};
use crate::types::InstallMetadata;

use super::{
    AuditEntry, UndoItem, append_audit, audit_unix_ts, new_action_id, read_undo_stack,
    write_undo_stack,
};

pub(crate) fn undo(home: &Path, id: Option<&str>, cli_verbose: bool, record: bool) -> Result<()> {
    let mut stack = read_undo_stack(home)?;
    if stack.is_empty() {
        bail!("nothing to undo");
    }
    let last_idx = stack.len() - 1;
    let idx = if let Some(want) = id {
        let pos = stack
            .iter()
            .rposition(|f| f.id == want)
            .ok_or_else(|| anyhow::anyhow!("unknown action id `{want}`"))?;
        if pos != last_idx {
            let top = &stack[last_idx];
            bail!(
                "action `{want}` is not the most recent undoable step; undo `{}` first",
                top.id
            );
        }
        pos
    } else {
        last_idx
    };

    let frame = stack.remove(idx);
    write_undo_stack(home, &stack)?;

    for item in frame.items.iter().rev() {
        match item {
            UndoItem::FreshInstall { app_id } => {
                let layout = app_layout_for_id(home, app_id.clone());
                let app_root = layout
                    .repo_dir
                    .parent()
                    .ok_or_else(|| anyhow::anyhow!("invalid layout for {}", app_id))?;
                if app_root.exists() {
                    fs::remove_dir_all(app_root)
                        .with_context(|| format!("failed to remove {}", app_root.display()))?;
                }
            }
            UndoItem::RestoreSnapshot {
                app_id,
                snapshot_dir,
            } => {
                apply_restore_snapshot(home, app_id, snapshot_dir, cli_verbose)?;
            }
        }
    }

    for item in &frame.items {
        if let UndoItem::RestoreSnapshot { snapshot_dir, .. } = item {
            let _ = fs::remove_dir_all(snapshot_dir);
        }
    }

    append_audit(
        home,
        &AuditEntry {
            id: new_action_id(),
            ts: audit_unix_ts(),
            kind: "undo".into(),
            summary: format!("undo {}", frame.id),
            app: None,
            source_url: None,
            undoable: false,
        },
        record,
    )?;

    println!("undone: {} ({})", frame.summary, frame.id);
    Ok(())
}

/// Roll back one app from the **latest** undo frame (for `bmx update` / multi-item frames).
pub(crate) fn undo_one_app(
    home: &Path,
    app_input: &str,
    cli_verbose: bool,
    record: bool,
) -> Result<()> {
    let mut stack = read_undo_stack(home)?;
    let Some(frame) = stack.last_mut() else {
        bail!("nothing to undo");
    };

    let resolved_id = resolve_installed_layout(home, app_input)
        .ok()
        .map(|l| l.id)
        .filter(|id| !id.is_empty());

    let pos = frame
        .items
        .iter()
        .position(|item| match item {
            UndoItem::FreshInstall { app_id } => {
                app_id == app_input || resolved_id.as_deref() == Some(app_id.as_str())
            }
            UndoItem::RestoreSnapshot { app_id, .. } => {
                app_id == app_input || resolved_id.as_deref() == Some(app_id.as_str())
            }
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no undo entry for `{app_input}` in the latest batch (see `bmx history`); each app’s id is in ~/.bmx/apps/<id>/"
            )
        })?;

    if matches!(frame.items.get(pos), Some(UndoItem::FreshInstall { .. })) {
        bail!(
            "partial undo is only for snapshot restores (e.g. after update); use `bmx undo` to remove a fresh install"
        );
    }

    let item = frame.items.remove(pos);
    let empty = frame.items.is_empty();

    if let UndoItem::RestoreSnapshot {
        app_id,
        snapshot_dir,
    } = &item
    {
        apply_restore_snapshot(home, app_id, snapshot_dir, cli_verbose)?;
        let _ = fs::remove_dir_all(snapshot_dir);
    }

    if empty {
        stack.pop();
    }
    write_undo_stack(home, &stack)?;

    append_audit(
        home,
        &AuditEntry {
            id: new_action_id(),
            ts: audit_unix_ts(),
            kind: "undo-partial".into(),
            summary: format!("undo one app from batch ({app_input})"),
            app: resolved_id.or_else(|| match &item {
                UndoItem::RestoreSnapshot { app_id, .. } => Some(app_id.clone()),
                UndoItem::FreshInstall { .. } => None,
            }),
            source_url: None,
            undoable: false,
        },
        record,
    )?;

    println!("undone one install: {app_input}");
    Ok(())
}

pub(crate) fn apply_restore_snapshot(
    home: &Path,
    app_id: &str,
    snapshot_dir: &Path,
    cli_verbose: bool,
) -> Result<()> {
    let snap_meta = snapshot_dir.join("install.toml");
    let meta: InstallMetadata = read_toml(&snap_meta)?;
    if meta.app != app_id {
        bail!(
            "snapshot app mismatch: expected {}, got {}",
            app_id,
            meta.app
        );
    }
    let layout = app_layout_for_id(home, app_id);
    fs::create_dir_all(&layout.repo_dir)?;
    let head_path = snapshot_dir.join("git_head.txt");
    let head = fs::read_to_string(&head_path).unwrap_or_default();
    let head = head.trim();
    fs::copy(&snap_meta, &layout.meta_file)
        .with_context(|| format!("restore install.toml from {}", snap_meta.display()))?;
    if layout.repo_dir.join(".git").exists() && !head.is_empty() {
        git_run(
            &layout.repo_dir,
            &["checkout", "--force", head],
            &[],
            !cli_verbose,
        )
        .with_context(|| format!("git checkout {head} in {}", layout.repo_dir.display()))?;
    }
    rebuild_worktree_from_saved_metadata(home, &meta, cli_verbose)?;
    Ok(())
}
