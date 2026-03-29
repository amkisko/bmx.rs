use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};

use crate::history::{AuditEntry, append_audit, audit_unix_ts, new_action_id};
use crate::io::read_toml;
use crate::layout::resolve_layout_for_run;
use crate::types::InstallMetadata;

pub(crate) fn uninstall_app(home: &Path, app: &str, record_history: bool) -> Result<()> {
    let layout = resolve_layout_for_run(home, app);
    let app_root = layout
        .repo_dir
        .parent()
        .ok_or_else(|| anyhow!("invalid app layout for {}", layout.id))?;

    let source_url = if layout.meta_file.exists() {
        read_toml::<InstallMetadata>(&layout.meta_file)
            .ok()
            .map(|m| m.source_url)
    } else {
        None
    };

    append_audit(
        home,
        &AuditEntry {
            id: new_action_id(),
            ts: audit_unix_ts(),
            kind: "uninstall".into(),
            summary: format!("uninstall {app}"),
            app: Some(layout.id.clone()),
            source_url,
            undoable: false,
        },
        record_history,
    )?;

    if app_root.exists() {
        fs::remove_dir_all(app_root)
            .with_context(|| format!("failed removing {}", app_root.display()))?;
    }
    Ok(())
}
