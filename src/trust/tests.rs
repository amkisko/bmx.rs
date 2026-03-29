use std::path::Path;
use std::process::Command;

use crate::io::write_toml;
use crate::types::InstallMetadata;

use super::commands::{
    TrustListScope, add_allowed_signing_key, import_signing_keys_from_repo, list_policy, set_allow,
    set_require_signed_commit,
};
use super::enforce::enforce_source_trust;
use super::git_env::{trust_allowed_signers_path, trust_git_env};
use super::policy::{
    TrustPolicy, TrustRule, append_missing_keys, best_rule, load_policy_or_default,
    mutable_rule_for_match_prefix, save_policy,
};

fn run_git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo)
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.email=test@example.com",
            "-c",
            "user.name=BMX Test",
        ])
        .args(args)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {:?} failed", args);
}

fn init_unsigned_repo(root: &Path, name: &str) -> std::path::PathBuf {
    let repo = root.join(name);
    std::fs::create_dir_all(&repo).unwrap();
    run_git(&repo, &["init"]);
    std::fs::write(repo.join("README.md"), "x").unwrap();
    run_git(&repo, &["add", "."]);
    run_git(&repo, &["commit", "-m", "init"]);
    repo
}

#[test]
fn best_rule_prefers_longest_prefix() {
    let policy = TrustPolicy {
        default: TrustRule::default(),
        rules: vec![
            TrustRule {
                match_prefix: Some("https://github.com/".to_string()),
                allow: true,
                require_signed_commit: false,
                allowed_signing_keys: vec![],
            },
            TrustRule {
                match_prefix: Some("https://github.com/acme/".to_string()),
                allow: false,
                require_signed_commit: false,
                allowed_signing_keys: vec![],
            },
        ],
    };

    let r = best_rule(&policy, "https://github.com/acme/tool.git");
    assert!(!r.allow);
}

#[test]
fn mutable_rule_creates_and_updates_rule() {
    let mut policy = TrustPolicy::default();
    {
        let r = mutable_rule_for_match_prefix(&mut policy, "https://example.com/");
        r.allow = false;
    }
    assert_eq!(policy.rules.len(), 1);
    assert_eq!(
        policy.rules[0].match_prefix.as_deref(),
        Some("https://example.com/")
    );
    assert!(!policy.rules[0].allow);
}

#[test]
fn trust_git_env_creates_local_allowed_signers_file() {
    let home = tempfile::tempdir().expect("tempdir");
    let source = "https://github.com/acme/tool.git";
    let env = trust_git_env(home.path(), source).expect("trust_git_env");
    let path = trust_allowed_signers_path(home.path(), source);
    assert!(path.exists());
    assert!(env.iter().any(|(k, _)| k == "GIT_CONFIG_COUNT"));
    assert!(env.iter().any(|(k, _)| k == "GIT_CONFIG_KEY_0"));
    assert!(env.iter().any(|(k, _)| k == "GIT_CONFIG_VALUE_0"));
}

#[test]
fn trust_allowed_signers_path_is_source_scoped() {
    let home = tempfile::tempdir().expect("tempdir");
    let a = trust_allowed_signers_path(home.path(), "https://github.com/acme/a.git");
    let b = trust_allowed_signers_path(home.path(), "https://github.com/acme/b.git");
    assert_ne!(a, b);
}

#[test]
fn add_key_set_allow_and_set_signed_roundtrip() {
    let home = tempfile::tempdir().unwrap();
    let prefix = "https://github.com/acme/";
    add_allowed_signing_key(home.path(), "abcd1234", Some(prefix)).unwrap();
    add_allowed_signing_key(home.path(), "abcd1234", Some(prefix)).unwrap(); // dedupe
    set_allow(home.path(), prefix, false).unwrap();
    set_require_signed_commit(home.path(), prefix, true).unwrap();
    let policy = load_policy_or_default(home.path()).unwrap();
    let r = best_rule(&policy, "https://github.com/acme/tool.git");
    assert_eq!(r.allowed_signing_keys.len(), 1);
    assert!(!r.allow);
    assert!(r.require_signed_commit);
}

#[test]
fn enforce_source_trust_blocked_and_signed_requirements() {
    let home = tempfile::tempdir().unwrap();
    let repos = tempfile::tempdir().unwrap();
    let repo = init_unsigned_repo(repos.path(), "r1");

    set_allow(home.path(), "file://", false).unwrap();
    let e = enforce_source_trust(home.path(), "file://x", &repo).unwrap_err();
    assert!(e.to_string().contains("blocked by trust policy"));

    set_allow(home.path(), "file://", true).unwrap();
    set_require_signed_commit(home.path(), "file://", true).unwrap();
    assert!(enforce_source_trust(home.path(), "file://x", &repo).is_err());
}

#[test]
fn import_signing_keys_from_unsigned_repo_fails() {
    let home = tempfile::tempdir().unwrap();
    let repos = tempfile::tempdir().unwrap();
    let repo = init_unsigned_repo(repos.path(), "r2");
    assert!(
        import_signing_keys_from_repo(home.path(), "file://x", &repo, Some("file://x")).is_err()
    );
}

#[test]
fn list_policy_for_source_without_local_rules_reports_global_effective_scope() {
    let home = tempfile::tempdir().unwrap();
    let mut policy = TrustPolicy::default();
    policy.default.allowed_signing_keys = vec!["GLOBAL".into()];
    save_policy(home.path(), &policy).unwrap();
    let out = list_policy(
        home.path(),
        TrustListScope::All,
        Some("https://example.com/tool.git"),
    )
    .unwrap();
    assert!(out.contains("effective_scope: global"));
}

#[test]
fn list_policy_scopes_and_app_filter() {
    let home = tempfile::tempdir().expect("home");
    std::fs::create_dir_all(home.path()).unwrap();
    let mut policy = TrustPolicy::default();
    policy.default.allowed_signing_keys = vec!["GLOBAL".into()];
    policy.rules.push(TrustRule {
        match_prefix: Some("https://github.com/acme/".into()),
        allow: true,
        require_signed_commit: true,
        allowed_signing_keys: vec!["LOCAL".into()],
    });
    save_policy(home.path(), &policy).unwrap();

    let all = list_policy(home.path(), TrustListScope::All, None).unwrap();
    assert!(all.contains("scope: global"));
    assert!(all.contains("scope: local"));

    let global = list_policy(home.path(), TrustListScope::Global, None).unwrap();
    assert!(global.contains("GLOBAL"));
    assert!(!global.contains("scope: local"));

    let local = list_policy(home.path(), TrustListScope::Local, None).unwrap();
    assert!(local.contains("scope: local"));
    assert!(!local.contains("scope: global"));

    let layout = crate::layout::app_layout_for_id(home.path(), "my-app");
    std::fs::create_dir_all(layout.repo_dir.parent().unwrap()).unwrap();
    write_toml(
        &layout.meta_file,
        &InstallMetadata {
            app: "my-app".into(),
            source_url: "https://github.com/acme/tool.git".into(),
            executable_rel: "bin/tool".into(),
            strategy: "make".into(),
            requested_ref: None,
            resolved_commit: None,
            rust_package: None,
        },
    )
    .unwrap();
    let filtered = list_policy(home.path(), TrustListScope::All, Some("my-app")).unwrap();
    assert!(filtered.contains("effective_scope: local"));
    assert!(filtered.contains("LOCAL"));
}

#[test]
fn append_missing_keys_counts_only_new_values() {
    let mut rule = TrustRule {
        match_prefix: None,
        allow: true,
        require_signed_commit: false,
        allowed_signing_keys: vec!["ABCD1234".into()],
    };
    let keys = vec!["ABCD1234".to_string(), "EFGH5678".to_string()];
    let added = append_missing_keys(&mut rule, &keys);
    assert_eq!(added, 1);
    assert_eq!(rule.allowed_signing_keys.len(), 2);
    assert!(rule.allowed_signing_keys.iter().any(|k| k == "ABCD1234"));
    assert!(rule.allowed_signing_keys.iter().any(|k| k == "EFGH5678"));
}

#[test]
fn save_policy_handles_fingerprint_and_quoted_values() {
    let home = tempfile::tempdir().expect("home");
    let mut policy = TrustPolicy::default();
    policy.default.allowed_signing_keys = vec![
        "SHA256:6N4WHCBUBFBLXJ1PB+JCWNANCC0NDIX/TYCUDJRVDJO".into(),
        "KEY\"WITH\\ESCAPES".into(),
    ];
    policy.rules.push(TrustRule {
        match_prefix: Some("https://github.com/acme/tool.git".into()),
        allow: true,
        require_signed_commit: true,
        allowed_signing_keys: vec!["ABCD1234".into()],
    });

    save_policy(home.path(), &policy).expect("save policy");
    let loaded = load_policy_or_default(home.path()).expect("reload policy");
    assert_eq!(
        loaded.default.allowed_signing_keys,
        policy.default.allowed_signing_keys
    );
    assert_eq!(loaded.rules.len(), 1);
    assert_eq!(
        loaded.rules[0].match_prefix.as_deref(),
        Some("https://github.com/acme/tool.git")
    );
}
