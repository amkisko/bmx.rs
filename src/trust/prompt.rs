use std::io::{IsTerminal, Write};
use std::path::Path;

use anyhow::{Result, bail};

use super::commands::add_allowed_signing_key;
use super::enforce::signer_matches_allowed;
use super::git_env::{repo_signing_keys, trust_git_output};
use super::policy::{best_rule, keys_missing_for_trust_scope, load_policy_or_default};
use crate::runtime::debug_log;

#[derive(Debug, Clone)]
struct HeadAssessment {
    commit: String,
    short_commit: String,
    author_name: String,
    author_email: String,
    authored_at: String,
    subject: String,
    sig_status: String,
    sig_signer: String,
    sig_key: String,
    sig_fingerprint: String,
}

fn head_assessment(home: &Path, source_url: &str, repo_dir: &Path) -> Option<HeadAssessment> {
    let fmt = "%H%n%h%n%an%n%ae%n%aI%n%s%n%G?%n%GS%n%GK%n%GF";
    let raw = trust_git_output(
        home,
        source_url,
        repo_dir,
        &["log", "-1", &format!("--format={fmt}")],
    )
    .ok()?;
    let mut lines = raw.lines();
    Some(HeadAssessment {
        commit: lines.next().unwrap_or_default().to_string(),
        short_commit: lines.next().unwrap_or_default().to_string(),
        author_name: lines.next().unwrap_or_default().to_string(),
        author_email: lines.next().unwrap_or_default().to_string(),
        authored_at: lines.next().unwrap_or_default().to_string(),
        subject: lines.next().unwrap_or_default().to_string(),
        sig_status: lines.next().unwrap_or_default().to_string(),
        sig_signer: lines.next().unwrap_or_default().to_string(),
        sig_key: lines.next().unwrap_or_default().to_string(),
        sig_fingerprint: lines.next().unwrap_or_default().to_string(),
    })
}

fn sig_status_human(code: &str) -> &'static str {
    match code {
        "G" => "good signature",
        "U" => "good signature (untrusted key)",
        "B" => "bad signature",
        "N" => "no signature",
        "E" => "signature verification error",
        _ => "unknown signature state",
    }
}

pub(crate) fn prompt_import_signing_keys_for_source(
    home: &Path,
    source_url: &str,
    repo_dir: &Path,
    global_scope: bool,
) -> Result<()> {
    let keys = repo_signing_keys(home, source_url, repo_dir);
    if keys.is_empty() {
        eprintln!(
            "[bmx][trust] no signer key/fingerprint found on HEAD (nothing to import): {}",
            repo_dir.display()
        );
        return Ok(());
    }
    let missing = keys_missing_for_trust_scope(home, source_url, &keys, global_scope)?;
    if missing.is_empty() {
        eprintln!("[bmx][trust] signer key already trusted for {source_url}");
        return Ok(());
    }

    if crate::input_mode::no_input()
        || !std::io::stdin().is_terminal()
        || !std::io::stderr().is_terminal()
    {
        bail!(
            "--trust requires interactive consent to import signer keys for {source_url} (omit `--no-input` or run in a TTY)"
        );
    }

    let assessment = head_assessment(home, source_url, repo_dir);
    eprintln!("[bmx][trust] source: {source_url}");
    eprintln!("[bmx][trust] repo: {}", repo_dir.display());
    eprintln!(
        "[bmx][trust] policy scope: {}",
        if global_scope {
            "<default/global>"
        } else {
            "<source-specific>"
        }
    );
    if let Some(a) = &assessment {
        eprintln!(
            "[bmx][trust] head: {} ({})",
            a.short_commit,
            if a.commit.is_empty() { "-" } else { &a.commit }
        );
        eprintln!(
            "[bmx][trust] author: {} <{}>",
            a.author_name, a.author_email
        );
        eprintln!("[bmx][trust] date: {}", a.authored_at);
        eprintln!("[bmx][trust] subject: {}", a.subject);
        if !a.sig_status.is_empty() {
            eprintln!(
                "[bmx][trust] signature: {} ({})",
                sig_status_human(&a.sig_status),
                a.sig_status
            );
        }
        if !a.sig_signer.is_empty() {
            eprintln!("[bmx][trust] signer: {}", a.sig_signer);
        }
        if !a.sig_key.is_empty() {
            eprintln!("[bmx][trust] signer key id: {}", a.sig_key);
        }
        if !a.sig_fingerprint.is_empty() {
            eprintln!("[bmx][trust] signer fingerprint: {}", a.sig_fingerprint);
        }
    }
    eprintln!("[bmx][trust] proposed keys:");
    for key in &missing {
        eprintln!("[bmx][trust]   - {key}");
    }

    let mut stderr = std::io::stderr();
    write!(
        stderr,
        "[bmx][trust] add these keys to trust policy? [y/N]: "
    )?;
    stderr.flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let confirmed = matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes");
    if !confirmed {
        eprintln!("[bmx][trust] declined key import for {source_url}");
        return Ok(());
    }

    for key in &missing {
        if global_scope {
            add_allowed_signing_key(home, key, None)?;
        } else {
            add_allowed_signing_key(home, key, Some(source_url))?;
        }
    }
    eprintln!(
        "[bmx][trust] imported {} key(s) for {}",
        missing.len(),
        source_url
    );
    Ok(())
}

/// Ask consent before proceeding with install/update/rebuild when the source is not trusted.
///
/// A source is treated as trusted only when the signer key/fingerprint matches
/// `allowed_signing_keys` in the effective trust rule. A good local signature alone
/// does not skip consent.
pub(crate) fn prompt_untrusted_source_consent(
    home: &Path,
    source_url: &str,
    repo_dir: &Path,
) -> Result<()> {
    let policy = load_policy_or_default(home)?;
    let rule = best_rule(&policy, source_url);
    if !rule.allow {
        return Ok(());
    }

    let assessment = head_assessment(home, source_url, repo_dir);
    let sig_status = assessment
        .as_ref()
        .map(|a| a.sig_status.as_str())
        .unwrap_or_default();
    let good_signature = sig_status == "G";
    let trusted_signer = signer_matches_allowed(home, source_url, rule, repo_dir);
    debug_log(&format!(
        "trust consent source={} sig_status={} good_signature={} trusted_signer={} allow={} keys={}",
        source_url,
        sig_status,
        good_signature,
        trusted_signer,
        rule.allow,
        rule.allowed_signing_keys.len()
    ));
    // Intent: local GPG/SSH trust DB alone must not skip consent; only an explicit
    // bmx allowlist match counts as trusted without prompting.
    if trusted_signer {
        return Ok(());
    }

    if crate::input_mode::trust_assume_yes() {
        eprintln!(
            "[bmx][trust] auto-consent enabled via BMX_TRUST_ASSUME_YES for untrusted source {source_url}"
        );
        return Ok(());
    }

    if crate::input_mode::no_input()
        || !std::io::stdin().is_terminal()
        || !std::io::stderr().is_terminal()
    {
        bail!(
            "untrusted source requires interactive consent (no verified signature/trusted signer): {source_url} (use BMX_TRUST_ASSUME_YES=1 for non-interactive auto-consent)"
        );
    }

    eprintln!("[bmx][trust] untrusted source assessment");
    eprintln!("[bmx][trust] source: {source_url}");
    eprintln!("[bmx][trust] repo: {}", repo_dir.display());
    if let Some(a) = &assessment {
        eprintln!(
            "[bmx][trust] head: {} ({})",
            a.short_commit,
            if a.commit.is_empty() { "-" } else { &a.commit }
        );
        eprintln!(
            "[bmx][trust] author: {} <{}>",
            a.author_name, a.author_email
        );
        eprintln!("[bmx][trust] date: {}", a.authored_at);
        eprintln!("[bmx][trust] subject: {}", a.subject);
        if !a.sig_status.is_empty() {
            eprintln!(
                "[bmx][trust] signature: {} ({})",
                sig_status_human(&a.sig_status),
                a.sig_status
            );
        } else {
            eprintln!("[bmx][trust] signature: unknown");
        }
        if !a.sig_signer.is_empty() {
            eprintln!("[bmx][trust] signer: {}", a.sig_signer);
        }
        if !a.sig_key.is_empty() {
            eprintln!("[bmx][trust] signer key id: {}", a.sig_key);
        }
        if !a.sig_fingerprint.is_empty() {
            eprintln!("[bmx][trust] signer fingerprint: {}", a.sig_fingerprint);
        }
    } else {
        eprintln!("[bmx][trust] unable to read HEAD signing metadata");
    }
    eprintln!(
        "[bmx][trust] reason: source has no verified-good signature and signer is not trusted in policy"
    );

    let mut stderr = std::io::stderr();
    write!(
        stderr,
        "[bmx][trust] continue with this untrusted source? [y/N]: "
    )?;
    stderr.flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let confirmed = matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes");
    if !confirmed {
        bail!("installation aborted by user for untrusted source {source_url}");
    }
    Ok(())
}
