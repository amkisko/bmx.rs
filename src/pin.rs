use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::app_spec::{layout_id, parse_app_spec};
use crate::io::{read_toml, write_toml};

const PINS_FILE: &str = "pins.toml";
const LEGACY_PIN_FILE: &str = "pin";

/// Multi-app pins: each key is a layout id (`layout_id` / `~/.bmx/apps/<key>/`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct PinsFile {
    /// Layout id of the app `bmx --pin` runs when multiple `[apps]` entries exist.
    #[serde(default)]
    pub(crate) default: Option<String>,
    #[serde(default)]
    pub(crate) apps: BTreeMap<String, String>,
}

/// Resolve which spec to run for `bmx --pin` / `bmx exec --pin` with no explicit app.
#[allow(clippy::collapsible_if)]
pub(crate) fn resolve_implicit_pin_spec(start: &Path) -> Result<String> {
    let pins = load_pins_from_ancestors(start)?;
    match pins.apps.len() {
        0 => bail!("no app entries in pins file"),
        1 => Ok(pins.apps.values().next().expect("len checked").clone()),
        _ => {
            if let Some(d) = &pins.default {
                if let Some(s) = pins.apps.get(d) {
                    return Ok(s.clone());
                }
            }
            let keys: Vec<&str> = pins.apps.keys().map(String::as_str).collect();
            bail!(
                "multiple pins in .bmx/pins.toml; set `default` to a layout id (keys: {}) or pass an explicit app spec",
                keys.join(", ")
            );
        }
    }
}

/// Merge `spec` under its layout id into `{dir}/.bmx/pins.toml` (migrates legacy `.bmx/pin` if present).
pub(crate) fn upsert_pin_in_dir(dir: &Path, spec: &str) -> Result<()> {
    let s = spec.trim();
    if s.is_empty() {
        bail!("pin spec is empty");
    }
    let parsed = parse_app_spec(s);
    let key = layout_id(&parsed);
    let bmx = dir.join(".bmx");
    fs::create_dir_all(&bmx)?;
    let pins_path = bmx.join(PINS_FILE);
    let legacy_path = bmx.join(LEGACY_PIN_FILE);

    let mut pins = if pins_path.is_file() {
        read_toml::<PinsFile>(&pins_path)?
    } else if legacy_path.is_file() {
        legacy_file_to_pins(&legacy_path)?
    } else {
        PinsFile::default()
    };

    pins.apps.insert(key.clone(), s.to_string());
    if pins.default.is_none() && pins.apps.len() == 1 {
        pins.default = Some(key);
    }

    write_toml(&pins_path, &pins)?;
    if legacy_path.is_file() {
        let _ = fs::remove_file(&legacy_path);
    }
    Ok(())
}

fn load_pins_from_ancestors(start: &Path) -> Result<PinsFile> {
    let mut dir = if start.is_dir() {
        start.to_path_buf()
    } else {
        start
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| Path::new(".").to_path_buf())
    };

    loop {
        let bmx = dir.join(".bmx");
        let pins_path = bmx.join(PINS_FILE);
        if pins_path.is_file() {
            return read_toml::<PinsFile>(&pins_path);
        }
        let legacy = bmx.join(LEGACY_PIN_FILE);
        if legacy.is_file() {
            return legacy_file_to_pins(&legacy);
        }
        if !dir.pop() {
            break;
        }
    }

    bail!(
        "no .bmx/pins.toml or .bmx/pin found in {} or any parent directory",
        start.display()
    )
}

fn legacy_file_to_pins(path: &Path) -> Result<PinsFile> {
    let text = fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("failed reading {}: {e}", path.display()))?;
    let line = text
        .lines()
        .find(|l| !l.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("{} is empty", path.display()))?;
    let spec = line.trim();
    if spec.is_empty() {
        bail!(
            "{} is empty; put one app line (same form as `bmx install`) in the file",
            path.display()
        );
    }
    let parsed = parse_app_spec(spec);
    let key = layout_id(&parsed);
    let mut apps = BTreeMap::new();
    apps.insert(key.clone(), spec.to_string());
    Ok(PinsFile {
        default: Some(key),
        apps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn legacy_roundtrip_to_pins_file() {
        let dir = tempdir().unwrap();
        let legacy = dir.path().join(".bmx/pin");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        fs::write(&legacy, "https://example.com/foo.git@v1\n").unwrap();
        let p = legacy_file_to_pins(&legacy).unwrap();
        assert_eq!(p.apps.len(), 1);
        assert!(p.default.is_some());
    }
}
