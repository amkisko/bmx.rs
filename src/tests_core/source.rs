use crate::app_spec::parse_app_spec;
use crate::source::{
    app_id, looks_like_url, normalize_explicit_url, normalize_source_base, resolve_source,
    sanitize_bin_name,
};
use crate::types::Config;

#[test]
fn source_helpers_cover_normalization_and_ids() {
    assert!(looks_like_url("git@github.com:acme/tool.git"));
    assert!(looks_like_url("bitbucket.org/workspace/tool"));
    assert!(looks_like_url("codeberg.org/acme/tool"));
    assert!(looks_like_url("git.sr.ht/~acme/tool"));
    assert!(looks_like_url("gitea.example.com/acme/tool"));
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
        registries: [("acme".to_string(), "https://git.acme.com".to_string())]
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
    assert_eq!(
        resolve_source(&gh_default, "codeberg.org/acme/tool").expect("explicit host shortcut"),
        "https://codeberg.org/acme/tool"
    );
    assert_eq!(
        resolve_source(&gh_default, "bitbucket.org/acme/tool").expect("bitbucket explicit host"),
        "https://bitbucket.org/acme/tool"
    );
}
