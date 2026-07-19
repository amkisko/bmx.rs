//! Shared release helpers: workspace version, packaging sync, and loc limits.

mod loc_limits;
mod packaging_sync;

pub use loc_limits::{
    HARD_LIMIT as HARD_LOC_LIMIT, LocFinding, LocFindingKind, LocReport,
    SOFT_LIMIT as SOFT_LOC_LIMIT, check_loc_limits, write_baseline as write_loc_baseline,
};
pub use packaging_sync::{check_packaging, sync_packaging};

use std::fs;
use std::path::Path;

pub fn workspace_version(root: &Path) -> String {
    let content = fs::read_to_string(root.join("Cargo.toml")).expect("read root Cargo.toml");
    let mut in_workspace_package = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[workspace.package]" {
            in_workspace_package = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_workspace_package = false;
        }
        if in_workspace_package && trimmed.starts_with("version = ") {
            return trimmed
                .trim_start_matches("version = ")
                .trim_matches('"')
                .trim()
                .to_string();
        }
    }
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("version = ") && !line.contains("workspace") {
            return line
                .trim_start_matches("version = ")
                .trim_matches('"')
                .trim()
                .to_string();
        }
    }
    panic!("version not found in workspace Cargo.toml");
}
