use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub(crate) fn read_toml<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let content =
        fs::read_to_string(path).with_context(|| format!("failed reading {}", path.display()))?;
    toml::from_str(&content).with_context(|| format!("failed parsing {}", path.display()))
}

pub(crate) fn write_toml<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = toml::to_string(value)?;
    fs::write(path, content).with_context(|| format!("failed writing {}", path.display()))
}

pub(crate) fn path_relative_to(path: &Path, base: &Path) -> Result<String> {
    let rel = path
        .strip_prefix(base)
        .with_context(|| format!("{} is not under {}", path.display(), base.display()))?;
    Ok(rel.to_string_lossy().to_string())
}
