use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::git_cmd::{git_output, git_run};
use crate::install_flow::rebuild_worktree_from_saved_metadata;
use crate::io::read_toml;
use crate::layout::{app_layout_for_id, resolve_installed_layout};
use crate::types::InstallMetadata;

const MAX_UNDO_FRAMES: usize = 64;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct AuditEntry {
    pub(crate) id: String,
    pub(crate) ts: i64,
    pub(crate) kind: String,
    pub(crate) summary: String,
    #[serde(default)]
    pub(crate) app: Option<String>,
    /// Resolved git remote URL stored in `install.toml` (`source_url`) when applicable.
    #[serde(default)]
    pub(crate) source_url: Option<String>,
    #[serde(default)]
    pub(crate) undoable: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct UndoFrame {
    pub(crate) id: String,
    pub(crate) ts: i64,
    pub(crate) summary: String,
    pub(crate) items: Vec<UndoItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum UndoItem {
    FreshInstall {
        app_id: String,
    },
    RestoreSnapshot {
        app_id: String,
        snapshot_dir: PathBuf,
    },
}

pub(crate) fn new_action_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}-{}", std::process::id())
}

fn audit_path(home: &Path) -> PathBuf {
    home.join("audit.log.jsonl")
}

fn undo_stack_path(home: &Path) -> PathBuf {
    home.join("undo-stack.json")
}

pub(crate) fn append_audit(home: &Path, entry: &AuditEntry, record: bool) -> Result<()> {
    if !record {
        return Ok(());
    }
    fs::create_dir_all(home)?;
    let line = serde_json::to_string(entry).context("serialize audit entry")?;
    let path = audit_path(home);
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    writeln!(f, "{line}")?;
    Ok(())
}

pub(crate) fn read_undo_stack(home: &Path) -> Result<Vec<UndoFrame>> {
    let path = undo_stack_path(home);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let stack: Vec<UndoFrame> =
        serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
    Ok(stack)
}

fn write_undo_stack(home: &Path, stack: &[UndoFrame]) -> Result<()> {
    let path = undo_stack_path(home);
    let raw = serde_json::to_string_pretty(stack).context("serialize undo stack")?;
    fs::write(&path, raw).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn drop_snapshots_from_frame(frame: &UndoFrame) {
    for item in &frame.items {
        if let UndoItem::RestoreSnapshot { snapshot_dir, .. } = item {
            let _ = fs::remove_dir_all(snapshot_dir);
        }
    }
}

pub(crate) fn push_undo_frame(home: &Path, frame: UndoFrame, record: bool) -> Result<()> {
    if !record {
        return Ok(());
    }
    let mut stack = read_undo_stack(home)?;
    stack.push(frame);
    while stack.len() > MAX_UNDO_FRAMES {
        let dropped = stack.remove(0);
        drop_snapshots_from_frame(&dropped);
    }
    write_undo_stack(home, &stack)?;
    Ok(())
}

/// Capture `install.toml` and current `HEAD` for rollback after sync/build.
pub(crate) fn capture_install_snapshot(home: &Path, app_id: &str) -> Result<PathBuf> {
    let layout = app_layout_for_id(home, app_id);
    if !layout.meta_file.exists() {
        bail!("no install metadata to snapshot for {}", app_id);
    }
    let snap_root = home.join("snapshots");
    fs::create_dir_all(&snap_root)?;
    let dir_name = format!("{}-{}", new_action_id(), app_id.replace('/', "__"));
    let dir = snap_root.join(dir_name);
    fs::create_dir_all(&dir)?;
    fs::copy(&layout.meta_file, dir.join("install.toml")).with_context(|| {
        format!(
            "snapshot copy {} -> {}",
            layout.meta_file.display(),
            dir.display()
        )
    })?;
    let head = if layout.repo_dir.join(".git").exists() {
        git_output(&layout.repo_dir, &["rev-parse", "HEAD"], &[]).unwrap_or_default()
    } else {
        String::new()
    };
    fs::write(dir.join("git_head.txt"), head.trim_end())?;
    Ok(dir)
}

pub(crate) fn list_recent_audit(home: &Path, limit: usize) -> Result<Vec<AuditEntry>> {
    let path = audit_path(home);
    if !path.exists() || limit == 0 {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let mut lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
    let take = lines.len().saturating_sub(limit);
    lines.drain(..take);
    let mut out = Vec::new();
    for line in lines {
        if let Ok(e) = serde_json::from_str::<AuditEntry>(line) {
            out.push(e);
        }
    }
    Ok(out)
}

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

pub(crate) fn audit_unix_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn apply_restore_snapshot(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::write_toml;
    use tempfile::tempdir;

    fn sample_meta(app: &str, source_url: &str) -> InstallMetadata {
        InstallMetadata {
            app: app.to_string(),
            source_url: source_url.to_string(),
            executable_rel: "bin/tool".to_string(),
            strategy: "make".to_string(),
            requested_ref: None,
            resolved_commit: None,
            rust_package: None,
        }
    }

    #[test]
    fn append_and_list_audit_handles_limits() {
        let home = tempdir().unwrap();
        append_audit(
            home.path(),
            &AuditEntry {
                id: "1".into(),
                ts: 1,
                kind: "k".into(),
                summary: "s1".into(),
                app: None,
                source_url: None,
                undoable: false,
            },
            true,
        )
        .unwrap();
        append_audit(
            home.path(),
            &AuditEntry {
                id: "2".into(),
                ts: 2,
                kind: "k".into(),
                summary: "s2".into(),
                app: None,
                source_url: None,
                undoable: true,
            },
            true,
        )
        .unwrap();
        let one = list_recent_audit(home.path(), 1).unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].id, "2");
    }

    #[test]
    fn push_and_read_undo_stack_roundtrip() {
        let home = tempdir().unwrap();
        let frame = UndoFrame {
            id: "x".into(),
            ts: 1,
            summary: "undo".into(),
            items: vec![UndoItem::FreshInstall { app_id: "a".into() }],
        };
        push_undo_frame(home.path(), frame, true).unwrap();
        let stack = read_undo_stack(home.path()).unwrap();
        assert_eq!(stack.len(), 1);
        assert_eq!(stack[0].id, "x");
    }

    #[test]
    fn capture_snapshot_requires_existing_metadata() {
        let home = tempdir().unwrap();
        assert!(capture_install_snapshot(home.path(), "missing").is_err());
    }

    #[test]
    fn capture_snapshot_copies_meta_and_head_file() {
        let home = tempdir().unwrap();
        let layout = app_layout_for_id(home.path(), "snap-app");
        fs::create_dir_all(&layout.repo_dir).unwrap();
        write_toml(
            &layout.meta_file,
            &sample_meta("snap-app", "https://example.com/repo.git"),
        )
        .unwrap();
        let snap = capture_install_snapshot(home.path(), "snap-app").unwrap();
        assert!(snap.join("install.toml").exists());
        assert!(snap.join("git_head.txt").exists());
    }

    #[test]
    fn undo_errors_when_empty() {
        let home = tempdir().unwrap();
        assert!(undo(home.path(), None, false, true).is_err());
    }

    #[test]
    fn undo_one_app_errors_when_empty() {
        let home = tempdir().unwrap();
        assert!(undo_one_app(home.path(), "x", false, true).is_err());
    }

    #[test]
    fn apply_restore_snapshot_rejects_mismatched_app() {
        let home = tempdir().unwrap();
        let snap = home.path().join("snap");
        fs::create_dir_all(&snap).unwrap();
        write_toml(
            &snap.join("install.toml"),
            &sample_meta("other-app", "https://example.com/repo.git"),
        )
        .unwrap();
        fs::write(snap.join("git_head.txt"), "").unwrap();
        assert!(apply_restore_snapshot(home.path(), "wanted-app", &snap, false).is_err());
    }
}
