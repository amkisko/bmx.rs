use std::path::Path;

use anyhow::{Result, anyhow, bail};

use crate::process::{has_tool, run_checked};
use crate::types::BuildIsolation;

pub(crate) fn run_build_checked(
    cwd: &Path,
    cmd: &str,
    args: &[&str],
    mode: BuildIsolation,
    show_output: bool,
) -> Result<()> {
    if mode == BuildIsolation::Off {
        return run_checked(Some(cwd), cmd, args, show_output);
    }

    let backend = resolve_backend(mode)?;
    let mount = format!("{}:/workspace", cwd.display());
    let shell_cmd = shell_join(cmd, args);
    let image = std::env::var("BMX_ISOLATION_IMAGE")
        .unwrap_or_else(|_| "ghcr.io/catthehacker/ubuntu:full-latest".to_string());

    let container_args = [
        "run",
        "--rm",
        "-v",
        &mount,
        "-w",
        "/workspace",
        &image,
        "sh",
        "-lc",
        &shell_cmd,
    ];

    run_checked(None, backend, &container_args, show_output).map_err(|err| {
        anyhow!(
            "{err}. isolated build backend `{backend}` uses image `{image}`; override with BMX_ISOLATION_IMAGE if required toolchains are missing"
        )
    })
}

pub(crate) fn resolve_backend(mode: BuildIsolation) -> Result<&'static str> {
    match mode {
        BuildIsolation::Off => bail!("isolation backend is disabled"),
        BuildIsolation::Docker => require_tool("docker"),
        BuildIsolation::Podman => require_tool("podman"),
        BuildIsolation::Nerdctl => require_tool("nerdctl"),
        BuildIsolation::Auto => {
            for tool in ["docker", "podman", "nerdctl"] {
                if has_tool(tool) {
                    return Ok(tool);
                }
            }
            bail!(
                "build isolation is enabled (auto), but no supported backend was found; install docker, podman, or nerdctl"
            )
        }
    }
}

fn require_tool(tool: &'static str) -> Result<&'static str> {
    if has_tool(tool) {
        return Ok(tool);
    }
    bail!("build isolation backend `{tool}` is not installed")
}

fn shell_join(cmd: &str, args: &[&str]) -> String {
    let mut out = shell_quote(cmd);
    for arg in args {
        out.push(' ');
        out.push_str(&shell_quote(arg));
    }
    out
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
