use crate::repo::resolve_checkout_plan;
use crate::types::{
    BuildIsolation, BuildStrategy, CheckoutBackend, CheckoutProfile, Config, EnvVar,
};

#[test]
fn build_strategy_labels_are_stable() {
    assert_eq!(BuildStrategy::RustCargo.as_str(), "rust-cargo");
    assert_eq!(BuildStrategy::CMake.as_str(), "cmake");
    assert_eq!(BuildStrategy::Make.as_str(), "make");
    assert_eq!(BuildStrategy::Homebrew.as_str(), "homebrew");
    assert_eq!(BuildStrategy::Aur.as_str(), "aur");

    assert_eq!(
        BuildStrategy::parse("rust-cargo"),
        Some(BuildStrategy::RustCargo)
    );
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
