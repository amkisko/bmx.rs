use serde_json::Value as JsonValue;
use url::Url;

use crate::types::Config;

use super::github::github_contents_has_marker;
use super::helpers::{config_with_gitlab_base, github_qualifiers, gitlab_api_v4_root};
use super::http::{clip, github_headers, gitlab_headers};
use super::registries::{
    AurRpcResponse, brew_formula_to_install_spec, extract_git_clone_from_archive_url,
    homepage_to_git_clone_spec, install_hint_resolves, looks_like_any_git_remote,
};

#[allow(unsafe_code)]
fn with_env_var<F: FnOnce()>(name: &str, value: Option<&str>, f: F) {
    let old = std::env::var(name).ok();
    match value {
        Some(v) => unsafe { std::env::set_var(name, v) },
        None => unsafe { std::env::remove_var(name) },
    }
    f();
    if let Some(v) = old {
        unsafe { std::env::set_var(name, v) };
    } else {
        unsafe { std::env::remove_var(name) };
    }
}

#[test]
fn github_qualifiers_adds_archived_and_fork() {
    assert_eq!(
        github_qualifiers("ripgrep", false),
        "ripgrep archived:false fork:false"
    );
    assert_eq!(github_qualifiers("ripgrep", true), "ripgrep archived:false");
}

#[test]
fn github_contents_detects_cargo() {
    let body = r#"[{"name":"Cargo.toml","type":"file"},{"name":"README.md","type":"file"}]"#;
    assert!(github_contents_has_marker(body));
}

#[test]
fn github_contents_negative() {
    let body = r#"[{"name":"README.md","type":"file"}]"#;
    assert!(!github_contents_has_marker(body));
}

#[test]
fn homepage_to_git_clone_spec_github() {
    assert_eq!(
        homepage_to_git_clone_spec("https://github.com/BurntSushi/ripgrep"),
        Some("https://github.com/BurntSushi/ripgrep.git".into())
    );
}

#[test]
fn extract_git_from_archive() {
    assert_eq!(
        extract_git_clone_from_archive_url(
            "https://github.com/BurntSushi/ripgrep/archive/refs/tags/14.1.0.tar.gz"
        ),
        Some("https://github.com/BurntSushi/ripgrep.git".into())
    );
}

#[test]
fn parses_aur_rpc() {
    let j = r#"{"resultcount":1,"results":[{"Name":"yay","PackageBase":"yay","Version":"1-1"}]}"#;
    let p: AurRpcResponse = serde_json::from_str(j).unwrap();
    assert_eq!(p.results[0].package_base, "yay");
}

#[test]
fn brew_formula_conversion_uses_homepage_or_archive() {
    let f: JsonValue = serde_json::from_str(
        r#"{"name":"ripgrep","homepage":"https://github.com/BurntSushi/ripgrep"}"#,
    )
    .unwrap();
    assert_eq!(
        brew_formula_to_install_spec(&f).as_deref(),
        Some("https://github.com/BurntSushi/ripgrep.git")
    );

    let f2: JsonValue = serde_json::from_str(
        r#"{"name":"rg","urls":{"stable":{"url":"https://github.com/BurntSushi/ripgrep/archive/refs/tags/1.tar.gz"}}}"#,
    )
    .unwrap();
    assert_eq!(
        brew_formula_to_install_spec(&f2).as_deref(),
        Some("https://github.com/BurntSushi/ripgrep.git")
    );
}

#[test]
fn homepage_to_git_clone_spec_gitlab_and_invalid() {
    assert_eq!(
        homepage_to_git_clone_spec("https://gitlab.com/acme/tool"),
        Some("https://gitlab.com/acme/tool.git".into())
    );
    assert_eq!(
        homepage_to_git_clone_spec("https://example.com/acme/tool"),
        None
    );
}

#[test]
fn github_contents_and_clip_helpers_cover_edge_cases() {
    assert!(!github_contents_has_marker("not-json"));
    assert_eq!(clip(&"a".repeat(300)).len(), 200);
}

#[test]
fn gitlab_root_and_config_helpers() {
    let u = Url::parse("https://gitlab.example:8443/group").unwrap();
    assert_eq!(
        gitlab_api_v4_root(&u),
        "https://gitlab.example:8443/api/v4".to_string()
    );
    let cfg = Config::default();
    let cfg2 = config_with_gitlab_base(&cfg, &u);
    assert_eq!(
        cfg2.default_source.as_deref(),
        Some("https://gitlab.example:8443")
    );
}

#[test]
fn source_and_remote_detection_helpers() {
    let cfg = Config {
        default_source: Some("https://github.com".into()),
        ..Default::default()
    };
    assert!(install_hint_resolves(&cfg, "owner/repo").unwrap());
    assert!(looks_like_any_git_remote("https://example.com/x.git"));
    assert!(looks_like_any_git_remote("git@github.com:a/b.git"));
    assert!(!looks_like_any_git_remote("ftp://example.com/x"));
}

#[test]
fn header_helpers_include_tokens_when_present() {
    with_env_var("GITHUB_TOKEN", Some("abc123"), || {
        let u = Url::parse("https://api.github.com/search/repositories").unwrap();
        let h = github_headers(&u);
        assert!(
            h.iter()
                .any(|(k, v)| *k == "Authorization" && v.contains("abc123"))
        );
    });
    with_env_var("GITLAB_TOKEN", Some("tok"), || {
        let u = Url::parse("https://gitlab.com/api/v4/projects").unwrap();
        let h = gitlab_headers(&u);
        assert!(h.iter().any(|(k, v)| *k == "PRIVATE-TOKEN" && v == "tok"));
    });
}
