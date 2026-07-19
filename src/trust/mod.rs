mod commands;
mod enforce;
mod git_env;
mod list_json;
mod policy;
mod prompt;

pub(crate) use commands::{
    TrustListScope, add_allowed_signing_key, import_signing_keys_from_repo, list_policy,
    remove_allowed_signing_key, set_allow, set_require_signed_commit, show_policy_toml,
};
pub(crate) use enforce::enforce_source_trust;
pub(crate) use list_json::list_policy_json;
pub(crate) use prompt::{prompt_import_signing_keys_for_source, prompt_untrusted_source_consent};

#[cfg(test)]
mod tests;
