use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serial_test::serial;
use temp_env::with_var;
use tempfile::TempDir;

use crate::app_spec::{layout_id, parse_app_spec};
use crate::config::{bmx_home, ensure_base_dirs, load_config, save_config};
use crate::io::{path_relative_to, read_toml, write_toml};
use crate::isolation::resolve_backend;
use crate::process::{forward_exit, has_tool, run_checked};
use crate::repo::resolve_checkout_plan;
use crate::source::{
    app_id, looks_like_url, normalize_explicit_url, normalize_source_base, resolve_source,
    sanitize_bin_name,
};
use crate::types::{
    BuildIsolation, BuildStrategy, CheckoutBackend, CheckoutProfile, Config, EnvVar,
};

#[test]
fn source_helpers_cover_normalization_and_ids() {
    assert!(looks_like_url("git@github.com:acme/tool.git"));
    assert!(!looks_like_url("tool"));
    assert_eq!(
        normalize_explicit_url("github.com/a/b"),
        "https://github.com/a/b"
    );
    assert_eq!(
        normalize_source_base("github.com/acme/"),
        "https://github.com/acme"
    );
    assert_eq!(app_id("https://github.com/acme/my-tool.git"), "my-tool");
    assert_eq!(sanitize_bin_name("my-tool"), "my_tool");
}

#[test]
fn resolve_source_covers_success_and_failure_paths() {
    let cfg = Config {
        default_source: Some("https://github.com/acme".to_string()),
        ..Default::default()
    };
    assert_eq!(
        resolve_source(&cfg, "tool").expect("source"),
        "https://github.com/acme/tool.git"
    );
    assert_eq!(
        resolve_source(&cfg, "/tool").expect("source"),
        "https://github.com/acme/tool.git"
    );

    let missing = Config {
        default_source: None,
        ..Default::default()
    };
    let err = resolve_source(&missing, "tool").expect_err("should fail");
    assert!(err.to_string().contains("default source is not configured"));

    let reg = Config {
        default_source: None,
        registries: [(
            "acme".to_string(),
            "https://git.acme.com".to_string(),
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    };
    assert_eq!(
        resolve_source(&reg, "acme:widgets").expect("registry"),
        "https://git.acme.com/widgets.git"
    );

    let gh_default = Config {
        default_source: Some("https://github.com".to_string()),
        ..Default::default()
    };
    let short_pkg = parse_app_spec("amkisko/scout-cli.rs:scout");
    assert_eq!(short_pkg.source, "amkisko/scout-cli.rs");
    assert_eq!(short_pkg.cargo_package.as_deref(), Some("scout"));
    assert_eq!(
        resolve_source(&gh_default, &short_pkg.source).expect("short path"),
        "https://github.com/amkisko/scout-cli.rs.git"
    );
}

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
    assert_eq!(cfg.checkout_backend, CheckoutBackend::Git);
    assert!(cfg.checkout_profiles.is_empty());
    assert!(!cfg.integrity_check);
    assert!(cfg.registries.is_empty());
}

#[test]
fn build_strategy_labels_are_stable() {
    assert_eq!(BuildStrategy::RustCargo.as_str(), "rust-cargo");
    assert_eq!(BuildStrategy::CMake.as_str(), "cmake");
    assert_eq!(BuildStrategy::Make.as_str(), "make");
    assert_eq!(BuildStrategy::Homebrew.as_str(), "homebrew");
    assert_eq!(BuildStrategy::Aur.as_str(), "aur");

    assert_eq!(BuildStrategy::parse("rust-cargo"), Some(BuildStrategy::RustCargo));
    assert_eq!(BuildStrategy::parse("make"), Some(BuildStrategy::Make));
    assert_eq!(BuildStrategy::parse("not-a-strategy"), None);
}

#[test]
fn build_isolation_labels_and_parse_are_stable() {
    assert_eq!(BuildIsolation::Off.as_str(), "off");
    assert_eq!(BuildIsolation::Auto.as_str(), "auto");
    assert_eq!(BuildIsolation::Docker.as_str(), "docker");
    assert_eq!(BuildIsolation::Podman.as_str(), "podman");
    assert_eq!(BuildIsolation::Nerdctl.as_str(), "nerdctl");

    assert_eq!(BuildIsolation::parse("off"), Some(BuildIsolation::Off));
    assert_eq!(BuildIsolation::parse("auto"), Some(BuildIsolation::Auto));
    assert_eq!(
        BuildIsolation::parse("docker"),
        Some(BuildIsolation::Docker)
    );
    assert_eq!(
        BuildIsolation::parse("podman"),
        Some(BuildIsolation::Podman)
    );
    assert_eq!(
        BuildIsolation::parse("nerdctl"),
        Some(BuildIsolation::Nerdctl)
    );
    assert_eq!(BuildIsolation::parse("invalid"), None);
}

#[test]
fn checkout_backend_labels_and_parse_are_stable() {
    assert_eq!(CheckoutBackend::Git.as_str(), "git");
    assert_eq!(CheckoutBackend::Gh.as_str(), "gh");
    assert_eq!(CheckoutBackend::Custom.as_str(), "custom");

    assert_eq!(CheckoutBackend::parse("git2"), Some(CheckoutBackend::Git));
    assert_eq!(CheckoutBackend::parse("git"), Some(CheckoutBackend::Git));
    assert_eq!(CheckoutBackend::parse("gh"), Some(CheckoutBackend::Gh));
    assert_eq!(
        CheckoutBackend::parse("custom"),
        Some(CheckoutBackend::Custom)
    );
    assert_eq!(CheckoutBackend::parse("invalid"), None);
}

#[test]
fn checkout_backend_git2_in_toml_deserializes_as_git() {
    let cfg: Config = toml::from_str(
        r#"default_source = "https://github.com"
checkout_backend = "git2"
"#,
    )
    .expect("parse");
    assert_eq!(cfg.checkout_backend, CheckoutBackend::Git);
}

#[test]
fn checkout_profile_resolution_applies_backend_auth_and_proxy() {
    let cfg = Config {
        default_source: Some("https://github.com".to_string()),
        checkout_profiles: vec![CheckoutProfile {
            name: Some("corp".to_string()),
            match_prefix: "https://github.com/acme/".to_string(),
            backend: Some(CheckoutBackend::Git),
            env: vec![EnvVar {
                key: "GH_TOKEN".to_string(),
                value: "abc".to_string(),
            }],
            ssh_command: Some("ssh -i ~/.ssh/acme".to_string()),
            http_proxy: Some("http://proxy.local:8080".to_string()),
            https_proxy: Some("http://proxy.local:8443".to_string()),
            all_proxy: None,
            no_proxy: Some("localhost,127.0.0.1".to_string()),
            custom_clone: None,
            custom_sync: None,
        }],
        ..Default::default()
    };

    let plan = resolve_checkout_plan(&cfg, "https://github.com/acme/tool.git").expect("plan");
    assert_eq!(plan.backend, CheckoutBackend::Git);
    assert!(plan.env.iter().any(|(k, v)| k == "GH_TOKEN" && v == "abc"));
    assert!(
        plan.env
            .iter()
            .any(|(k, v)| k == "GIT_SSH_COMMAND" && v.contains("~/.ssh/acme"))
    );
    assert!(
        plan.env
            .iter()
            .any(|(k, v)| k == "HTTP_PROXY" && v.contains("proxy.local"))
    );
    assert!(
        plan.env
            .iter()
            .any(|(k, v)| k == "NO_PROXY" && v.contains("localhost"))
    );
}

#[test]
#[serial]
fn isolation_backend_resolution_errors_without_available_backend() {
    with_var("PATH", Some(""), || {
        assert!(resolve_backend(BuildIsolation::Auto).is_err());
        assert!(resolve_backend(BuildIsolation::Docker).is_err());
    });
}

#[test]
fn parse_app_spec_handles_plain_url_and_ref_inputs() {
    let plain = parse_app_spec("tool");
    assert_eq!(plain.source, "tool");
    assert_eq!(plain.requested_ref, None);
    assert_eq!(plain.cargo_package, None);

    let semver = parse_app_spec("tool@^1.2");
    assert_eq!(semver.source, "tool");
    assert_eq!(semver.requested_ref.as_deref(), Some("^1.2"));
    assert_eq!(semver.cargo_package, None);

    let sha = parse_app_spec("file:///tmp/repo@deadbeef");
    assert_eq!(sha.source, "file:///tmp/repo");
    assert_eq!(sha.requested_ref.as_deref(), Some("deadbeef"));
    assert_eq!(sha.cargo_package, None);

    let ssh = parse_app_spec("git@github.com:acme/tool.git");
    assert_eq!(ssh.source, "git@github.com:acme/tool.git");
    assert_eq!(ssh.requested_ref, None);
    assert_eq!(ssh.cargo_package, None);

    let ssh_with_ref = parse_app_spec("git@github.com:acme/tool.git@v1.0.0");
    assert_eq!(ssh_with_ref.source, "git@github.com:acme/tool.git");
    assert_eq!(ssh_with_ref.requested_ref.as_deref(), Some("v1.0.0"));
    assert_eq!(ssh_with_ref.cargo_package, None);

    let https_with_user = parse_app_spec("https://user@example.com/acme/tool.git");
    assert_eq!(
        https_with_user.source,
        "https://user@example.com/acme/tool.git"
    );
    assert_eq!(https_with_user.requested_ref, None);
    assert_eq!(https_with_user.cargo_package, None);

    let trailing_at = parse_app_spec("tool@");
    assert_eq!(trailing_at.source, "tool@");
    assert_eq!(trailing_at.requested_ref, None);
    assert_eq!(trailing_at.cargo_package, None);

    let https_pkg = parse_app_spec("https://github.com/acme/foo.rs:bar");
    assert_eq!(https_pkg.source, "https://github.com/acme/foo.rs");
    assert_eq!(https_pkg.cargo_package.as_deref(), Some("bar"));
    assert_eq!(layout_id(&https_pkg), "foo-rs__bar");

    let https_pkg_ref = parse_app_spec("https://github.com/acme/foo:bar@main");
    assert_eq!(https_pkg_ref.source, "https://github.com/acme/foo");
    assert_eq!(https_pkg_ref.cargo_package.as_deref(), Some("bar"));
    assert_eq!(https_pkg_ref.requested_ref.as_deref(), Some("main"));

    let ssh_pkg = parse_app_spec("git@github.com:acme/tool.git:scout");
    assert_eq!(ssh_pkg.source, "git@github.com:acme/tool.git");
    assert_eq!(ssh_pkg.cargo_package.as_deref(), Some("scout"));

    let path_pkg_ref = parse_app_spec("acme/foo.rs:bar@v1");
    assert_eq!(path_pkg_ref.source, "acme/foo.rs");
    assert_eq!(path_pkg_ref.cargo_package.as_deref(), Some("bar"));
    assert_eq!(path_pkg_ref.requested_ref.as_deref(), Some("v1"));
}

#[test]
fn parse_app_spec_does_not_treat_registry_colons_as_cargo_package() {
    let reg = parse_app_spec("corp:widgets/extra");
    assert_eq!(reg.source, "corp:widgets/extra");
    assert_eq!(reg.cargo_package, None);
}
