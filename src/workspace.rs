use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

use crate::config::save_config;
use crate::types::Config;

pub(crate) fn ephemeral_bmx_home() -> Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("bmx-ephemeral-{}-{}", std::process::id(), nanos));
    fs::create_dir_all(dir.join("apps"))?;
    // Isolated defaults — do not read the user's ~/.bmx config.
    save_config(&dir, &Config::default())?;
    Ok(dir)
}

pub(crate) fn remove_dir_all_best_effort(path: &Path) {
    if path.exists() {
        let _ = fs::remove_dir_all(path);
    }
}
