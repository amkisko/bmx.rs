use std::fs;

use tempfile::TempDir;

use crate::app_spec::parse_app_spec;
use crate::executable::detect_executable_rel;

#[cfg(unix)]
use super::helpers::make_executable;

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
    fs::write(rust_qualified.path().join("target/release/my_tool"), "x").expect("write");
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
