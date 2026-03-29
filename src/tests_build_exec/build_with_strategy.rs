use std::fs;

use serial_test::serial;
use temp_env::with_var;
use tempfile::TempDir;

use crate::build::build_with_strategy;
use crate::types::{BuildIsolation, BuildStrategy};

use super::helpers::{write_cwd_stub_command, write_stub_command};

#[test]
#[serial]
fn build_with_strategy_covers_all_supported_build_paths() {
    let tools = TempDir::new().expect("tool dir");
    for bin in ["cargo", "cmake", "make", "brew", "yay", "paru", "makepkg"] {
        write_stub_command(tools.path(), bin);
    }

    let old = std::env::var("PATH").unwrap_or_default();
    let path = if old.is_empty() {
        tools.path().display().to_string()
    } else {
        format!("{}:{old}", tools.path().display())
    };

    with_var("PATH", Some(path), || {
        let rust = TempDir::new().expect("rust");
        build_with_strategy(
            &BuildStrategy::RustCargo,
            rust.path(),
            BuildIsolation::Off,
            false,
            None,
        )
        .expect("rust");

        let cmake = TempDir::new().expect("cmake");
        build_with_strategy(
            &BuildStrategy::CMake,
            cmake.path(),
            BuildIsolation::Off,
            false,
            None,
        )
        .expect("cmake");

        let make = TempDir::new().expect("make");
        build_with_strategy(
            &BuildStrategy::Make,
            make.path(),
            BuildIsolation::Off,
            false,
            None,
        )
        .expect("make");

        let brewfile = TempDir::new().expect("brewfile");
        fs::write(brewfile.path().join("Brewfile"), "brew 'x'\n").expect("brewfile");
        build_with_strategy(
            &BuildStrategy::Homebrew,
            brewfile.path(),
            BuildIsolation::Off,
            false,
            None,
        )
        .expect("brewfile");

        let formula = TempDir::new().expect("formula");
        fs::write(
            formula.path().join("Tool.rb"),
            "class Tool < Formula\nend\n",
        )
        .expect("rb");
        build_with_strategy(
            &BuildStrategy::Homebrew,
            formula.path(),
            BuildIsolation::Off,
            false,
            None,
        )
        .expect("formula");

        let aur = TempDir::new().expect("aur");
        build_with_strategy(
            &BuildStrategy::Aur,
            aur.path(),
            BuildIsolation::Off,
            false,
            None,
        )
        .expect("aur");
    });
}

#[test]
#[serial]
fn build_with_strategy_honors_bmx_workdir() {
    let tools = TempDir::new().expect("tool dir");
    write_cwd_stub_command(tools.path(), "cargo");

    let old = std::env::var("PATH").unwrap_or_default();
    let path = if old.is_empty() {
        tools.path().display().to_string()
    } else {
        format!("{}:{old}", tools.path().display())
    };

    with_var("PATH", Some(path), || {
        let repo = TempDir::new().expect("repo");
        fs::create_dir_all(repo.path().join("subproj")).expect("mkdir");
        fs::write(
            repo.path().join("bmx.toml"),
            "[bmx]\nworkdir = \"subproj\"\n",
        )
        .expect("manifest");
        fs::write(
            repo.path().join("subproj/Cargo.toml"),
            "[package]\nname='x'\n",
        )
        .expect("cargo");

        let out = repo.path().join("cwd.txt");
        with_var(
            "BMX_TEST_CWD_OUT",
            Some(out.to_string_lossy().to_string()),
            || {
                build_with_strategy(
                    &BuildStrategy::RustCargo,
                    repo.path(),
                    BuildIsolation::Off,
                    false,
                    None,
                )
                .expect("build");
            },
        );

        let ran_cwd = fs::read_to_string(out).expect("read cwd");
        assert!(ran_cwd.trim_end().ends_with("/subproj"));
    });
}
