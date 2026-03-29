//! Integration tests for install/run **controls**: ephemeral cache (`--rm`), project pin (`.bmx/pin`),
//! verbose resolution (`-v`), `integrity_check`, `[registries]` aliases, `bmx.toml` hooks, and `shim` subcommands.
//! Complements `cli.rs` (core workflows) and `version_pinning_cli.rs` (ref pinning).

#[path = "cli_install_run_options/hooks_registry_integrity.rs"]
mod hooks_registry_integrity;
#[path = "cli_install_run_options/pin_rm_verbose.rs"]
mod pin_rm_verbose;
#[path = "cli_install_run_options/shim_rm.rs"]
mod shim_rm;
#[path = "cli_install_run_options/trust.rs"]
mod trust;
#[path = "cli_install_run_options/util.rs"]
mod util;
