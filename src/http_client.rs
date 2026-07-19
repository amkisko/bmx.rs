//! Shared ureq agent with timeout and response body limits.

use std::io::Read;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use ureq::Agent;

pub(crate) const DEFAULT_TIMEOUT_SECS: u64 = 30;
pub(crate) const MAX_RESPONSE_BODY_BYTES: u64 = 8 * 1024 * 1024;

fn agent() -> &'static Agent {
    static AGENT: OnceLock<Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .redirects(5)
            .build()
    })
}

pub(crate) fn require_https(url: &str) -> Result<()> {
    let lower = url.trim().to_ascii_lowercase();
    if !(lower.starts_with("https://")) {
        bail!("only https URLs are allowed for remote HTTP fetches");
    }
    Ok(())
}

pub(crate) fn http_get_limited(
    url: &str,
    set_headers: impl FnOnce(ureq::Request) -> ureq::Request,
) -> Result<(u16, String)> {
    require_https(url)?;
    let request = set_headers(agent().get(url));
    let response = request
        .call()
        .with_context(|| format!("HTTP request failed for {url}"))?;
    let status = response.status();
    let mut reader = response.into_reader().take(MAX_RESPONSE_BODY_BYTES + 1);
    let mut body = String::new();
    reader
        .read_to_string(&mut body)
        .with_context(|| format!("failed reading HTTP body for {url}"))?;
    if body.len() as u64 > MAX_RESPONSE_BODY_BYTES {
        bail!("HTTP response for {url} exceeded {MAX_RESPONSE_BODY_BYTES} byte limit");
    }
    Ok((status, body))
}

#[cfg(test)]
mod tests {
    use super::require_https;

    #[test]
    fn require_https_rejects_non_https() {
        assert!(require_https("http://example.com").is_err());
        assert!(require_https("https://example.com").is_ok());
    }
}
