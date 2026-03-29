mod checkout;

use std::path::Path;

use anyhow::{Result, bail};

use crate::runtime::subprocess_output_visible;
use crate::types::{CheckoutBackend, Config};

use checkout::{clone_repo, sync_existing_repo};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CheckoutProfile, EnvVar};

    #[test]
    fn resolve_checkout_plan_applies_matching_profile() {
        let cfg = Config {
            checkout_backend: CheckoutBackend::Git,
            checkout_profiles: vec![CheckoutProfile {
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
            }],
            ..Default::default()
        };

        let plan = resolve_checkout_plan(&cfg, "https://github.com/acme/tool.git").unwrap();
        assert_eq!(plan.backend, CheckoutBackend::Custom);
        assert!(plan.env.iter().any(|(k, v)| k == "A" && v == "B"));
        assert!(plan.env.iter().any(|(k, _)| k == "GIT_SSH_COMMAND"));
        assert!(plan.env.iter().any(|(k, _)| k == "HTTP_PROXY"));
        assert_eq!(plan.custom_clone.as_deref(), Some("echo clone"));
        assert_eq!(plan.custom_sync.as_deref(), Some("echo sync"));
    }
}
