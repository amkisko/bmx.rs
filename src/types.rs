use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Config {
    pub(crate) default_source: Option<String>,
    #[serde(default)]
    pub(crate) build_isolation: BuildIsolation,
    #[serde(default)]
    pub(crate) checkout_backend: CheckoutBackend,
    #[serde(default)]
    pub(crate) checkout_profiles: Vec<CheckoutProfile>,
    /// When true, `bmx exec` / default run verifies repo `HEAD` matches `install.toml` `resolved_commit`.
    #[serde(default)]
    pub(crate) integrity_check: bool,
    /// Named source bases: `alias:repo` resolves like default_source (e.g. `acme = "https://git.acme.com"`).
    #[serde(default)]
    pub(crate) registries: BTreeMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_source: Some("https://github.com".to_string()),
            build_isolation: BuildIsolation::Off,
            checkout_backend: CheckoutBackend::Git,
            checkout_profiles: Vec::new(),
            integrity_check: false,
            registries: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CheckoutBackend {
    /// System `git` CLI. `git2` in TOML is a legacy alias (same behavior).
    #[default]
    #[serde(alias = "git2")]
    Git,
    Gh,
    Custom,
}

impl CheckoutBackend {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::Gh => "gh",
            Self::Custom => "custom",
        }
    }

    pub(crate) fn parse(input: &str) -> Option<Self> {
        match input {
            "git" | "git2" => Some(Self::Git),
            "gh" => Some(Self::Gh),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub(crate) struct CheckoutProfile {
    pub(crate) name: Option<String>,
    pub(crate) match_prefix: String,
    pub(crate) backend: Option<CheckoutBackend>,
    #[serde(default)]
    pub(crate) env: Vec<EnvVar>,
    pub(crate) ssh_command: Option<String>,
    pub(crate) http_proxy: Option<String>,
    pub(crate) https_proxy: Option<String>,
    pub(crate) all_proxy: Option<String>,
    pub(crate) no_proxy: Option<String>,
    pub(crate) custom_clone: Option<String>,
    pub(crate) custom_sync: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub(crate) struct EnvVar {
    pub(crate) key: String,
    pub(crate) value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum BuildIsolation {
    #[default]
    Off,
    Auto,
    Docker,
    Podman,
    Nerdctl,
}

impl BuildIsolation {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Auto => "auto",
            Self::Docker => "docker",
            Self::Podman => "podman",
            Self::Nerdctl => "nerdctl",
        }
    }

    pub(crate) fn parse(input: &str) -> Option<Self> {
        match input {
            "off" => Some(Self::Off),
            "auto" => Some(Self::Auto),
            "docker" => Some(Self::Docker),
            "podman" => Some(Self::Podman),
            "nerdctl" => Some(Self::Nerdctl),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct InstallMetadata {
    pub(crate) app: String,
    pub(crate) source_url: String,
    pub(crate) executable_rel: String,
    pub(crate) strategy: String,
    #[serde(default)]
    pub(crate) requested_ref: Option<String>,
    #[serde(default)]
    pub(crate) resolved_commit: Option<String>,
    /// Cargo workspace package passed as `cargo build -p` when set (`url:package` app spec).
    #[serde(default)]
    pub(crate) rust_package: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct BMXManifest {
    pub(crate) bmx: Option<BmxSection>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct BmxSection {
    pub(crate) run: Option<String>,
    pub(crate) workdir: Option<String>,
    /// Force a build plugin id (`rust-cargo`, `cmake`, `make`, `homebrew`, `aur`) instead of auto-detect.
    #[serde(default)]
    pub(crate) strategy: Option<String>,
    #[serde(default)]
    pub(crate) hooks: Option<BmxHooks>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub(crate) struct BmxHooks {
    /// Shell command run from repository root before `bmx exec` launches the app binary.
    #[serde(default)]
    pub(crate) pre_run: Option<String>,
    /// Shell command run from repository root after a successful install/build.
    #[serde(default)]
    pub(crate) post_install: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BuildStrategy {
    RustCargo,
    CMake,
    Make,
    Homebrew,
    Aur,
}

impl BuildStrategy {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::RustCargo => "rust-cargo",
            Self::CMake => "cmake",
            Self::Make => "make",
            Self::Homebrew => "homebrew",
            Self::Aur => "aur",
        }
    }

    pub(crate) fn parse(input: &str) -> Option<Self> {
        match input {
            "rust-cargo" => Some(Self::RustCargo),
            "cmake" => Some(Self::CMake),
            "make" => Some(Self::Make),
            "homebrew" => Some(Self::Homebrew),
            "aur" => Some(Self::Aur),
            _ => None,
        }
    }
}
