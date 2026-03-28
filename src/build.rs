use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use anyhow::{Result, anyhow, bail};

use crate::io::read_toml;
use crate::isolation::run_build_checked;
use crate::process::has_tool;
use crate::types::{BMXManifest, BuildIsolation, BuildStrategy};

pub(crate) fn detect_strategy(repo_dir: &Path) -> Option<BuildStrategy> {
    let build_dir = manifest_workdir(repo_dir).unwrap_or_else(|_| repo_dir.to_path_buf());
    if build_dir.join("Cargo.toml").exists() {
        return Some(BuildStrategy::RustCargo);
    }
    if build_dir.join("CMakeLists.txt").exists() {
        return Some(BuildStrategy::CMake);
    }
    if build_dir.join("PKGBUILD").exists() {
        return Some(BuildStrategy::Aur);
    }
    if build_dir.join("Brewfile").exists() || has_formula_file(&build_dir) {
        return Some(BuildStrategy::Homebrew);
    }
    if build_dir.join("Makefile").exists() || build_dir.join("makefile").exists() {
        return Some(BuildStrategy::Make);
    }
    None
}

pub(crate) fn build_with_strategy(
    strategy: &BuildStrategy,
    repo_dir: &Path,
    isolation: BuildIsolation,
    show_output: bool,
    rust_package: Option<&str>,
) -> Result<()> {
    let build_dir = manifest_workdir(repo_dir)?;
    match strategy {
        BuildStrategy::RustCargo => {
            if let Some(pkg) = rust_package {
                run_build_checked(
                    &build_dir,
                    "cargo",
                    &["build", "--release", "-p", pkg],
                    isolation,
                    show_output,
                )
            } else {
                run_build_checked(
                    &build_dir,
                    "cargo",
                    &["build", "--release"],
                    isolation,
                    show_output,
                )
            }
        }
        BuildStrategy::CMake => {
            run_build_checked(
                &build_dir,
                "cmake",
                &["-S", ".", "-B", "build", "-DCMAKE_BUILD_TYPE=Release"],
                isolation,
                show_output,
            )?;
            run_build_checked(
                &build_dir,
                "cmake",
                &["--build", "build", "--config", "Release"],
                isolation,
                show_output,
            )
        }
        BuildStrategy::Make => {
            run_build_checked(&build_dir, "make", &["-j"], isolation, show_output)
        }
        BuildStrategy::Homebrew => build_homebrew(&build_dir, isolation, show_output),
        BuildStrategy::Aur => build_aur(&build_dir, isolation, show_output),
    }
}

fn build_homebrew(
    repo_dir: &Path,
    isolation: BuildIsolation,
    show_output: bool,
) -> Result<()> {
    if repo_dir.join("Brewfile").exists() {
        return run_build_checked(
            repo_dir,
            "brew",
            &["bundle", "install"],
            isolation,
            show_output,
        );
    }

    let formula = detect_formula_file(repo_dir)
        .ok_or_else(|| anyhow!("homebrew strategy selected but no formula file found"))?;
    run_build_checked(
        repo_dir,
        "brew",
        &["install", "--build-from-source", &formula],
        isolation,
        show_output,
    )
}

fn build_aur(repo_dir: &Path, isolation: BuildIsolation, show_output: bool) -> Result<()> {
    if has_tool("yay") {
        return run_build_checked(
            repo_dir,
            "yay",
            &["-S", "--noconfirm", "--needed", "."],
            isolation,
            show_output,
        );
    }
    if has_tool("paru") {
        return run_build_checked(
            repo_dir,
            "paru",
            &["-S", "--noconfirm", "--needed", "."],
            isolation,
            show_output,
        );
    }

    run_build_checked(
        repo_dir,
        "makepkg",
        &["-si", "--noconfirm"],
        isolation,
        show_output,
    )
}

fn has_formula_file(repo_dir: &Path) -> bool {
    fs::read_dir(repo_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .any(|e| e.path().extension().and_then(OsStr::to_str) == Some("rb"))
}

fn detect_formula_file(repo_dir: &Path) -> Option<String> {
    fs::read_dir(repo_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().and_then(OsStr::to_str) == Some("rb"))
        .map(|path| path.to_string_lossy().to_string())
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
