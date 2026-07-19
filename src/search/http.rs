use anyhow::{Result, bail};
use url::Url;

use crate::http_client::http_get_limited;

use super::consts::DEFAULT_UA;

pub(crate) fn clip(s: &str) -> String {
    s.chars().take(200).collect()
}

pub(crate) fn minimal_headers(_url: &Url) -> Vec<(&'static str, String)> {
    vec![("User-Agent", DEFAULT_UA.to_string())]
}

pub(crate) fn github_headers(_url: &Url) -> Vec<(&'static str, String)> {
    let mut h = vec![
        ("User-Agent", DEFAULT_UA.to_string()),
        ("Accept", "application/vnd.github+json".to_string()),
    ];
    if let Ok(tok) = std::env::var("GITHUB_TOKEN") {
        let tok = tok.trim();
        if !tok.is_empty() {
            h.push(("Authorization", format!("Bearer {tok}")));
        }
    }
    h
}

pub(crate) fn gitlab_headers(_url: &Url) -> Vec<(&'static str, String)> {
    let mut h = vec![("User-Agent", DEFAULT_UA.to_string())];
    if let Ok(tok) = std::env::var("GITLAB_TOKEN") {
        let tok = tok.trim();
        if !tok.is_empty() {
            h.push(("PRIVATE-TOKEN", tok.to_string()));
        }
    }
    h
}

pub(crate) fn http_get_json(
    url: &Url,
    headers: fn(&Url) -> Vec<(&'static str, String)>,
) -> Result<String> {
    let body = http_request_json(url, headers, true)?;
    Ok(body)
}

pub(crate) fn http_get_json_soft(
    url: &Url,
    headers: fn(&Url) -> Vec<(&'static str, String)>,
) -> Result<String> {
    http_request_json(url, headers, false)
}

fn http_request_json(
    url: &Url,
    headers: fn(&Url) -> Vec<(&'static str, String)>,
    bail_on_error: bool,
) -> Result<String> {
    let header_pairs = headers(url);
    let (status, body) = http_get_limited(url.as_str(), |mut req| {
        for (key, value) in &header_pairs {
            req = req.set(key, value);
        }
        req
    })?;
    if !(200..300).contains(&status) {
        if bail_on_error {
            bail!("search API returned HTTP {status}: {}", clip(&body));
        }
        bail!("HTTP {}", status);
    }
    Ok(body)
}
