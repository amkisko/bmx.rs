use std::fs;

use tempfile::TempDir;

use crate::build::detect_strategy;
use crate::types::BuildStrategy;

#[test]
fn detect_strategy_covers_all_markers_and_none() {
    let empty = TempDir::new().expect("tmp");
    assert!(detect_strategy(empty.path()).is_none());

    fs::write(empty.path().join("Cargo.toml"), "[package]\nname='x'\n").expect("cargo");
    assert!(matches!(
        detect_strategy(empty.path()),
        Some(BuildStrategy::RustCargo)
    ));

    let cmake = TempDir::new().expect("tmp");
    fs::write(cmake.path().join("CMakeLists.txt"), "project(x)\n").expect("cmake");
    assert!(matches!(
        detect_strategy(cmake.path()),
        Some(BuildStrategy::CMake)
    ));

    let make = TempDir::new().expect("tmp");
    fs::write(make.path().join("Makefile"), "all:\n\t@true\n").expect("make");
    assert!(matches!(
        detect_strategy(make.path()),
        Some(BuildStrategy::Make)
    ));

    let brew = TempDir::new().expect("tmp");
    fs::write(brew.path().join("Tool.rb"), "class Tool < Formula\nend\n").expect("rb");
    assert!(matches!(
        detect_strategy(brew.path()),
        Some(BuildStrategy::Homebrew)
    ));

    let aur = TempDir::new().expect("tmp");
    fs::write(aur.path().join("PKGBUILD"), "pkgname=tool\n").expect("pkgbuild");
    assert!(matches!(
        detect_strategy(aur.path()),
        Some(BuildStrategy::Aur)
    ));
}

#[test]
fn detect_strategy_honors_bmx_toml_strategy_override() {
    let repo = TempDir::new().expect("tmp");
    fs::write(repo.path().join("bmx.toml"), "[bmx]\nstrategy = \"make\"\n").expect("manifest");
    fs::write(repo.path().join("Cargo.toml"), "[package]\nname='x'\n").expect("cargo");
    fs::write(repo.path().join("Makefile"), "all:\n\t@true\n").expect("make");
    assert!(matches!(
        detect_strategy(repo.path()),
        Some(BuildStrategy::Make)
    ));
}

#[test]
fn detect_strategy_honors_bmx_workdir() {
    let repo = TempDir::new().expect("tmp");
    fs::create_dir_all(repo.path().join("tooling")).expect("mkdir");
    fs::write(
        repo.path().join("bmx.toml"),
        "[bmx]\nworkdir = \"tooling\"\n",
    )
    .expect("manifest");
    fs::write(
        repo.path().join("tooling/Cargo.toml"),
        "[package]\nname='x'\n",
    )
    .expect("cargo");

    assert!(matches!(
        detect_strategy(repo.path()),
        Some(BuildStrategy::RustCargo)
    ));
}
