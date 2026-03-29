use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow};

use crate::app_spec::{AppSpec, layout_id, parse_app_spec};
use crate::build::{build_with_strategy, detect_strategy};
use crate::config::load_config;
use crate::executable::detect_executable_rel;
use crate::history::{
    AuditEntry, UndoFrame, UndoItem, append_audit, audit_unix_ts, capture_install_snapshot,
    new_action_id, push_undo_frame,
};
use crate::hooks;
use crate::install_flow::{must_reinstall, reinstall_from_meta};
use crate::integrity::verify_repo_matches_metadata;
use crate::io::read_toml;
use crate::layout::{
    app_layout_for_id, resolve_installed_layout, resolve_layout_for_run, sanitize_install_as,
};
use crate::process::{forward_exit, has_tool};
use crate::repo::sync_repo;
use crate::revision::checkout_requested_ref;
use crate::runtime::subprocess_output_visible;
use crate::source::resolve_source;
use crate::types::{Config, InstallMetadata};

fn install_refers_to_existing(
    cfg: &Config,
    spec: &AppSpec,
    existing: &InstallMetadata,
) -> Result<bool> {
    if spec.source == existing.app {
        return Ok(true);
    }
    let resolved = resolve_source(cfg, &spec.source)?;
    Ok(resolved == existing.source_url)
}

pub(crate) fn run_app(home: &Path, app: &str, args: &[String], verbose: bool) -> Result<()> {
    let spec = parse_app_spec(app);
    let layout = resolve_layout_for_run(home, app);
    if must_reinstall(&layout.meta_file, &spec.requested_ref)? {
        install_app(home, app, None, verbose, false)?;
    }

    let cfg = load_config(home)?;
    let layout = resolve_layout_for_run(home, app);
    let mut metadata: InstallMetadata = read_toml(&layout.meta_file)?;
    if cfg.integrity_check {
        verify_repo_matches_metadata(&layout, &metadata)?;
    }

    let mut executable = layout.repo_dir.join(&metadata.executable_rel);
    executable = executable
        .canonicalize()
        .unwrap_or_else(|_| layout.repo_dir.join(&metadata.executable_rel));
    if !executable.exists() {
        install_app(home, app, None, verbose, false)?;
        let layout = resolve_layout_for_run(home, app);
        metadata = read_toml(&layout.meta_file)?;
        if cfg.integrity_check {
            verify_repo_matches_metadata(&layout, &metadata)?;
        }
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

fn path_for_child(executable: &Path) -> Option<OsString> {
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

pub(crate) fn install_app(
    home: &Path,
    app: &str,
    install_as: Option<&str>,
    cli_verbose: bool,
    record_history: bool,
) -> Result<()> {
    let spec = parse_app_spec(app);
    let cfg = load_config(home)?;
    let directory_id = if let Some(a) = install_as {
        sanitize_install_as(a)?
    } else {
        layout_id(&spec)
    };
    let layout = app_layout_for_id(home, directory_id);

    if layout.meta_file.exists() {
        let existing: InstallMetadata = read_toml(&layout.meta_file)?;
        if let Some(a) = install_as {
            let want = sanitize_install_as(a)?;
            if existing.app != want {
                anyhow::bail!(
                    "this install is registered as `{}`, not `{}`",
                    existing.app,
                    want
                );
            }
        }
        if !install_refers_to_existing(&cfg, &spec, &existing)? {
            anyhow::bail!(
                "directory `{}` already holds an install from {}\n\
                 hint: `bmx uninstall {}` or use `bmx install {} --as <unique-name>` for the other source",
                layout.id,
                existing.source_url,
                layout.id,
                app
            );
        }
        let mut merged = existing;
        if spec.requested_ref.is_some() {
            merged.requested_ref = spec.requested_ref.clone();
        }
        if spec.cargo_package.is_some() {
            merged.rust_package = spec.cargo_package.clone();
        }
        let app_id = merged.app.clone();
        let source_url = merged.source_url.clone();
        let snapshot_dir = capture_install_snapshot(home, &app_id)?;
        reinstall_from_meta(home, merged, cli_verbose)?;
        let id = new_action_id();
        append_audit(
            home,
            &AuditEntry {
                id: id.clone(),
                ts: audit_unix_ts(),
                kind: "update".into(),
                summary: format!("update {app}"),
                app: Some(app_id.clone()),
                source_url: Some(source_url),
                undoable: true,
            },
            record_history,
        )?;
        push_undo_frame(
            home,
            UndoFrame {
                id,
                ts: audit_unix_ts(),
                summary: format!("update {app}"),
                items: vec![UndoItem::RestoreSnapshot {
                    app_id,
                    snapshot_dir,
                }],
            },
            record_history,
        )?;
        return Ok(());
    }

    fs::create_dir_all(&layout.repo_dir)?;

    let show_output = subprocess_output_visible(cli_verbose);
    let source_url = resolve_source(&cfg, &spec.source)?;
    sync_repo(&source_url, &layout.repo_dir, &cfg, cli_verbose)?;
    let resolved_commit =
        checkout_requested_ref(&layout.repo_dir, spec.requested_ref.as_deref(), cli_verbose)?;

    let strategy = detect_strategy(&layout.repo_dir).ok_or_else(|| {
        anyhow!(
            "unable to detect build strategy for {}",
            layout.repo_dir.display()
        )
    })?;
    build_with_strategy(
        &strategy,
        &layout.repo_dir,
        cfg.build_isolation,
        show_output,
        spec.cargo_package.as_deref(),
    )?;

    let executable_rel = detect_executable_rel(&layout.repo_dir, &spec)?;
    let app_id = layout.id.clone();
    let metadata = InstallMetadata {
        app: app_id.clone(),
        source_url,
        executable_rel,
        strategy: strategy.as_str().to_string(),
        requested_ref: spec.requested_ref,
        resolved_commit,
        rust_package: spec.cargo_package.clone(),
    };
    crate::io::write_toml(&layout.meta_file, &metadata)?;
    hooks::run_post_install_hook(&layout.repo_dir)?;
    let id = new_action_id();
    append_audit(
        home,
        &AuditEntry {
            id: id.clone(),
            ts: audit_unix_ts(),
            kind: "install".into(),
            summary: format!("install {app}"),
            app: Some(app_id.clone()),
            source_url: Some(metadata.source_url.clone()),
            undoable: true,
        },
        record_history,
    )?;
    push_undo_frame(
        home,
        UndoFrame {
            id,
            ts: audit_unix_ts(),
            summary: format!("install {app}"),
            items: vec![UndoItem::FreshInstall { app_id }],
        },
        record_history,
    )?;
    Ok(())
}

pub(crate) fn reinstall_app(
    home: &Path,
    app: &str,
    cli_verbose: bool,
    record_history: bool,
) -> Result<()> {
    let layout = resolve_installed_layout(home, app)?;
    let meta: InstallMetadata = read_toml(&layout.meta_file)?;
    let app_id = meta.app.clone();
    let source_url = meta.source_url.clone();
    let snapshot_dir = capture_install_snapshot(home, &app_id)?;
    reinstall_from_meta(home, meta, cli_verbose)?;
    let id = new_action_id();
    append_audit(
        home,
        &AuditEntry {
            id: id.clone(),
            ts: audit_unix_ts(),
            kind: "reinstall".into(),
            summary: format!("reinstall {app}"),
            app: Some(app_id.clone()),
            source_url: Some(source_url),
            undoable: true,
        },
        record_history,
    )?;
    push_undo_frame(
        home,
        UndoFrame {
            id,
            ts: audit_unix_ts(),
            summary: format!("reinstall {app}"),
            items: vec![UndoItem::RestoreSnapshot {
                app_id,
                snapshot_dir,
            }],
        },
        record_history,
    )?;
    Ok(())
}

pub(crate) fn show_package(home: &Path, app: &str, would_remove: bool) -> Result<()> {
    let layout = resolve_installed_layout(home, app)?;
    let app_root = layout
        .repo_dir
        .parent()
        .ok_or_else(|| anyhow!("invalid app layout for {}", layout.id))?;
    let root = if would_remove {
        app_root
    } else {
        &layout.repo_dir
    };
    let files = list_files_under(root)?;
    for rel in files {
        println!("{}", rel.display());
    }
    Ok(())
}

fn list_files_under(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    fn walk(dir: &Path, base: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
        for e in fs::read_dir(dir)? {
            let p = e?.path();
            if p.is_dir() {
                walk(&p, base, out)?;
            } else {
                let rel = p.strip_prefix(base).unwrap_or(&p).to_path_buf();
                out.push(rel);
            }
        }
        Ok(())
    }
    if root.is_dir() {
        walk(root, root, &mut out)?;
    }
    out.sort();
    Ok(out)
}

pub(crate) fn self_update(
    home: &Path,
    app: &str,
    cli_verbose: bool,
    record_history: bool,
) -> Result<()> {
    install_app(home, app, None, cli_verbose, false)?;

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
        return Ok(());
    }
}

#[cfg(windows)]
fn powershell_quote(input: &str) -> String {
    input.replace('\'', "''")
}

pub(crate) fn uninstall_app(home: &Path, app: &str, record_history: bool) -> Result<()> {
    let layout = resolve_layout_for_run(home, app);
    let app_root = layout
        .repo_dir
        .parent()
        .ok_or_else(|| anyhow!("invalid app layout for {}", layout.id))?;

    let source_url = if layout.meta_file.exists() {
        read_toml::<InstallMetadata>(&layout.meta_file)
            .ok()
            .map(|m| m.source_url)
    } else {
        None
    };

    append_audit(
        home,
        &AuditEntry {
            id: new_action_id(),
            ts: audit_unix_ts(),
            kind: "uninstall".into(),
            summary: format!("uninstall {app}"),
            app: Some(layout.id.clone()),
            source_url,
            undoable: false,
        },
        record_history,
    )?;

    if app_root.exists() {
        fs::remove_dir_all(app_root)
            .with_context(|| format!("failed removing {}", app_root.display()))?;
    }
    Ok(())
}

pub(crate) fn update_all(home: &Path, cli_verbose: bool, record_history: bool) -> Result<()> {
    let apps_root = home.join("apps");
    if !apps_root.exists() {
        return Ok(());
    }

    let mut plan: Vec<(InstallMetadata, PathBuf)> = Vec::new();
    for entry in fs::read_dir(&apps_root)? {
        let meta_path = entry?.path().join("install.toml");
        if !meta_path.exists() {
            continue;
        }

        let install: InstallMetadata = read_toml(&meta_path)?;
        let snapshot_dir = capture_install_snapshot(home, &install.app)?;
        plan.push((install, snapshot_dir));
    }
    plan.sort_by(|a, b| a.0.app.cmp(&b.0.app));

    for (install, _) in &plan {
        reinstall_from_meta(home, install.clone(), cli_verbose)?;
    }

    if plan.is_empty() {
        return Ok(());
    }

    let n = plan.len();
    let items: Vec<UndoItem> = plan
        .into_iter()
        .map(|(i, snap)| UndoItem::RestoreSnapshot {
            app_id: i.app,
            snapshot_dir: snap,
        })
        .collect();
    let id = new_action_id();
    append_audit(
        home,
        &AuditEntry {
            id: id.clone(),
            ts: audit_unix_ts(),
            kind: "update-all".into(),
            summary: format!("update all ({n} apps)"),
            app: None,
            source_url: None,
            undoable: true,
        },
        record_history,
    )?;
    push_undo_frame(
        home,
        UndoFrame {
            id,
            ts: audit_unix_ts(),
            summary: format!("update all ({n} apps)"),
            items,
        },
        record_history,
    )?;
    Ok(())
}

pub(crate) fn doctor(home: &Path) -> Result<()> {
    let cfg = load_config(home)?;
    println!("bmx home: {}", home.display());
    println!("checkout/sync: system `git` CLI (unless gh/custom profile)");
    println!(
        "default source: {}",
        cfg.default_source
            .clone()
            .unwrap_or_else(|| "(not set)".to_string())
    );
    println!("build isolation: {}", cfg.build_isolation.as_str());
    println!("checkout backend: {}", cfg.checkout_backend.as_str());
    println!("integrity_check (config): {}", cfg.integrity_check);
    println!("registries defined: {}", cfg.registries.len());

    if let Ok(out) = Command::new("git").arg("--version").output()
        && out.status.success()
    {
        print!("{}", String::from_utf8_lossy(&out.stdout));
    }

    let apps = home.join("apps");
    let mut approx_bytes: u64 = 0;
    let mut broken: Vec<String> = Vec::new();
    if apps.is_dir() {
        for e in fs::read_dir(&apps)? {
            let p = e?.path();
            if !p.is_dir() {
                continue;
            }
            approx_bytes += du_dir_bytes(&p)?;
            let meta = p.join("install.toml");
            if !meta.exists() {
                continue;
            }
            let ok = match read_toml::<InstallMetadata>(&meta) {
                Ok(m) => p.join("repo").join(&m.executable_rel).exists(),
                Err(_) => false,
            };
            if !ok && let Some(id) = p.file_name().and_then(|n| n.to_str()) {
                broken.push(id.to_string());
            }
        }
    }
    println!(
        "approx cache under {}/apps: {} bytes",
        home.display(),
        approx_bytes
    );
    if !broken.is_empty() {
        println!(
            "possible broken installs (bad install.toml or missing binary): {}",
            broken.join(", ")
        );
    }

    for tool in [
        "cargo", "cmake", "make", "brew", "yay", "paru", "makepkg", "docker", "podman", "nerdctl",
        "git", "gh",
    ] {
        println!("{tool}: {}", if has_tool(tool) { "ok" } else { "missing" });
    }
    Ok(())
}

fn du_dir_bytes(path: &Path) -> Result<u64> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_dir() {
        return Ok(meta.len());
    }
    let mut n = 0u64;
    for e in fs::read_dir(path)? {
        n += du_dir_bytes(&e?.path())?;
    }
    Ok(n)
}
