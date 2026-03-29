use std::fs;
use std::path::Path;

use serial_test::serial;
use temp_env::with_var;
use tempfile::TempDir;

use crate::app_spec::parse_app_spec;
use crate::build::{build_with_strategy, detect_strategy};
use crate::executable::detect_executable_rel;
use crate::types::{BuildIsolation, BuildStrategy};

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("chmod");
}

fn write_stub_command(dir: &Path, name: &str) {
    let cmd = dir.join(name);
    fs::write(&cmd, "#!/bin/sh\nexit 0\n").expect("write stub");
    #[cfg(unix)]
    make_executable(&cmd);
}

fn write_cwd_stub_command(dir: &Path, name: &str) {
    let cmd = dir.join(name);
    fs::write(
        &cmd,
        "#!/bin/sh\nif [ -n \"$BMX_TEST_CWD_OUT\" ]; then\n  pwd > \"$BMX_TEST_CWD_OUT\"\nfi\nexit 0\n",
    )
    .expect("write stub");
    #[cfg(unix)]
    make_executable(&cmd);
}

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
    fs::write(
        repo.path().join("bmx.toml"),
        "[bmx]\nstrategy = \"make\"\n",
    )
    .expect("manifest");
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

#[test]
fn detect_executable_rel_paths_cover_manifest_rust_fallback_and_failure() {
    let manifest = TempDir::new().expect("tmp");
    fs::create_dir_all(manifest.path().join("bin")).expect("mkdir");
    fs::write(manifest.path().join("bin/tool"), "x").expect("write");
    fs::write(
        manifest.path().join("bmx.toml"),
        "[bmx]\nrun = \"bin/tool\"\n",
    )
    .expect("manifest");
    assert_eq!(
        detect_executable_rel(manifest.path(), &parse_app_spec("tool")).expect("manifest"),
        "bin/tool"
    );

    let rust = TempDir::new().expect("tmp");
    fs::create_dir_all(rust.path().join("target/release")).expect("mkdir");
    fs::write(rust.path().join("target/release/my_tool"), "x").expect("write");
    assert_eq!(
        detect_executable_rel(rust.path(), &parse_app_spec("my-tool")).expect("rust"),
        "target/release/my_tool"
    );

    let rust_qualified = TempDir::new().expect("tmp");
    fs::create_dir_all(rust_qualified.path().join("target/release")).expect("mkdir");
    fs::write(
        rust_qualified.path().join("target/release/my_tool"),
        "x",
    )
    .expect("write");
    assert_eq!(
        detect_executable_rel(rust_qualified.path(), &parse_app_spec("my-tool@^1.0"))
            .expect("rust qualified"),
        "target/release/my_tool"
    );

    let bad = TempDir::new().expect("tmp");
    fs::write(bad.path().join("bmx.toml"), "[bmx]\nrun = \"missing\"\n").expect("bad");
    assert!(detect_executable_rel(bad.path(), &parse_app_spec("tool")).is_err());

    let none = TempDir::new().expect("tmp");
    assert!(detect_executable_rel(none.path(), &parse_app_spec("tool")).is_err());
}

#[test]
fn detect_executable_rel_prefers_cargo_package_binary_name() {
    let tmp = TempDir::new().expect("tmp");
    fs::create_dir_all(tmp.path().join("target/release")).expect("mkdir");
    fs::write(tmp.path().join("target/release/scout"), "x").expect("write");
    fs::write(tmp.path().join("target/release/release"), "x").expect("noise bin");
    let spec = parse_app_spec("https://github.com/acme/wiki.rs:scout");
    assert_eq!(
        detect_executable_rel(tmp.path(), &spec).expect("detect"),
        "target/release/scout"
    );
}

#[test]
fn detect_executable_rel_honors_bmx_workdir() {
    let repo = TempDir::new().expect("tmp");
    fs::create_dir_all(repo.path().join("sub/target/release")).expect("mkdir");
    fs::write(repo.path().join("bmx.toml"), "[bmx]\nworkdir = \"sub\"\n").expect("manifest");
    fs::write(repo.path().join("sub/target/release/tool"), "x").expect("write");

    assert_eq!(
        detect_executable_rel(repo.path(), &parse_app_spec("tool")).expect("detect"),
        "sub/target/release/tool"
    );
}

#[test]
fn detect_executable_rel_resolves_manifest_run_from_workdir() {
    let repo = TempDir::new().expect("tmp");
    fs::create_dir_all(repo.path().join("sub/bin")).expect("mkdir");
    fs::write(
        repo.path().join("bmx.toml"),
        "[bmx]\nworkdir = \"sub\"\nrun = \"bin/tool\"\n",
    )
    .expect("manifest");
    fs::write(repo.path().join("sub/bin/tool"), "x").expect("write");

    assert_eq!(
        detect_executable_rel(repo.path(), &parse_app_spec("tool")).expect("detect"),
        "sub/bin/tool"
    );
}

#[cfg(unix)]
#[test]
fn detect_executable_rel_scans_nested_fallback_binary() {
    let tmp = TempDir::new().expect("tmp");
    let nested = tmp.path().join("build/out/tool");
    fs::create_dir_all(nested.parent().expect("parent")).expect("mkdir");
    fs::write(&nested, "x").expect("write");
    make_executable(&nested);

    let rel = detect_executable_rel(tmp.path(), &parse_app_spec("not-tool")).expect("fallback");
    assert_eq!(rel, "build/out/tool");
}
