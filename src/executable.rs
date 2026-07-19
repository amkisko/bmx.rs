use std::fs;
use std::path::Component;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};

use crate::app_spec::AppSpec;
use crate::io::{path_relative_to, read_toml};
use crate::source::sanitize_bin_name;
use crate::types::BMXManifest;

pub(crate) fn validate_repo_relative_path(path: &str, field: &str) -> Result<()> {
    let relative = Path::new(path);
    if relative.as_os_str().is_empty() {
        bail!("bmx.toml {field} must not be empty");
    }
    if relative.is_absolute() {
        bail!("bmx.toml {field} must be a relative path");
    }
    if relative
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        bail!("bmx.toml {field} must not contain '..'");
    }
    Ok(())
}

/// Resolve `executable_rel` under `repo_dir` and refuse paths that escape the install tree.
pub(crate) fn resolve_executable_in_repo(repo_dir: &Path, executable_rel: &str) -> Result<PathBuf> {
    validate_repo_relative_path(executable_rel, "executable path")?;
    let repo_canonical = repo_dir
        .canonicalize()
        .with_context(|| format!("failed to resolve repo {}", repo_dir.display()))?;
    // Join onto the canonical repo so missing files still stay under the install tree
    // (macOS /var vs /private/var must not false-fail when canonicalize is unavailable).
    let joined = repo_canonical.join(executable_rel);
    match joined.canonicalize() {
        Ok(executable) => {
            if !executable.starts_with(&repo_canonical) {
                bail!(
                    "executable path escapes install tree: {} (repo {})",
                    executable_rel,
                    repo_dir.display()
                );
            }
            Ok(executable)
        }
        Err(_) => Ok(joined),
    }
}

pub(crate) fn detect_executable_rel(repo_dir: &Path, spec: &AppSpec) -> Result<String> {
    let manifest = load_manifest(repo_dir)?;
    let build_dir = manifest_workdir(repo_dir, &manifest)?;
    if let Some(path) = manifest.bmx.and_then(|b| b.run) {
        validate_repo_relative_path(&path, "bmx.run")?;
        let run_path = repo_dir.join(&path);
        if run_path.exists() {
            let _ = resolve_executable_in_repo(repo_dir, &path)?;
            return Ok(path);
        }

        let run_in_workdir = build_dir.join(&path);
        if run_in_workdir.exists() {
            let relative = path_relative_to(&run_in_workdir, repo_dir)?;
            let _ = resolve_executable_in_repo(repo_dir, &relative)?;
            return Ok(relative);
        }

        bail!(
            "bmx.toml defines bmx.run={}, but file does not exist",
            run_path.display()
        );
    }

    let release_dir = build_dir.join("target").join("release");
    if let Some(pkg) = spec.cargo_package.as_deref() {
        for name in rust_binary_name_candidates(pkg) {
            let p = release_dir.join(name);
            if p.exists() {
                return path_relative_to(&p, repo_dir);
            }
        }
    }

    let rust_candidate = release_dir.join(sanitize_bin_name(&spec.source));
    if rust_candidate.exists() {
        return path_relative_to(&rust_candidate, repo_dir);
    }

    let fallback = scan_executable_candidates(&build_dir).ok_or_else(|| {
        anyhow!("unable to detect executable output; add bmx.toml with [bmx] run = \"path/to/bin\"")
    })?;
    path_relative_to(&fallback, repo_dir)
}

fn rust_binary_name_candidates(pkg: &str) -> Vec<String> {
    let underscored = pkg.replace('-', "_");
    if underscored == pkg {
        vec![pkg.to_string()]
    } else {
        vec![pkg.to_string(), underscored]
    }
}

fn scan_executable_candidates(repo_dir: &Path) -> Option<PathBuf> {
    let roots = [
        repo_dir.join("build"),
        repo_dir.join("bin"),
        repo_dir.join("target/release"),
    ];

    for root in roots {
        if !root.exists() {
            continue;
        }
        if let Some(found) = find_first_executable(&root) {
            return Some(found);
        }
    }

    None
}

fn find_first_executable(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_first_executable(&path) {
                return Some(found);
            }
            continue;
        }

        if path.is_file() && is_probably_executable(&path) {
            return Some(path);
        }
    }
    None
}

fn load_manifest(repo_dir: &Path) -> Result<BMXManifest> {
    let manifest_file = repo_dir.join("bmx.toml");
    if manifest_file.exists() {
        return read_toml(&manifest_file);
    }
    Ok(BMXManifest::default())
}

fn manifest_workdir(repo_dir: &Path, manifest: &BMXManifest) -> Result<PathBuf> {
    let Some(path) = manifest.bmx.as_ref().and_then(|b| b.workdir.as_deref()) else {
        return Ok(repo_dir.to_path_buf());
    };
    if path.is_empty() {
        return Ok(repo_dir.to_path_buf());
    }

    let workdir = Path::new(path);
    if workdir.is_absolute() {
        bail!("bmx.toml bmx.workdir must be a relative path");
    }
    if workdir
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        bail!("bmx.toml bmx.workdir must not contain '..'");
    }

    let full = repo_dir.join(workdir);
    if !full.is_dir() {
        bail!("bmx.toml bmx.workdir={} is not a directory", full.display());
    }
    Ok(full)
}

#[cfg(unix)]
fn is_probably_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|meta| meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_probably_executable(path: &Path) -> bool {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(|ext| ext.eq_ignore_ascii_case("exe"))
        .unwrap_or(false)
}
