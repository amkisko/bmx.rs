use std::env;
#[cfg(windows)]
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::time::Duration;
use std::{io::IsTerminal, thread};

use anyhow::{Context, Result, bail};

use crate::runtime::debug_log;

pub(crate) fn has_tool(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let tool_path = Path::new(name);
    if tool_path.is_absolute() || contains_path_separators(name) {
        return is_executable(tool_path);
    }

    let Some(path_env) = env::var_os("PATH") else {
        return false;
    };

    for dir in env::split_paths(&path_env) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        if tool_in_dir(&dir, name) {
            return true;
        }
    }

    false
}

fn contains_path_separators(input: &str) -> bool {
    input.contains(std::path::MAIN_SEPARATOR) || input.contains('/') || input.contains('\\')
}

#[cfg(unix)]
fn tool_in_dir(dir: &Path, name: &str) -> bool {
    let candidate = dir.join(name);
    is_executable(&candidate)
}

#[cfg(windows)]
fn tool_in_dir(dir: &Path, name: &str) -> bool {
    if is_executable(&dir.join(name)) {
        return true;
    }

    if Path::new(name).extension().is_some() {
        return false;
    }

    for ext in pathext_values() {
        let mut file = OsString::from(name);
        file.push(ext);
        if is_executable(&dir.join(&file)) {
            return true;
        }
    }

    false
}

#[cfg(windows)]
fn pathext_values() -> Vec<String> {
    env::var("PATHEXT")
        .ok()
        .map(|value| {
            value
                .split(';')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_ascii_uppercase())
                .collect()
        })
        .unwrap_or_else(|| vec![".COM".into(), ".EXE".into(), ".BAT".into(), ".CMD".into()])
}

fn is_executable(path: &Path) -> bool {
    let metadata = match fs::metadata(path) {
        Ok(meta) => meta,
        Err(_) => return false,
    };
    if !metadata.is_file() {
        return false;
    }

    is_executable_mode(&metadata)
}

#[cfg(unix)]
fn is_executable_mode(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(windows)]
fn is_executable_mode(_: &fs::Metadata) -> bool {
    true
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
