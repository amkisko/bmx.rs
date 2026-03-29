use url::Url;

use crate::types::Config;

pub(crate) fn config_with_github_base(cfg: &Config) -> Config {
    let mut c = cfg.clone();
    c.default_source = Some("https://github.com".to_string());
    c
}

pub(crate) fn config_with_gitlab_base(cfg: &Config, site: &Url) -> Config {
    let mut c = cfg.clone();
    let scheme = site.scheme();
    let host = site.host_str().unwrap_or("");
    let port = site.port().map(|p| format!(":{p}")).unwrap_or_default();
    c.default_source = Some(format!("{scheme}://{host}{port}"));
    c
}

pub(crate) fn gitlab_api_v4_root(site: &Url) -> String {
    let scheme = site.scheme();
    let host = site.host_str().unwrap_or("");
    let port = site.port().map(|p| format!(":{p}")).unwrap_or_default();
    format!("{scheme}://{host}{port}/api/v4")
}

pub(crate) fn github_qualifiers(q: &str, include_forks: bool) -> String {
    let mut out = q.trim().to_string();
    if !out.contains("archived:") {
        out.push_str(" archived:false");
    }
    if !include_forks && !out.contains("fork:") {
        out.push_str(" fork:false");
    }
    out
}
