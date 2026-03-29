//! Integration tests for install/run **controls**: ephemeral cache (`--rm`), project pin (`.bmx/pin`),
//! verbose resolution (`-v`), `integrity_check`, `[registries]` aliases, `bmx.toml` hooks, and `shim` subcommands.
//! Complements `cli.rs` (core workflows) and `version_pinning_cli.rs` (ref pinning).
use std::fs;
use std::path::Path;
use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

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
        .expect("git command should run");
    assert!(status.success(), "git {:?} should succeed", args);
}

fn init_make_repo(root: &Path, repo_name: &str, bin_name: &str, msg: &str) -> std::path::PathBuf {
    let repo = root.join(repo_name);
    fs::create_dir_all(&repo).expect("create repo dir");
    let makefile = format!(
        "all:\n\tmkdir -p bin\n\tprintf '#!/bin/sh\\necho {}\\n' > bin/{}\n\tchmod +x bin/{}\n",
        msg, bin_name, bin_name
    );
    fs::write(repo.join("Makefile"), makefile).expect("write Makefile");
    fs::write(
        repo.join("bmx.toml"),
        format!("[bmx]\nrun = \"bin/{}\"\n", bin_name),
    )
    .expect("write bmx.toml");
    run_git(&repo, &["init"]);
    run_git(&repo, &["config", "user.email", "test@example.com"]);
    run_git(&repo, &["config", "user.name", "BMX Test"]);
    run_git(&repo, &["add", "."]);
    run_git(&repo, &["commit", "-m", "init"]);
    repo
}

fn init_repo_with_post_install(
    root: &Path,
    repo_name: &str,
    bin_name: &str,
    msg: &str,
    post_install: &str,
) -> std::path::PathBuf {
    let repo = root.join(repo_name);
    fs::create_dir_all(&repo).expect("create repo dir");
    let makefile = format!(
        "all:\n\tmkdir -p bin\n\tprintf '#!/bin/sh\\necho {}\\n' > bin/{}\n\tchmod +x bin/{}\n",
        msg, bin_name, bin_name
    );
    fs::write(repo.join("Makefile"), makefile).expect("write Makefile");
    let manifest = format!(
        "[bmx]\nrun = \"bin/{bin_name}\"\n\n[bmx.hooks]\npost_install = \"{post_install}\"\n"
    );
    fs::write(repo.join("bmx.toml"), manifest).expect("write bmx.toml");
    run_git(&repo, &["init"]);
    run_git(&repo, &["config", "user.email", "test@example.com"]);
    run_git(&repo, &["config", "user.name", "BMX Test"]);
    run_git(&repo, &["add", "."]);
    run_git(&repo, &["commit", "-m", "init"]);
    repo
}

fn app_id_for_file_url(app: &str) -> String {
    let base = app.trim_start_matches("file://");
    base.trim_end_matches(".git")
        .rsplit('/')
        .next()
        .unwrap_or(base)
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn bmx(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("bmx").expect("binary should build");
    cmd.env("HOME", home);
    cmd.env("BMX_TRUST_ASSUME_YES", "1");
    cmd
}

#[test]
fn rm_flag_uses_ephemeral_home_not_dot_bmx_under_home() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "rm-tool", "rm-tool", "RM_OK");
    let app = format!("file://{}", repo.display());

    bmx(home.path())
        .args(["--rm", "exec", &app])
        .assert()
        .success()
        .stdout(predicate::str::contains("RM_OK"));

    assert!(
        !home.path().join(".bmx").exists(),
        "--rm must not create a persistent ~/.bmx under HOME"
    );
}

#[test]
fn pin_resolves_app_from_bmx_pin_in_parent_directory() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "pin-app", "pin-app", "PIN_OK");
    let app = format!("file://{}", repo.display());

    let project = TempDir::new().expect("project");
    fs::create_dir_all(project.path().join(".bmx")).expect("pin dir");
    fs::write(project.path().join(".bmx/pin"), format!("{app}\n")).expect("write pin");

    bmx(home.path())
        .current_dir(project.path())
        .args(["exec", "--pin"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PIN_OK"));
}

#[test]
fn pin_with_explicit_app_writes_cwd_pin_and_runs_exec() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "pin-write-exec", "pin-write-exec", "PW_EXEC");
    let app = format!("file://{}", repo.display());

    let project = TempDir::new().expect("project");

    bmx(home.path())
        .current_dir(project.path())
        .args(["exec", "--pin", &app])
        .assert()
        .success()
        .stdout(predicate::str::contains("PW_EXEC"));

    let pins_path = project.path().join(".bmx/pins.toml");
    assert!(pins_path.is_file(), "expected {}", pins_path.display());
    let raw = fs::read_to_string(&pins_path).expect("read pins.toml");
    assert!(
        raw.contains(&app),
        "pins.toml should contain spec; got:\n{raw}"
    );
}

#[test]
fn pin_with_explicit_app_writes_cwd_pin_default_command() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "pin-write-def", "pin-write-def", "PW_DEF");
    let app = format!("file://{}", repo.display());

    let project = TempDir::new().expect("project");

    bmx(home.path())
        .current_dir(project.path())
        .args([&app, "--pin"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PW_DEF"));

    let pins_path = project.path().join(".bmx/pins.toml");
    let raw = fs::read_to_string(&pins_path).expect("read pins.toml");
    assert!(
        raw.contains(&app),
        "pins.toml should contain spec; got:\n{raw}"
    );
}

#[test]
fn pin_second_app_merges_pins_toml_and_keeps_default() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let r1 = init_make_repo(repos.path(), "multi-a", "multi-a", "MULTI_A");
    let r2 = init_make_repo(repos.path(), "multi-b", "multi-b", "MULTI_B");
    let app_a = format!("file://{}", r1.display());
    let app_b = format!("file://{}", r2.display());

    let project = TempDir::new().expect("project");

    bmx(home.path())
        .current_dir(project.path())
        .args(["exec", "--pin", &app_a])
        .assert()
        .success()
        .stdout(predicate::str::contains("MULTI_A"));

    bmx(home.path())
        .current_dir(project.path())
        .args(["exec", "--pin", &app_b])
        .assert()
        .success()
        .stdout(predicate::str::contains("MULTI_B"));

    let pins_path = project.path().join(".bmx/pins.toml");
    let raw = fs::read_to_string(&pins_path).expect("read pins.toml");
    assert!(raw.contains(&app_a), "first spec preserved:\n{raw}");
    assert!(raw.contains(&app_b), "second spec present:\n{raw}");

    bmx(home.path())
        .current_dir(project.path())
        .args(["exec", "--pin"])
        .assert()
        .success()
        .stdout(predicate::str::contains("MULTI_A"));

    fs::write(
        &pins_path,
        fs::read_to_string(&pins_path)
            .expect("read")
            .replace("default = \"multi-a\"", "default = \"multi-b\""),
    )
    .expect("set default");

    bmx(home.path())
        .current_dir(project.path())
        .args(["exec", "--pin"])
        .assert()
        .success()
        .stdout(predicate::str::contains("MULTI_B"));
}

#[test]
fn verbose_flag_prints_resolution_line_to_stderr() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "verb-tool", "verb-tool", "VERB_OK");
    let app = format!("file://{}", repo.display());

    bmx(home.path())
        .args(["-v", "exec", &app])
        .assert()
        .success()
        .stdout(predicate::str::contains("VERB_OK"))
        .stderr(predicate::str::contains("[bmx] app="))
        .stderr(predicate::str::contains("executable="));
}

#[test]
fn post_install_hook_runs_after_install() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_repo_with_post_install(
        repos.path(),
        "hook-tool",
        "hook-tool",
        "HOOK_OK",
        "touch hook-ran-marker",
    );
    let app = format!("file://{}", repo.display());
    let app_id = app_id_for_file_url(&app);

    bmx(home.path()).args(["install", &app]).assert().success();

    let marker = home
        .path()
        .join(".bmx/apps")
        .join(&app_id)
        .join("repo/hook-ran-marker");
    assert!(
        marker.exists(),
        "post_install hook should create {}",
        marker.display()
    );
}

#[test]
fn registry_alias_resolves_via_config() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");

    let source_repo = init_make_repo(repos.path(), "reg-src", "regtool", "REGISTRY_OK");
    let bare = repos.path().join("regtool.git");
    let st = Command::new("git")
        .args([
            "clone",
            "--bare",
            source_repo.to_string_lossy().as_ref(),
            bare.to_string_lossy().as_ref(),
        ])
        .status()
        .expect("git");
    assert!(st.success());

    fs::create_dir_all(home.path().join(".bmx")).expect("bmx home");
    let base_url = format!("file://{}", repos.path().display());
    fs::write(
        home.path().join(".bmx/config.toml"),
        format!(
            r#"default_source = "https://github.com"
integrity_check = false

[registries]
loc = "{base_url}"
"#
        ),
    )
    .expect("config");

    bmx(home.path())
        .args(["install", "loc:regtool"])
        .assert()
        .success()
        .stdout(predicate::str::contains("installed"));

    bmx(home.path())
        .args(["exec", "loc:regtool"])
        .assert()
        .success()
        .stdout(predicate::str::contains("REGISTRY_OK"));
}

#[test]
fn integrity_check_fails_when_repo_head_diverges() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "int-tool", "int-tool", "INT_OK");
    let app = format!("file://{}", repo.display());

    bmx(home.path()).args(["install", &app]).assert().success();

    // Overwrite (do not append): serde may emit `[registries]` and later keys would nest wrongly.
    fs::write(
        home.path().join(".bmx/config.toml"),
        r#"default_source = "https://github.com"
build_isolation = "off"
checkout_backend = "git"
integrity_check = true
checkout_profiles = []
"#,
    )
    .expect("write config");

    bmx(home.path()).args(["exec", &app]).assert().success();

    let app_id = app_id_for_file_url(&app);
    let cached = home.path().join(".bmx/apps").join(&app_id).join("repo");
    run_git(&cached, &["commit", "--allow-empty", "-m", "drift"]);

    bmx(home.path())
        .args(["exec", &app])
        .assert()
        .failure()
        .stderr(predicate::str::contains("install cache integrity"));
}

#[cfg(unix)]
#[test]
fn shim_init_path_and_add_generate_executable_stub() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "shim-app", "shim-app", "SHIM_OK");
    let app = format!("file://{}", repo.display());

    bmx(home.path())
        .args(["shim", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PATH="));

    bmx(home.path()).args(["shim", "init"]).assert().success();

    bmx(home.path()).args(["install", &app]).assert().success();

    bmx(home.path())
        .args(["shim", "add", &app])
        .assert()
        .success();

    let shim = home.path().join(".bmx/shims/shim-app");
    assert!(shim.is_file(), "shim file exists");
    let st = Command::new("sh")
        .current_dir(repos.path())
        .env("HOME", home.path())
        .arg(&shim)
        .arg("x")
        .stdout(std::process::Stdio::null())
        .status()
        .expect("run shim");
    assert!(st.success(), "shim should execute bmx exec");
}

#[test]
fn rm_flag_with_shim_does_not_create_persistent_home_state() {
    let home = TempDir::new().expect("home");

    bmx(home.path())
        .args(["--rm", "shim", "init"])
        .assert()
        .success();

    assert!(
        !home.path().join(".bmx").exists(),
        "--rm must not create a persistent ~/.bmx under HOME"
    );
}

#[test]
fn trust_policy_can_block_unapproved_source() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "blocked-tool", "blocked-tool", "BLOCKED");
    let app = format!("file://{}", repo.display());

    fs::create_dir_all(home.path().join(".bmx")).expect("bmx home");
    fs::write(
        home.path().join(".bmx/trust.toml"),
        "[default]\nallow = false\n",
    )
    .expect("write trust policy");

    bmx(home.path())
        .args(["install", &app])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "source is blocked by trust policy",
        ));
}

#[test]
fn trust_policy_allows_explicit_prefix_when_default_denies() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "allowed-tool", "allowed-tool", "ALLOWED");
    let app = format!("file://{}", repo.display());

    fs::create_dir_all(home.path().join(".bmx")).expect("bmx home");
    let trust = format!(
        "[default]\nallow = false\n\n[[rules]]\nmatch_prefix = \"file://{}\"\nallow = true\n",
        repos.path().display()
    );
    fs::write(home.path().join(".bmx/trust.toml"), trust).expect("write trust policy");

    bmx(home.path()).args(["install", &app]).assert().success();
    bmx(home.path())
        .args(["exec", &app])
        .assert()
        .success()
        .stdout(predicate::str::contains("ALLOWED"));
}

#[test]
fn trust_add_key_and_show_roundtrip() {
    let home = TempDir::new().expect("home");

    bmx(home.path())
        .args([
            "trust",
            "add-key",
            "ABCD1234",
            "--match-prefix",
            "https://github.com/acme/",
        ])
        .assert()
        .success();

    bmx(home.path())
        .args(["trust", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("https://github.com/acme/"))
        .stdout(predicate::str::contains("ABCD1234"));
}

#[test]
fn trust_list_supports_global_and_local_filters() {
    let home = TempDir::new().expect("home");
    fs::create_dir_all(home.path().join(".bmx")).expect("bmx home");
    fs::write(
        home.path().join(".bmx/trust.toml"),
        "[default]\nallow = true\nallowed_signing_keys = [\"GLOBALKEY\"]\n\n[[rules]]\nmatch_prefix = \"https://github.com/acme/\"\nallow = true\nallowed_signing_keys = [\"LOCALKEY\"]\n",
    )
    .expect("write trust policy");

    bmx(home.path())
        .args(["trust", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("scope: global"))
        .stdout(predicate::str::contains("scope: local"));

    bmx(home.path())
        .args(["--global", "trust", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("scope: global"))
        .stdout(predicate::str::contains("GLOBALKEY"))
        .stdout(predicate::str::contains("scope: local").not());

    bmx(home.path())
        .args(["trust", "list", "--local"])
        .assert()
        .success()
        .stdout(predicate::str::contains("scope: local"))
        .stdout(predicate::str::contains("LOCALKEY"))
        .stdout(predicate::str::contains("scope: global").not());
}

#[test]
fn trust_list_can_filter_by_installed_app_source() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "trust-list-app", "trust-list-app", "TL");
    let app = format!("file://{}", repo.display());
    bmx(home.path()).args(["install", &app]).assert().success();

    fs::create_dir_all(home.path().join(".bmx")).expect("bmx home");
    let trust = format!(
        "[default]\nallow = true\n\n[[rules]]\nmatch_prefix = \"file://{}\"\nallow = true\nallowed_signing_keys = [\"MATCH\"]\n\n[[rules]]\nmatch_prefix = \"https://github.com/other/\"\nallow = true\nallowed_signing_keys = [\"OTHER\"]\n",
        repos.path().display()
    );
    fs::write(home.path().join(".bmx/trust.toml"), trust).expect("write trust policy");

    bmx(home.path())
        .args(["trust", "list", &app])
        .assert()
        .success()
        .stdout(predicate::str::contains("source:"))
        .stdout(predicate::str::contains("MATCH"))
        .stdout(predicate::str::contains("OTHER").not())
        .stdout(predicate::str::contains("effective_scope: local"));
}

#[test]
fn trust_import_repo_fails_for_unsigned_head_commit() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "unsigned-tool", "unsigned-tool", "U");
    let app = format!("file://{}", repo.display());

    bmx(home.path()).args(["install", &app]).assert().success();

    bmx(home.path())
        .args(["trust", "import-repo", &app])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "no commit signing key/fingerprint found",
        ));
}

#[test]
fn trust_import_repo_from_source_without_install_attempts_temp_checkout() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "unsigned-src", "unsigned-src", "U");
    let app = format!("file://{}", repo.display());

    bmx(home.path())
        .args(["trust", "import-repo", &app])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "no commit signing key/fingerprint found",
        ))
        .stderr(predicate::str::contains("not installed").not());
}

#[test]
fn install_fails_without_interactive_consent_for_untrusted_unsigned_source() {
    let home = TempDir::new().expect("home");
    let repos = TempDir::new().expect("repos");
    let repo = init_make_repo(repos.path(), "consent-tool", "consent-tool", "C");
    let app = format!("file://{}", repo.display());

    let mut cmd = Command::cargo_bin("bmx").expect("binary should build");
    cmd.env("HOME", home.path());
    cmd.args(["install", &app])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "untrusted source requires interactive consent",
        ));
}
