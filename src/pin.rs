use std::fs;
use std::path::Path;

use anyhow::{Result, bail};

/// First non-empty line from `.bmx/pin` in `start` or any parent directory.
pub(crate) fn read_pin_from_ancestors(start: &Path) -> Result<String> {
    let mut dir = if start.is_dir() {
        start.to_path_buf()
    } else {
        start
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| Path::new(".").to_path_buf())
    };

    loop {
        let candidate = dir.join(".bmx").join("pin");
        if candidate.is_file() {
            let text = fs::read_to_string(&candidate)
                .map_err(|e| anyhow::anyhow!("failed reading {}: {e}", candidate.display()))?;
            let line = text.lines().find(|l| !l.trim().is_empty());
            let pin = line.map(str::trim).unwrap_or("").to_string();
            if pin.is_empty() {
                bail!(
                    "{} is empty; put one app line (same form as `bmx install`) in the file",
                    candidate.display()
                );
            }
            return Ok(pin);
        }
        if !dir.pop() {
            break;
        }
    }

    bail!(
        "no .bmx/pin found in {} or any parent directory",
        start.display()
    )
}
