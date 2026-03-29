use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

use crate::config::save_config;
use crate::types::Config;

/// Temp-dir folder prefix for [`ephemeral_bmx_home`]. Orphaned dirs (e.g. after a crash) are removed by [`clean_bmx_temp_directories`].
pub(crate) const TEMP_DIR_PREFIX_EPHEMERAL: &str = "bmx-ephemeral-";
/// Temp checkout for `bmx trust import-repo` when the app is not installed.
pub(crate) const TEMP_DIR_PREFIX_TRUST_IMPORT: &str = "bmx-trust-import-";
/// Temp checkout for `bmx trust check` git sources.
pub(crate) const TEMP_DIR_PREFIX_TRUST_CHECK: &str = "bmx-trust-check-";

/// `true` for directory names bmx creates under [`std::env::temp_dir`].
pub(crate) fn is_bmx_managed_temp_dir(name: &str) -> bool {
    name.starts_with(TEMP_DIR_PREFIX_EPHEMERAL)
        || name.starts_with(TEMP_DIR_PREFIX_TRUST_IMPORT)
        || name.starts_with(TEMP_DIR_PREFIX_TRUST_CHECK)
}

#[derive(Debug, Default)]
pub(crate) struct CleanTempSummary {
    pub(crate) matched: usize,
    pub(crate) removed: usize,
    pub(crate) failures: Vec<(PathBuf, String)>,
}

/// Deletes bmx-owned directories directly under the system temporary directory.
pub(crate) fn clean_bmx_temp_directories(dry_run: bool) -> Result<CleanTempSummary> {
    let tmp = std::env::temp_dir();
    let mut summary = CleanTempSummary::default();
    for entry in fs::read_dir(&tmp).with_context(|| format!("read {}", tmp.display()))? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        if !is_bmx_managed_temp_dir(name_str) {
            continue;
        }
        summary.matched += 1;
        let path = entry.path();
        if dry_run {
            summary.removed += 1;
            continue;
        }
        match fs::remove_dir_all(&path) {
            Ok(()) => summary.removed += 1,
            Err(e) => summary.failures.push((path, format!("{e:#}"))),
        }
    }
    Ok(summary)
}

fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
    if !src.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(dst).with_context(|| format!("failed to create {}", dst.display()))?;
    for entry in fs::read_dir(src).with_context(|| format!("read {}", src.display()))? {
        let entry = entry?;
        let path = entry.path();
        let dest = dst.join(entry.file_name());
        if path.is_dir() {
            copy_tree(&path, &dest)?;
        } else {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create parent {}", parent.display()))?;
            }
            fs::copy(&path, &dest).with_context(|| {
                format!("failed to copy {} -> {}", path.display(), dest.display())
            })?;
        }
    }
    Ok(())
}

/// Creates a throwaway `BMX_HOME`. Reuses **`persistent_bmx_home`** trust material
/// (`trust.toml` and `trust/`) so `--rm` still honors allowlists and deny rules; other
/// config stays isolated (`config.toml` defaults only).
pub(crate) fn ephemeral_bmx_home(persistent_bmx_home: &Path) -> Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "{}{}-{}",
        TEMP_DIR_PREFIX_EPHEMERAL,
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(dir.join("apps"))?;
    save_config(&dir, &Config::default())?;

    let trust_toml = persistent_bmx_home.join("trust.toml");
    if trust_toml.is_file() {
        fs::copy(&trust_toml, dir.join("trust.toml")).with_context(|| {
            format!(
                "failed to copy trust policy {} -> {}",
                trust_toml.display(),
                dir.join("trust.toml").display()
            )
        })?;
    }

    let trust_dir = persistent_bmx_home.join("trust");
    if trust_dir.is_dir() {
        copy_tree(&trust_dir, &dir.join("trust"))?;
    }

    Ok(dir)
}

pub(crate) fn remove_dir_all_best_effort(path: &Path) {
    if path.exists() {
        let _ = fs::remove_dir_all(path);
    }
}

#[cfg(test)]
mod tests {
    use super::is_bmx_managed_temp_dir;

    #[test]
    fn managed_temp_dir_detection() {
        assert!(is_bmx_managed_temp_dir("bmx-ephemeral-1-2"));
        assert!(is_bmx_managed_temp_dir("bmx-trust-import-9-0"));
        assert!(is_bmx_managed_temp_dir("bmx-trust-check-3-4"));
        assert!(!is_bmx_managed_temp_dir("bmx"));
        assert!(!is_bmx_managed_temp_dir("npm-123"));
    }
}
