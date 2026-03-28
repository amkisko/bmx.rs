use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, anyhow};

use crate::app_spec::parse_app_spec;
use crate::build::{build_with_strategy, detect_strategy};
use crate::config::load_config;
use crate::executable::detect_executable_rel;
use crate::hooks;
use crate::install_flow::{must_reinstall, reinstall_from_meta};
use crate::integrity::verify_repo_matches_metadata;
use crate::io::read_toml;
use crate::layout::app_layout_from_spec;
use crate::process::{forward_exit, has_tool};
use crate::repo::sync_repo;
use crate::revision::checkout_requested_ref;
use crate::runtime::subprocess_output_visible;
use crate::source::resolve_source;
use crate::types::InstallMetadata;

pub(crate) fn run_app(home: &Path, app: &str, args: &[String], verbose: bool) -> Result<()> {
    let spec = parse_app_spec(app);
    let layout = app_layout_from_spec(home, &spec);
    if must_reinstall(&layout.meta_file, &spec.requested_ref)? {
        install_app(home, app, verbose)?;
    }

    let cfg = load_config(home)?;
    let mut metadata: InstallMetadata = read_toml(&layout.meta_file)?;
    if cfg.integrity_check {
        verify_repo_matches_metadata(&layout, &metadata)?;
    }

    let mut executable = layout.repo_dir.join(&metadata.executable_rel);
    executable = executable
        .canonicalize()
        .unwrap_or_else(|_| layout.repo_dir.join(&metadata.executable_rel));
    if !executable.exists() {
        install_app(home, app, verbose)?;
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
            metadata
                .resolved_commit
                .as_deref()
                .unwrap_or("(none)")
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

pub(crate) fn install_app(home: &Path, app: &str, cli_verbose: bool) -> Result<()> {
    let spec = parse_app_spec(app);
    let cfg = load_config(home)?;
    let layout = app_layout_from_spec(home, &spec);
    fs::create_dir_all(&layout.repo_dir)?;

    let show_output = subprocess_output_visible(cli_verbose);
    let source_url = resolve_source(&cfg, &spec.source)?;
    sync_repo(&source_url, &layout.repo_dir, &cfg, cli_verbose)?;
    let resolved_commit = checkout_requested_ref(
        &layout.repo_dir,
        spec.requested_ref.as_deref(),
        cli_verbose,
    )?;

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
    let metadata = InstallMetadata {
        app: layout.id,
        source_url,
        executable_rel,
        strategy: strategy.as_str().to_string(),
        requested_ref: spec.requested_ref,
        resolved_commit,
        rust_package: spec.cargo_package.clone(),
    };
    crate::io::write_toml(&layout.meta_file, &metadata)?;
    hooks::run_post_install_hook(&layout.repo_dir)?;
    Ok(())
}

pub(crate) fn self_update(home: &Path, app: &str, cli_verbose: bool) -> Result<()> {
    install_app(home, app, cli_verbose)?;

    let spec = parse_app_spec(app);
    let layout = app_layout_from_spec(home, &spec);
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
    replace_current_executable(&current_executable, &new_executable)
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
        return Ok(());
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

pub(crate) fn uninstall_app(home: &Path, app: &str) -> Result<()> {
    let spec = parse_app_spec(app);
    let layout = app_layout_from_spec(home, &spec);
    let app_root = layout
        .repo_dir
        .parent()
        .ok_or_else(|| anyhow!("invalid app layout for {}", layout.id))?;

    if app_root.exists() {
        fs::remove_dir_all(app_root)
            .with_context(|| format!("failed removing {}", app_root.display()))?;
    }
    Ok(())
}

pub(crate) fn update_all(home: &Path, cli_verbose: bool) -> Result<()> {
    let apps_root = home.join("apps");
    if !apps_root.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(&apps_root)? {
        let meta = entry?.path().join("install.toml");
        if !meta.exists() {
            continue;
        }

        let install: InstallMetadata = read_toml(&meta)?;
        reinstall_from_meta(home, install, cli_verbose)?;
    }

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

    if let Ok(out) = Command::new("git").arg("--version").output() {
        if out.status.success() {
            print!("{}", String::from_utf8_lossy(&out.stdout));
        }
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
            if !ok {
                if let Some(id) = p.file_name().and_then(|n| n.to_str()) {
                    broken.push(id.to_string());
                }
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
