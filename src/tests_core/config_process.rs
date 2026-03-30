use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serial_test::serial;
use temp_env::with_var;
use tempfile::TempDir;

use crate::config::{bmx_home, ensure_base_dirs, load_config, save_config};
use crate::io::{path_relative_to, read_toml, write_toml};
use crate::process::{forward_exit, has_tool, run_checked};
use crate::types::{BuildIsolation, Config};

#[test]
fn config_and_io_roundtrip_and_path_errors() {
    let tmp = TempDir::new().expect("tempdir");
    ensure_base_dirs(tmp.path()).expect("ensure dirs");

    let cfg = Config {
        default_source: Some("https://example.test/org".to_string()),
        build_isolation: BuildIsolation::Auto,
        ..Default::default()
    };
    save_config(tmp.path(), &cfg).expect("save");
    let loaded = load_config(tmp.path()).expect("load");
    assert_eq!(loaded.default_source, cfg.default_source);

    let custom = tmp.path().join("x.toml");
    write_toml(&custom, &cfg).expect("write toml");
    let read_back: Config = read_toml(&custom).expect("read toml");
    assert_eq!(read_back.default_source, cfg.default_source);

    let outside = PathBuf::from("/tmp/not-under-base");
    let err = path_relative_to(&outside, tmp.path()).expect_err("must fail");
    assert!(err.to_string().contains("is not under"));

    let child = tmp.path().join("a/b");
    fs::create_dir_all(&child).expect("mkdir child");
    let rel = path_relative_to(&child, tmp.path()).expect("relative path");
    assert_eq!(rel, "a/b");
}

#[test]
#[serial]
fn process_helpers_cover_command_and_exit() {
    assert!(has_tool("sh"));
    assert!(!has_tool("definitely_not_installed_bmx_test_tool"));

    run_checked(None, "sh", &["-c", "exit 0"], true).expect("success command");
    let err = run_checked(None, "sh", &["-c", "exit 9"], true).expect_err("must fail");
    assert!(err.to_string().contains("command failed"));

    let status = Command::new("sh")
        .arg("-c")
        .arg("exit 0")
        .status()
        .expect("status");
    forward_exit(status).expect("forward success");
}

#[test]
#[serial]
fn bmx_home_uses_home_env() {
    with_var("HOME", Some("/tmp/bmx-home-test"), || {
        let home = bmx_home().expect("home path");
        assert_eq!(home, PathBuf::from("/tmp/bmx-home-test/.bmx"));
    });
}

#[test]
fn load_config_defaults_when_config_file_is_missing() {
    let tmp = TempDir::new().expect("tempdir");
    let cfg = load_config(tmp.path()).expect("load default");
    assert_eq!(cfg.default_source.as_deref(), Some("https://github.com"));
    assert_eq!(cfg.build_isolation, BuildIsolation::Off);
    assert_eq!(cfg.run_isolation, BuildIsolation::Off);
    assert_eq!(cfg.checkout_backend, crate::types::CheckoutBackend::Git);
    assert!(cfg.checkout_profiles.is_empty());
    assert!(!cfg.integrity_check);
    assert!(cfg.registries.is_empty());
}
