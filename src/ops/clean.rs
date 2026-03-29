use anyhow::Result;

use crate::workspace::clean_bmx_temp_directories;

pub(crate) fn clean_temp_artifacts(dry_run: bool, verbose: bool) -> Result<()> {
    let summary = clean_bmx_temp_directories(dry_run)?;
    if dry_run || verbose {
        eprintln!(
            "[bmx][clean] scanned system temp for bmx-ephemeral-*, bmx-trust-import-*, bmx-trust-check-* ({})",
            if dry_run { "dry run" } else { "deleting" }
        );
    }
    for (path, err) in &summary.failures {
        eprintln!("[bmx][clean] failed to remove {}: {err}", path.display());
    }
    if summary.matched == 0 {
        println!("bmx clean: nothing to remove");
        return Ok(());
    }
    let action = if dry_run { "would remove" } else { "removed" };
    println!(
        "bmx clean: {action} {} director{}",
        summary.removed,
        if summary.removed == 1 { "y" } else { "ies" }
    );
    Ok(())
}
