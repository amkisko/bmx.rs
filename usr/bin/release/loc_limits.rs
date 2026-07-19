//! Line-count limits for workspace Rust sources.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const SOFT_LIMIT: usize = 150;
pub const HARD_LIMIT: usize = 300;

const BASELINE_FILE: &str = "usr/loc-baseline.tsv";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocFinding {
    pub path: String,
    pub lines: usize,
    pub kind: LocFindingKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocFindingKind {
    SoftLimit,
    HardLimit,
    BaselineExceeded { allowed: usize },
    BaselineShrink { allowed: usize },
}

pub struct LocReport {
    pub warnings: Vec<LocFinding>,
    pub errors: Vec<LocFinding>,
}

impl LocReport {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

pub fn check_loc_limits(root: &Path) -> Result<LocReport, String> {
    let baseline = load_baseline(root)?;
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    for path in collect_rust_sources(root)? {
        let relative = path
            .strip_prefix(root)
            .map_err(|error| format!("strip prefix from {}: {error}", path.display()))?
            .to_string_lossy()
            .replace('\\', "/");
        let lines = count_lines(&path)?;

        if let Some(&allowed) = baseline.get(relative.as_str()) {
            if lines > allowed {
                errors.push(LocFinding {
                    path: relative,
                    lines,
                    kind: LocFindingKind::BaselineExceeded { allowed },
                });
            } else if lines < allowed {
                warnings.push(LocFinding {
                    path: relative,
                    lines,
                    kind: LocFindingKind::BaselineShrink { allowed },
                });
            }
            continue;
        }

        if lines > HARD_LIMIT {
            errors.push(LocFinding {
                path: relative,
                lines,
                kind: LocFindingKind::HardLimit,
            });
        } else if lines > SOFT_LIMIT {
            warnings.push(LocFinding {
                path: relative,
                lines,
                kind: LocFindingKind::SoftLimit,
            });
        }
    }

    Ok(LocReport { warnings, errors })
}

pub fn write_baseline(root: &Path) -> Result<Vec<String>, String> {
    let mut oversized = Vec::new();

    for path in collect_rust_sources(root)? {
        let lines = count_lines(&path)?;
        if lines > HARD_LIMIT {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| format!("strip prefix from {}: {error}", path.display()))?
                .to_string_lossy()
                .replace('\\', "/");
            oversized.push((relative, lines));
        }
    }

    oversized.sort_by(|left, right| left.0.cmp(&right.0));

    let baseline_path = root.join(BASELINE_FILE);
    if let Some(parent) = baseline_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }

    let mut content = String::from("# path<TAB>lines — files above the hard limit; do not grow\n");
    for (path, lines) in &oversized {
        content.push_str(path);
        content.push('\t');
        content.push_str(&lines.to_string());
        content.push('\n');
    }

    fs::write(&baseline_path, content)
        .map_err(|error| format!("write {}: {error}", baseline_path.display()))?;

    Ok(oversized.into_iter().map(|(path, _)| path).collect())
}

fn load_baseline(root: &Path) -> Result<BTreeMap<String, usize>, String> {
    let path = root.join(BASELINE_FILE);
    if !path.is_file() {
        return Ok(BTreeMap::new());
    }

    let content =
        fs::read_to_string(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let mut baseline = BTreeMap::new();

    for (line_number, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((file_path, line_count)) = line.split_once('\t') else {
            return Err(format!(
                "invalid baseline entry at {}:{} (expected path<TAB>lines)",
                BASELINE_FILE,
                line_number + 1
            ));
        };

        let line_count = line_count.parse::<usize>().map_err(|error| {
            format!(
                "invalid line count in {}:{}: {error}",
                BASELINE_FILE,
                line_number + 1
            )
        })?;

        baseline.insert(file_path.to_string(), line_count);
    }

    Ok(baseline)
}

fn collect_rust_sources(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut sources = Vec::new();
    let members = workspace_members(root)?;

    for member in members {
        let member_root = root.join(&member);
        for directory in ["src", "tests"] {
            let directory_path = member_root.join(directory);
            if directory_path.is_dir() {
                collect_rust_files(&directory_path, &mut sources)?;
            }
        }

        if member == "usr/bin/release" {
            collect_rust_files(&member_root, &mut sources)?;
        }
    }

    sources.sort();
    sources.dedup();
    Ok(sources)
}

fn workspace_members(root: &Path) -> Result<Vec<String>, String> {
    let content = fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|error| format!("read Cargo.toml: {error}"))?;

    let Some(start) = content.find("members = [") else {
        return Err("members table not found in Cargo.toml".to_string());
    };
    let remainder = &content[start + "members = [".len()..];
    let Some(end) = remainder.find(']') else {
        return Err("unclosed members table in Cargo.toml".to_string());
    };

    let members = remainder[..end]
        .split(',')
        .filter_map(|entry| {
            let entry = entry.trim().trim_matches('"');
            if entry.is_empty() {
                None
            } else {
                Some(entry.to_string())
            }
        })
        .collect::<Vec<_>>();

    if members.is_empty() {
        return Err("no workspace members found in Cargo.toml".to_string());
    }

    Ok(members)
}

fn collect_rust_files(directory: &Path, sources: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in
        fs::read_dir(directory).map_err(|error| format!("read {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("read {}: {error}", directory.display()))?;
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some("target") {
                continue;
            }
            collect_rust_files(&path, sources)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            sources.push(path);
        }
    }
    Ok(())
}

fn count_lines(path: &Path) -> Result<usize, String> {
    let content =
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    Ok(content.lines().count())
}

#[cfg(test)]
#[path = "loc_limits_tests.rs"]
mod tests;
