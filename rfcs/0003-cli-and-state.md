# RFC 0003: CLI and managed state

- Feature Name: cli-and-state
- Type: Standards Track
- Status: Stable
- Created: 2026-08-18
- Author: Andrei Makarov
- Relates: RFC 0002, RFC 0004, RFC 0005

## Summary

The binary is `bmx`. Invoking `bmx <app-or-repo>` installs if needed and runs. Managed state lives under `~/.bmx`. `bmx undo` reverses the top undoable frame. Uninstall and self-update are not undoable.

## Motivation

Language-specific installers disagree on where a binary lives and how to undo an install. A silent change to `apps/<id>/install.toml` or `config.toml` keys breaks every later `bmx <app>`.

## Guide-level explanation

`bmx <app>` installs and runs. State files: `~/.bmx/config.toml`, `apps/<app-id>/repo/`, `apps/<app-id>/install.toml`. Optional: `audit.log.jsonl`, `undo-stack.json`, `snapshots/`. Project pins live under `.bmx/pins.toml` (legacy `.bmx/pin`).

Commands include `install`, `rebuild`, `uninstall`, `reinstall`, `self-update`, `update`, `show`, `search`, `history`, `undo`, `source`, `isolation`, `checkout`, `trust`, `shim`, `doctor`, `clean`, and `exec`.

`--rm` uses a throwaway `BMX_HOME` and still copies `trust.toml`.

## Reference-level explanation

`install.toml` records `app`, `source_url`, `executable_rel`, `strategy`, `requested_ref`, `resolved_commit`. `config.toml` keys include `default_source`, `build_isolation`, `run_isolation`, `checkout_backend`, `integrity_check`, `hooks_enabled`, and optional `[registries]`. Isolation is RFC 0005. Trust is RFC 0004. Search is RFC 0006.

## Registrar

State files: `config.toml`, `install.toml`, `trust.toml`, `audit.log.jsonl`, `undo-stack.json`. Home: `~/.bmx` or `BMX_HOME`.

## Drawbacks

One home cache is a single machine-local store. Undo cannot reverse uninstall or self-update. Legacy `.bmx/pin` must keep parsing.

## Rationale and alternatives

Wrapping cargo, brew, or gem would inherit those caches and still leave undo to the user. A daemon would help rebuilds and would add a long-lived process. Doing nothing leaves clone-and-build scripts with no install record.

## Prior art

`cargo install`, Homebrew, and nix profiles each own a cache layout. This RFC keeps a language-agnostic home directory. RFC 0002 rejects becoming a language package manager.

## Unresolved questions

Whether a compiled command list stays in SPEC.md or later cites RFCs per command family only.
