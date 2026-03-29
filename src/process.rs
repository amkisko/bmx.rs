use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::time::Duration;
use std::{io::IsTerminal, thread};

use anyhow::{Context, Result, bail};

use crate::runtime::debug_log;

pub(crate) fn has_tool(name: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {name} >/dev/null 2>&1"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub(crate) fn run_checked(
    cwd: Option<&Path>,
    cmd: &str,
    args: &[&str],
    show_output: bool,
) -> Result<()> {
    let command_preview = format!("{cmd} {}", args.join(" "));
    debug_log(&format!("run_checked: {command_preview}"));

    let mut command = Command::new(cmd);
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }

    let status = if show_output {
        command
            .args(args)
            .status()
            .with_context(|| format!("failed to execute {command_preview}"))?
    } else {
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .args(args);

        let mut child = command
            .spawn()
            .with_context(|| format!("failed to execute {command_preview}"))?;
        let spinner = ["|", "/", "-", "\\"];
        let tty = std::io::stderr().is_terminal();
        let mut i = 0usize;

        loop {
            if let Some(status) = child.try_wait()? {
                if tty {
                    eprint!("\r\x1b[2K");
                }
                break status;
            }

            if tty {
                eprint!("\r[bmx] {} {}", spinner[i % spinner.len()], command_preview);
                i += 1;
            }
            thread::sleep(Duration::from_millis(120));
        }
    };

    if !status.success() {
        bail!("command failed: {command_preview}");
    }

    Ok(())
}

pub(crate) fn forward_exit(status: ExitStatus) -> Result<()> {
    if status.success() {
        return Ok(());
    }

    if let Some(code) = status.code() {
        std::process::exit(code);
    }

    bail!("process terminated by signal")
}
