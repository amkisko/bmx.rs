use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use anyhow::{Result, anyhow};

use crate::isolation::run_build_checked;
use crate::process::has_tool;
use crate::types::{BuildIsolation, BuildStrategy};

use super::BuildPlugin;

struct RustCargoPlugin;
impl BuildPlugin for RustCargoPlugin {
    fn strategy(&self) -> BuildStrategy {
        BuildStrategy::RustCargo
    }
    fn detect(&self, build_dir: &Path) -> bool {
        build_dir.join("Cargo.toml").exists()
    }
    fn build(
        &self,
        _repo_dir: &Path,
        build_dir: &Path,
        isolation: BuildIsolation,
        show_output: bool,
        rust_package: Option<&str>,
    ) -> Result<()> {
        if let Some(pkg) = rust_package {
            run_build_checked(
                build_dir,
                "cargo",
                &["build", "--release", "-p", pkg],
                isolation,
                show_output,
            )
        } else {
            run_build_checked(
                build_dir,
                "cargo",
                &["build", "--release"],
                isolation,
                show_output,
            )
        }
    }
}

struct CMakePlugin;
impl BuildPlugin for CMakePlugin {
    fn strategy(&self) -> BuildStrategy {
        BuildStrategy::CMake
    }
    fn detect(&self, build_dir: &Path) -> bool {
        build_dir.join("CMakeLists.txt").exists()
    }
    fn build(
        &self,
        _repo_dir: &Path,
        build_dir: &Path,
        isolation: BuildIsolation,
        show_output: bool,
        _rust_package: Option<&str>,
    ) -> Result<()> {
        run_build_checked(
            build_dir,
            "cmake",
            &["-S", ".", "-B", "build", "-DCMAKE_BUILD_TYPE=Release"],
            isolation,
            show_output,
        )?;
        run_build_checked(
            build_dir,
            "cmake",
            &["--build", "build", "--config", "Release"],
            isolation,
            show_output,
        )
    }
}

struct AurPlugin;
impl BuildPlugin for AurPlugin {
    fn strategy(&self) -> BuildStrategy {
        BuildStrategy::Aur
    }
    fn detect(&self, build_dir: &Path) -> bool {
        build_dir.join("PKGBUILD").exists()
    }
    fn build(
        &self,
        _repo_dir: &Path,
        build_dir: &Path,
        isolation: BuildIsolation,
        show_output: bool,
        _rust_package: Option<&str>,
    ) -> Result<()> {
        build_aur(build_dir, isolation, show_output)
    }
}

struct HomebrewPlugin;
impl BuildPlugin for HomebrewPlugin {
    fn strategy(&self) -> BuildStrategy {
        BuildStrategy::Homebrew
    }
    fn detect(&self, build_dir: &Path) -> bool {
        build_dir.join("Brewfile").exists() || has_formula_file(build_dir)
    }
    fn build(
        &self,
        _repo_dir: &Path,
        build_dir: &Path,
        isolation: BuildIsolation,
        show_output: bool,
        _rust_package: Option<&str>,
    ) -> Result<()> {
        build_homebrew(build_dir, isolation, show_output)
    }
}

struct MakePlugin;
impl BuildPlugin for MakePlugin {
    fn strategy(&self) -> BuildStrategy {
        BuildStrategy::Make
    }
    fn detect(&self, build_dir: &Path) -> bool {
        build_dir.join("Makefile").exists() || build_dir.join("makefile").exists()
    }
    fn build(
        &self,
        _repo_dir: &Path,
        build_dir: &Path,
        isolation: BuildIsolation,
        show_output: bool,
        _rust_package: Option<&str>,
    ) -> Result<()> {
        run_build_checked(build_dir, "make", &["-j"], isolation, show_output)
    }
}

/// Ordered registry: first match wins (same rules as before this refactor).
pub(super) static BUILD_PLUGINS: &[&dyn BuildPlugin] = &[
    &RustCargoPlugin,
    &CMakePlugin,
    &AurPlugin,
    &HomebrewPlugin,
    &MakePlugin,
];

fn build_homebrew(repo_dir: &Path, isolation: BuildIsolation, show_output: bool) -> Result<()> {
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
