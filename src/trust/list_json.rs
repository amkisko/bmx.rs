use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use super::commands::TrustListScope;
use super::policy::{TrustRule, best_rule, load_policy_or_default, source_url_from_filter};

#[derive(Debug, Serialize)]
struct TrustListJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default: Option<TrustRule>,
    rules: Vec<TrustRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    effective_scope: Option<String>,
}

pub(crate) fn list_policy_json(
    home: &Path,
    scope: TrustListScope,
    app: Option<&str>,
) -> Result<String> {
    let policy = load_policy_or_default(home)?;
    let mut payload = TrustListJson {
        source: None,
        default: None,
        rules: Vec::new(),
        effective_scope: None,
    };

    if let Some(app_filter) = app {
        let source_url = source_url_from_filter(home, app_filter)?;
        payload.source = Some(source_url.clone());
        if matches!(scope, TrustListScope::All | TrustListScope::Global) {
            payload.default = Some(policy.default.clone());
        }
        if matches!(scope, TrustListScope::All | TrustListScope::Local) {
            let mut matched: Vec<TrustRule> = policy
                .rules
                .iter()
                .filter(|r| {
                    r.match_prefix
                        .as_deref()
                        .is_some_and(|prefix| source_url.starts_with(prefix))
                })
                .cloned()
                .collect();
            matched.sort_by(|a, b| {
                let al = a.match_prefix.as_deref().map(str::len).unwrap_or(0);
                let bl = b.match_prefix.as_deref().map(str::len).unwrap_or(0);
                bl.cmp(&al)
            });
            payload.rules = matched;
        }
        if matches!(scope, TrustListScope::All) {
            let effective = best_rule(&policy, &source_url);
            payload.effective_scope = Some(match effective.match_prefix.as_deref() {
                Some(prefix) => format!("local ({prefix})"),
                None => "global".to_string(),
            });
        }
        return Ok(serde_json::to_string_pretty(&payload)?);
    }

    if matches!(scope, TrustListScope::All | TrustListScope::Global) {
        payload.default = Some(policy.default.clone());
    }
    if matches!(scope, TrustListScope::All | TrustListScope::Local) {
        let mut rules = policy.rules;
        rules.sort_by(|a, b| a.match_prefix.cmp(&b.match_prefix));
        payload.rules = rules;
    }
    Ok(serde_json::to_string_pretty(&payload)?)
}
