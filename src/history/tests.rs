use crate::io::write_toml;
use tempfile::tempdir;

use super::{
    AuditEntry, UndoFrame, UndoItem, append_audit, apply_restore_snapshot,
    capture_install_snapshot, list_recent_audit, push_undo_frame, read_undo_stack, undo,
    undo_one_app,
};
use crate::layout::app_layout_for_id;
use crate::types::InstallMetadata;

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
    std::fs::create_dir_all(&layout.repo_dir).unwrap();
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
    std::fs::create_dir_all(&snap).unwrap();
    write_toml(
        &snap.join("install.toml"),
        &sample_meta("other-app", "https://example.com/repo.git"),
    )
    .unwrap();
    std::fs::write(snap.join("git_head.txt"), "").unwrap();
    assert!(apply_restore_snapshot(home.path(), "wanted-app", &snap, false).is_err());
}
