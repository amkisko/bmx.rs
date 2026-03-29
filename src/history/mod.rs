mod undo;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::git_cmd::git_output;
use crate::layout::app_layout_for_id;

#[cfg(test)]
pub(crate) use undo::apply_restore_snapshot;
pub(crate) use undo::{undo, undo_one_app};

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

pub(super) fn write_undo_stack(home: &Path, stack: &[UndoFrame]) -> Result<()> {
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

pub(crate) fn audit_unix_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;
