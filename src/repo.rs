use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};

use crate::git_cmd::{git_output, git_run};
use crate::process::has_tool;
use crate::runtime::{debug_log, subprocess_output_visible};
use crate::types::{CheckoutBackend, Config};

pub(crate) fn sync_repo(
    source_url: &str,
    repo_dir: &Path,
    cfg: &Config,
    cli_verbose: bool,
) -> Result<()> {
    let loud = subprocess_output_visible(cli_verbose);
    let plan = resolve_checkout_plan(cfg, source_url)?;
    let quiet_checkout = !loud;
    if repo_dir.join(".git").exists() {
        if loud {
            eprintln!("[bmx] syncing repository updates");
        }
        sync_existing_repo(repo_dir, &plan, quiet_checkout)?;
        return Ok(());
    }

    if repo_dir.read_dir()?.next().is_some() {
        bail!(
            "target repo dir {} is not empty and not a git repo",
            repo_dir.display()
        );
    }

    if loud {
        eprintln!("[bmx] cloning repository");
    }
    clone_repo(source_url, repo_dir, &plan, quiet_checkout)
}

#[derive(Debug, Clone)]
pub(crate) struct CheckoutPlan {
    pub(crate) backend: CheckoutBackend,
    pub(crate) env: Vec<(String, String)>,
    pub(crate) custom_clone: Option<String>,
    pub(crate) custom_sync: Option<String>,
}

pub(crate) fn resolve_checkout_plan(cfg: &Config, source_url: &str) -> Result<CheckoutPlan> {
    let mut selected = None;
    for profile in &cfg.checkout_profiles {
        if source_url.starts_with(&profile.match_prefix) {
            selected = Some(profile);
            break;
        }
    }

    let backend = selected
        .and_then(|profile| profile.backend)
        .unwrap_or(cfg.checkout_backend);
    let mut env = Vec::new();

    if let Some(profile) = selected {
        for pair in &profile.env {
            env.push((pair.key.clone(), pair.value.clone()));
        }
        if let Some(value) = &profile.ssh_command {
            env.push(("GIT_SSH_COMMAND".to_string(), value.clone()));
        }
        if let Some(value) = &profile.http_proxy {
            env.push(("HTTP_PROXY".to_string(), value.clone()));
        }
        if let Some(value) = &profile.https_proxy {
            env.push(("HTTPS_PROXY".to_string(), value.clone()));
        }
        if let Some(value) = &profile.all_proxy {
            env.push(("ALL_PROXY".to_string(), value.clone()));
        }
        if let Some(value) = &profile.no_proxy {
            env.push(("NO_PROXY".to_string(), value.clone()));
        }
    }

    let custom_clone = selected.and_then(|profile| profile.custom_clone.clone());
    let custom_sync = selected.and_then(|profile| profile.custom_sync.clone());
    Ok(CheckoutPlan {
        backend,
        env,
        custom_clone,
        custom_sync,
    })
}

fn clone_repo(source_url: &str, repo_dir: &Path, plan: &CheckoutPlan, quiet: bool) -> Result<()> {
    debug_log(&format!(
        "clone from {source_url} to {} via {}",
        repo_dir.display(),
        plan.backend.as_str()
    ));
    match plan.backend {
        CheckoutBackend::Git => git_cli_clone(source_url, repo_dir, &plan.env, quiet),
        CheckoutBackend::Gh => gh_clone(source_url, repo_dir, &plan.env, quiet),
        CheckoutBackend::Custom => custom_clone(source_url, repo_dir, plan),
    }
}

fn sync_existing_repo(repo_dir: &Path, plan: &CheckoutPlan, quiet: bool) -> Result<()> {
    match plan.backend {
        CheckoutBackend::Git => git_cli_fetch_and_ff(repo_dir, &plan.env, quiet),
        CheckoutBackend::Gh => git_cli_sync(repo_dir, &plan.env, quiet),
        CheckoutBackend::Custom => custom_sync(repo_dir, plan),
    }
}

fn git_cli_clone(
    source_url: &str,
    repo_dir: &Path,
    env: &[(String, String)],
    quiet: bool,
) -> Result<()> {
    if !has_tool("git") {
        bail!("this checkout backend requires `git` to be installed");
    }
    let mut args = vec![
        "clone".to_string(),
        source_url.to_string(),
        repo_dir.to_string_lossy().to_string(),
    ];
    if quiet {
        args.insert(1, "--quiet".to_string());
    }
    run_status(None, "git", &args, env)
}

fn gh_clone(
    source_url: &str,
    repo_dir: &Path,
    env: &[(String, String)],
    quiet: bool,
) -> Result<()> {
    if !has_tool("gh") {
        bail!("checkout backend gh requires `gh` to be installed");
    }
    let repo = github_repo_slug(source_url).ok_or_else(|| {
        anyhow!("checkout backend gh supports only GitHub repositories; source was {source_url}")
    })?;
    let mut args = vec![
        "repo".to_string(),
        "clone".to_string(),
        repo,
        repo_dir.to_string_lossy().to_string(),
    ];
    if quiet {
        args.push("--".to_string());
        args.push("--quiet".to_string());
    }
    run_status(None, "gh", &args, env)
}

fn custom_clone(source_url: &str, repo_dir: &Path, plan: &CheckoutPlan) -> Result<()> {
    let command = plan
        .custom_clone
        .as_deref()
        .ok_or_else(|| anyhow!("custom checkout backend requires profile.custom_clone"))?;
    let rendered = render_custom(command, source_url, repo_dir);
    run_status(None, "sh", &["-lc".to_string(), rendered], &plan.env)
}

fn custom_sync(repo_dir: &Path, plan: &CheckoutPlan) -> Result<()> {
    let command = plan
        .custom_sync
        .as_deref()
        .ok_or_else(|| anyhow!("custom checkout backend requires profile.custom_sync"))?;
    let rendered = render_custom(command, "", repo_dir);
    run_status(
        Some(repo_dir),
        "sh",
        &["-lc".to_string(), rendered],
        &plan.env,
    )
}

fn git_cli_sync(repo_dir: &Path, env: &[(String, String)], quiet: bool) -> Result<()> {
    if !has_tool("git") {
        bail!("this checkout backend requires `git` to be installed");
    }
    let mut fetch = vec![
        "fetch".to_string(),
        "--all".to_string(),
        "--prune".to_string(),
    ];
    if quiet {
        fetch.insert(1, "-q".to_string());
    }
    run_status(Some(repo_dir), "git", &fetch, env)?;
    let mut pull = vec![
        "pull".to_string(),
        "--ff-only".to_string(),
        "origin".to_string(),
        "HEAD".to_string(),
    ];
    if quiet {
        pull.insert(1, "-q".to_string());
    }
    run_status(Some(repo_dir), "git", &pull, env)
}

fn run_status(
    cwd: Option<&Path>,
    cmd: &str,
    args: &[String],
    env: &[(String, String)],
) -> Result<()> {
    let preview = format!("{cmd} {}", args.join(" "));
    debug_log(&format!("repo command: {preview}"));
    let mut command = Command::new(cmd);
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    command.args(args);
    for (key, value) in env {
        command.env(key, value);
    }
    let status = command
        .status()
        .with_context(|| format!("failed to execute {preview}"))?;
    if !status.success() {
        bail!("command failed: {preview}");
    }
    Ok(())
}

fn render_custom(template: &str, source_url: &str, repo_dir: &Path) -> String {
    template
        .replace("{source_url}", &shell_quote(source_url))
        .replace("{repo_dir}", &shell_quote(&repo_dir.to_string_lossy()))
}

fn shell_quote(input: &str) -> String {
    if input.is_empty() {
        return "''".to_string();
    }
    let mut out = String::with_capacity(input.len() + 2);
    out.push('\'');
    for ch in input.chars() {
        if ch == '\'' {
            out.push_str("'\"'\"'");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn github_repo_slug(source_url: &str) -> Option<String> {
    if let Some(rest) = source_url.strip_prefix("git@github.com:") {
        return Some(rest.trim_end_matches(".git").to_string());
    }
    if let Some(rest) = source_url.strip_prefix("https://github.com/") {
        return Some(rest.trim_end_matches(".git").to_string());
    }
    if let Some(rest) = source_url.strip_prefix("http://github.com/") {
        return Some(rest.trim_end_matches(".git").to_string());
    }
    if let Some(rest) = source_url.strip_prefix("github.com/") {
        return Some(rest.trim_end_matches(".git").to_string());
    }
    None
}

fn git_cli_fetch_and_ff(repo_dir: &Path, env: &[(String, String)], quiet: bool) -> Result<()> {
    if !has_tool("git") {
        bail!("checkout backend requires `git` to be installed");
    }
    debug_log(&format!(
        "fetch and fast-forward {} (git CLI)",
        repo_dir.display()
    ));
    git_run(
        repo_dir,
        &["fetch", "--all", "--prune", "--tags"],
        env,
        quiet,
    )
    .context("failed to fetch updates")?;

    let abbrev = git_output(repo_dir, &["rev-parse", "--abbrev-ref", "HEAD"], env)
        .context("failed to read current HEAD")?;

    let (branch, remote_ref) = if abbrev != "HEAD" {
        let b = abbrev;
        (b.clone(), format!("origin/{b}"))
    } else {
        let sym = git_output(
            repo_dir,
            &["symbolic-ref", "-q", "refs/remotes/origin/HEAD"],
            env,
        )
        .context(
            "detached HEAD and could not read refs/remotes/origin/HEAD (try: git remote set-head origin -a)",
        )?;
        let b = sym
            .rsplit('/')
            .next()
            .ok_or_else(|| anyhow!("failed to parse remote HEAD ref"))?
            .to_string();
        (b.clone(), format!("origin/{b}"))
    };

    git_run(
        repo_dir,
        &["checkout", "-f", "-B", &branch, &remote_ref],
        env,
        quiet,
    )
    .with_context(|| format!("failed to fast-forward {branch} to {remote_ref}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CheckoutProfile, EnvVar};
    use tempfile::tempdir;

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

    #[test]
    fn github_repo_slug_handles_common_forms() {
        assert_eq!(
            github_repo_slug("git@github.com:acme/tool.git").as_deref(),
            Some("acme/tool")
        );
        assert_eq!(
            github_repo_slug("https://github.com/acme/tool").as_deref(),
            Some("acme/tool")
        );
        assert_eq!(github_repo_slug("https://gitlab.com/acme/tool"), None);
    }

    #[test]
    fn shell_quote_and_render_custom_escape_inputs() {
        assert_eq!(shell_quote(""), "''");
        assert_eq!(shell_quote("a'b"), "'a'\"'\"'b'");
        let rendered = render_custom(
            "clone {source_url} {repo_dir}",
            "https://github.com/acme/tool.git",
            Path::new("/tmp/a b"),
        );
        assert!(rendered.contains("'https://github.com/acme/tool.git'"));
        assert!(rendered.contains("'/tmp/a b'"));
    }

    #[test]
    fn resolve_checkout_plan_applies_matching_profile() {
        let mut cfg = Config::default();
        cfg.checkout_backend = CheckoutBackend::Git;
        cfg.checkout_profiles = vec![CheckoutProfile {
            name: Some("acme".into()),
            match_prefix: "https://github.com/acme/".into(),
            backend: Some(CheckoutBackend::Custom),
            env: vec![EnvVar {
                key: "A".into(),
                value: "B".into(),
            }],
            ssh_command: Some("ssh -i /k".into()),
            http_proxy: Some("http://p".into()),
            https_proxy: Some("https://p".into()),
            all_proxy: Some("socks5://p".into()),
            no_proxy: Some("localhost".into()),
            custom_clone: Some("echo clone".into()),
            custom_sync: Some("echo sync".into()),
        }];

        let plan = resolve_checkout_plan(&cfg, "https://github.com/acme/tool.git").unwrap();
        assert_eq!(plan.backend, CheckoutBackend::Custom);
        assert!(plan.env.iter().any(|(k, v)| k == "A" && v == "B"));
        assert!(plan.env.iter().any(|(k, _)| k == "GIT_SSH_COMMAND"));
        assert!(plan.env.iter().any(|(k, _)| k == "HTTP_PROXY"));
        assert_eq!(plan.custom_clone.as_deref(), Some("echo clone"));
        assert_eq!(plan.custom_sync.as_deref(), Some("echo sync"));
    }

    #[test]
    fn run_status_success_and_failure() {
        run_status(None, "sh", &["-lc".into(), "true".into()], &[]).unwrap();
        let err = run_status(None, "sh", &["-lc".into(), "false".into()], &[]).unwrap_err();
        assert!(err.to_string().contains("command failed"));
    }

    #[test]
    fn custom_clone_and_sync_execute_template_commands() {
        let td = tempdir().unwrap();
        let repo_dir = td.path().join("repo");
        std::fs::create_dir_all(&repo_dir).unwrap();
        let marker = td.path().join("marker.txt");
        let plan = CheckoutPlan {
            backend: CheckoutBackend::Custom,
            env: vec![],
            custom_clone: Some(format!(
                "printf cloned > {}",
                shell_quote(&marker.to_string_lossy())
            )),
            custom_sync: Some(format!(
                "printf synced > {}",
                shell_quote(&marker.to_string_lossy())
            )),
        };

        custom_clone("https://example.com/repo.git", &repo_dir, &plan).unwrap();
        assert_eq!(std::fs::read_to_string(&marker).unwrap(), "cloned");
        custom_sync(&repo_dir, &plan).unwrap();
        assert_eq!(std::fs::read_to_string(&marker).unwrap(), "synced");
    }

    #[test]
    fn git_cli_fetch_and_ff_works_for_simple_origin_main() {
        let td = tempdir().unwrap();
        let src = td.path().join("src");
        let origin = td.path().join("origin.git");
        let clone = td.path().join("clone");
        std::fs::create_dir_all(&src).unwrap();
        run_git(&src, &["init"]);
        std::fs::write(src.join("README.md"), "v1").unwrap();
        run_git(&src, &["add", "."]);
        run_git(&src, &["commit", "-m", "init"]);
        run_git(
            &src,
            &[
                "clone",
                "--bare",
                src.to_str().unwrap(),
                origin.to_str().unwrap(),
            ],
        );
        run_git(&src, &["remote", "add", "origin", origin.to_str().unwrap()]);
        run_git(&src, &["push", "-u", "origin", "HEAD"]);
        run_git(
            &td.path().to_path_buf(),
            &["clone", origin.to_str().unwrap(), clone.to_str().unwrap()],
        );

        std::fs::write(src.join("README.md"), "v2").unwrap();
        run_git(&src, &["add", "."]);
        run_git(&src, &["commit", "-m", "v2"]);
        run_git(&src, &["push", "origin", "HEAD"]);

        git_cli_fetch_and_ff(&clone, &[], true).unwrap();
        let head = git_output(&clone, &["show", "-s", "--format=%s", "HEAD"], &[]).unwrap();
        assert_eq!(head, "v2");
    }
}
