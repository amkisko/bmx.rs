use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

pub(crate) fn git_run(
    cwd: &Path,
    args: &[&str],
    env: &[(String, String)],
    quiet: bool,
) -> Result<()> {
    let preview = format!("git {}", args.join(" "));
    let mut cmd = Command::new("git");
    let owned: Vec<&str> = if quiet && supports_quiet(args) {
        let mut v = Vec::with_capacity(args.len() + 1);
        v.push(args[0]);
        v.push("-q");
        v.extend_from_slice(&args[1..]);
        v
    } else {
        args.to_vec()
    };
    cmd.current_dir(cwd).args(owned);
    for (key, value) in env {
        cmd.env(key, value);
    }
    let status = cmd
        .status()
        .with_context(|| format!("failed to execute {preview}"))?;
    if !status.success() {
        bail!("command failed: {preview}");
    }
    Ok(())
}

pub(crate) fn git_output(cwd: &Path, args: &[&str], env: &[(String, String)]) -> Result<String> {
    let preview = format!("git {}", args.join(" "));
    let mut cmd = Command::new("git");
    cmd.current_dir(cwd).args(args);
    for (key, value) in env {
        cmd.env(key, value);
    }
    let out = cmd
        .output()
        .with_context(|| format!("failed to execute {preview}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn supports_quiet(args: &[&str]) -> bool {
    match args.first().copied() {
        Some("clean") | Some("checkout") | Some("fetch") | Some("pull") => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::supports_quiet;

    #[test]
    fn quiet_allowed_commands() {
        assert!(supports_quiet(&["clean"]));
        assert!(supports_quiet(&["checkout", "--force"]));
        assert!(supports_quiet(&["fetch", "--all"]));
        assert!(supports_quiet(&["pull", "--ff-only"]));
    }

    #[test]
    fn quiet_blocked_for_other_commands() {
        assert!(!supports_quiet(&["verify-commit", "HEAD"]));
        assert!(!supports_quiet(&[]));
    }
}
