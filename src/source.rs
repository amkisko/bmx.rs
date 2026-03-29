use anyhow::{Result, anyhow};

use crate::types::Config;

pub(crate) fn resolve_source(cfg: &Config, app: &str) -> Result<String> {
    if looks_like_url(app) {
        return Ok(normalize_explicit_url(app));
    }

    if let Some(url) = resolve_registry_spec(cfg, app)? {
        return Ok(url);
    }

    let base = cfg.default_source.clone().ok_or_else(|| {
        anyhow!("default source is not configured; run `bmx source set-default <url>`")
    })?;

    let mut base = normalize_source_base(&base);
    if app.starts_with('/') {
        base.push_str(app);
    } else {
        base.push('/');
        base.push_str(app);
    }

    if !base.ends_with(".git") {
        base.push_str(".git");
    }

    Ok(base)
}

/// `registry_key:repo` using `[registries.registry_key]` in config (language-agnostic alias for a git host base URL).
fn resolve_registry_spec(cfg: &Config, app: &str) -> Result<Option<String>> {
    let Some((key, tail)) = app.split_once(':') else {
        return Ok(None);
    };
    if key.is_empty() || tail.is_empty() || key.contains('/') {
        return Ok(None);
    }
    let Some(base_raw) = cfg.registries.get(key) else {
        return Ok(None);
    };
    let mut base = normalize_source_base(base_raw);
    let tail = tail.trim_start_matches('/');
    if tail.starts_with('/') {
        base.push_str(tail);
    } else {
        base.push('/');
        base.push_str(tail);
    }
    if !base.ends_with(".git") {
        base.push_str(".git");
    }
    Ok(Some(base))
}

pub(crate) fn looks_like_url(app: &str) -> bool {
    app.contains("://")
        || app.starts_with("git@")
        || app.starts_with("github.com/")
        || app.starts_with("gitlab.com/")
}

pub(crate) fn normalize_explicit_url(input: &str) -> String {
    if input.starts_with("http://")
        || input.starts_with("https://")
        || input.starts_with("git://")
        || input.starts_with("ssh://")
        || input.starts_with("file://")
        || input.starts_with("ftp://")
        || input.starts_with("ftps://")
        || input.starts_with("git@")
    {
        input.to_string()
    } else {
        format!("https://{input}")
    }
}

pub(crate) fn normalize_source_base(input: &str) -> String {
    let normalized = if input.starts_with("http://")
        || input.starts_with("https://")
        || input.starts_with("git://")
        || input.starts_with("ssh://")
        || input.starts_with("file://")
        || input.starts_with("ftp://")
        || input.starts_with("ftps://")
    {
        input.to_string()
    } else {
        format!("https://{input}")
    };

    normalized.trim_end_matches('/').to_string()
}

pub(crate) fn app_id(app: &str) -> String {
    let raw = app
        .trim_end_matches(".git")
        .rsplit('/')
        .next()
        .unwrap_or(app)
        .rsplit(':')
        .next()
        .unwrap_or(app);

    raw.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

pub(crate) fn sanitize_bin_name(app: &str) -> String {
    app_id(app).replace('-', "_")
}
