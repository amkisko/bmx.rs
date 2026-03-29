mod plugins;

use std::path::Path;

use anyhow::{Result, anyhow, bail};

use crate::io::read_toml;
use crate::types::{BMXManifest, BuildIsolation, BuildStrategy};

use plugins::BUILD_PLUGINS;

/// Build plugins implement detection and compilation for a stack (Rust, CMake, …).
pub(crate) trait BuildPlugin: Sync {
    fn strategy(&self) -> BuildStrategy;
    fn detect(&self, build_dir: &Path) -> bool;
    fn build(
        &self,
        repo_dir: &Path,
        build_dir: &Path,
        isolation: BuildIsolation,
        show_output: bool,
        rust_package: Option<&str>,
    ) -> Result<()>;
}

#[allow(clippy::collapsible_if)]
pub(crate) fn detect_strategy(repo_dir: &Path) -> Option<BuildStrategy> {
    let manifest_path = repo_dir.join("bmx.toml");
    let mut forced: Option<BuildStrategy> = None;
    if manifest_path.exists() {
        if let Ok(manifest) = read_toml::<BMXManifest>(&manifest_path) {
            if let Some(name) = manifest.bmx.as_ref().and_then(|b| b.strategy.as_deref()) {
                forced = BuildStrategy::parse(name);
            }
        }
    }

    let build_dir = manifest_workdir(repo_dir).unwrap_or_else(|_| repo_dir.to_path_buf());

    if let Some(s) = forced {
        return Some(s);
    }

    BUILD_PLUGINS
        .iter()
        .find(|p| p.detect(&build_dir))
        .map(|p| p.strategy())
}

pub(crate) fn build_with_strategy(
    strategy: &BuildStrategy,
    repo_dir: &Path,
    isolation: BuildIsolation,
    show_output: bool,
    rust_package: Option<&str>,
) -> Result<()> {
    let build_dir = manifest_workdir(repo_dir)?;
    let plugin = BUILD_PLUGINS
        .iter()
        .find(|p| &p.strategy() == strategy)
        .ok_or_else(|| anyhow!("internal error: unknown strategy {:?}", strategy))?;
    plugin.build(repo_dir, &build_dir, isolation, show_output, rust_package)
}

fn manifest_workdir(repo_dir: &Path) -> Result<std::path::PathBuf> {
    let manifest_file = repo_dir.join("bmx.toml");
    if !manifest_file.exists() {
        return Ok(repo_dir.to_path_buf());
    }

    let manifest: BMXManifest = read_toml(&manifest_file)?;
    let Some(path) = manifest.bmx.and_then(|b| b.workdir) else {
        return Ok(repo_dir.to_path_buf());
    };
    if path.is_empty() {
        return Ok(repo_dir.to_path_buf());
    }

    let workdir = Path::new(&path);
    if workdir.is_absolute() {
        bail!("bmx.toml bmx.workdir must be a relative path");
    }
    if workdir
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        bail!("bmx.toml bmx.workdir must not contain '..'");
    }

    let full = repo_dir.join(workdir);
    if !full.is_dir() {
        bail!("bmx.toml bmx.workdir={} is not a directory", full.display());
    }
    Ok(full)
}
