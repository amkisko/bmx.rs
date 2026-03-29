use std::fs;
use std::path::Path;
#[cfg(windows)]
use std::process::Command;

use anyhow::{Context, Result, anyhow};

use crate::history::{AuditEntry, append_audit, audit_unix_ts, new_action_id};
use crate::io::read_toml;
use crate::layout::resolve_layout_for_run;
use crate::types::InstallMetadata;

use super::TrustOptions;
use super::install::install_app;

pub(crate) fn self_update(
    home: &Path,
    app: &str,
    cli_verbose: bool,
    record_history: bool,
    trust_prompt: bool,
    trust_global: bool,
) -> Result<()> {
    install_app(
        home,
        app,
        None,
        cli_verbose,
        false,
        true,
        TrustOptions {
            prompt: trust_prompt,
            global: trust_global,
        },
    )?;

    let layout = resolve_layout_for_run(home, app);
    let metadata: InstallMetadata = read_toml(&layout.meta_file)?;
    let new_executable = layout.repo_dir.join(&metadata.executable_rel);
    if !new_executable.exists() {
        return Err(anyhow!(
            "self-update built executable does not exist: {}",
            new_executable.display()
        ));
    }

    let current_executable =
        std::env::current_exe().context("failed to locate current executable")?;
    replace_current_executable(&current_executable, &new_executable)?;
    append_audit(
        home,
        &AuditEntry {
            id: new_action_id(),
            ts: audit_unix_ts(),
            kind: "self-update".into(),
            summary: format!("self-update {app}"),
            app: Some(metadata.app.clone()),
            source_url: Some(metadata.source_url.clone()),
            undoable: false,
        },
        record_history,
    )?;
    Ok(())
}

fn staged_replacement_path(current_executable: &Path) -> std::path::PathBuf {
    #[cfg(windows)]
    {
        let ext = current_executable
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("exe");
        return current_executable.with_extension(format!("{ext}.bmx-new"));
    }
    #[cfg(not(windows))]
    {
        current_executable.with_extension("bmx-new")
    }
}

fn replace_current_executable(current_executable: &Path, new_executable: &Path) -> Result<()> {
    let staged = staged_replacement_path(current_executable);
    if staged.exists() {
        fs::remove_file(&staged)
            .with_context(|| format!("failed removing stale staged file {}", staged.display()))?;
    }

    fs::copy(new_executable, &staged).with_context(|| {
        format!(
            "failed copying new executable from {} to {}",
            new_executable.display(),
            staged.display()
        )
    })?;

    #[cfg(unix)]
    {
        fs::rename(&staged, current_executable).with_context(|| {
            format!(
                "failed replacing {} with {}",
                current_executable.display(),
                staged.display()
            )
        })?;
        Ok(())
    }

    #[cfg(windows)]
    {
        let pid = std::process::id();
        let script = format!(
            "$src='{}';$dst='{}';$pid={};while (Get-Process -Id $pid -ErrorAction SilentlyContinue) {{ Start-Sleep -Milliseconds 200 }}; Move-Item -Force $src $dst",
            powershell_quote(&staged.to_string_lossy()),
            powershell_quote(&current_executable.to_string_lossy()),
            pid
        );
        Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .spawn()
            .context("failed to schedule deferred executable replacement")?;
        eprintln!(
            "[bmx] replacement of {} scheduled after process exit",
            current_executable.display()
        );
        Ok(())
    }
}

#[cfg(windows)]
fn powershell_quote(input: &str) -> String {
    input.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::staged_replacement_path;
    use std::path::{Path, PathBuf};

    #[test]
    fn staged_replacement_path_uses_bmx_new_extension_on_unix() {
        #[cfg(unix)]
        {
            let p = Path::new("/tmp/bmx");
            assert_eq!(
                staged_replacement_path(p),
                PathBuf::from("/tmp/bmx.bmx-new")
            );
        }
    }
}
