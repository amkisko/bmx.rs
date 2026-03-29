use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use crate::config::load_config;
use crate::hooks;
use crate::install_flow::must_reinstall;
use crate::integrity::verify_repo_matches_metadata;
use crate::io::read_toml;
use crate::layout::resolve_layout_for_run;
use crate::process::forward_exit;
use crate::trust::enforce_source_trust;
use crate::types::InstallMetadata;

use crate::app_spec::parse_app_spec;

use super::TrustOptions;
use super::install::install_app;

pub(crate) fn run_app(
    home: &Path,
    app: &str,
    args: &[String],
    verbose: bool,
    persist_requested_ref: bool,
) -> Result<()> {
    let spec = parse_app_spec(app);
    let layout = resolve_layout_for_run(home, app);
    let desired_persistent_ref = if persist_requested_ref {
        spec.requested_ref.clone()
    } else {
        None
    };
    let needs_reinstall = if spec.requested_ref.is_some() && !persist_requested_ref {
        // Explicit refs are one-off by default; reinstall to honor this invocation.
        true
    } else {
        must_reinstall(&layout.meta_file, &desired_persistent_ref)?
    };
    if needs_reinstall {
        install_app(
            home,
            app,
            None,
            verbose,
            false,
            persist_requested_ref,
            TrustOptions {
                prompt: false,
                global: false,
            },
        )?;
    }

    let cfg = load_config(home)?;
    let layout = resolve_layout_for_run(home, app);
    let mut metadata: InstallMetadata = read_toml(&layout.meta_file)?;
    if cfg.integrity_check {
        verify_repo_matches_metadata(&layout, &metadata)?;
    }
    enforce_source_trust(home, &metadata.source_url, &layout.repo_dir)?;

    let mut executable = layout.repo_dir.join(&metadata.executable_rel);
    executable = executable
        .canonicalize()
        .unwrap_or_else(|_| layout.repo_dir.join(&metadata.executable_rel));
    if !executable.exists() {
        install_app(
            home,
            app,
            None,
            verbose,
            false,
            persist_requested_ref,
            TrustOptions {
                prompt: false,
                global: false,
            },
        )?;
        let layout = resolve_layout_for_run(home, app);
        metadata = read_toml(&layout.meta_file)?;
        if cfg.integrity_check {
            verify_repo_matches_metadata(&layout, &metadata)?;
        }
        enforce_source_trust(home, &metadata.source_url, &layout.repo_dir)?;
        executable = layout.repo_dir.join(&metadata.executable_rel);
        executable = executable
            .canonicalize()
            .unwrap_or_else(|_| layout.repo_dir.join(&metadata.executable_rel));
    }

    if verbose {
        eprintln!(
            "[bmx] app={} executable={} resolved_commit={}",
            metadata.app,
            executable.display(),
            metadata.resolved_commit.as_deref().unwrap_or("(none)")
        );
    }

    hooks::run_pre_run_hook(&layout.repo_dir)?;

    let path_env = path_for_child(&executable);
    let mut cmd = Command::new(&executable);
    cmd.current_dir(&layout.repo_dir);
    cmd.args(args);
    if let Some(p) = path_env {
        cmd.env("PATH", p);
    }
    let status = cmd
        .status()
        .with_context(|| format!("failed to execute {}", executable.display()))?;
    forward_exit(status)
}

pub(crate) fn path_for_child(executable: &Path) -> Option<OsString> {
    let parent = executable.parent()?;
    let rest = std::env::var_os("PATH")?;
    #[cfg(windows)]
    const SEP: &str = ";";
    #[cfg(not(windows))]
    const SEP: &str = ":";
    let mut out = OsString::from(parent.as_os_str());
    out.push(SEP);
    out.push(rest);
    Some(out)
}
