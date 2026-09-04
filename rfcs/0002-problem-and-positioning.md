# RFC 0002: Problem and positioning

- Feature Name: problem-and-positioning
- Type: Informational
- Status: Stable
- Created: 2026-08-17
- Updated: 2026-08-18
- Author: Andrei Makarov
- Relates: RFC 0003, RFC 0004, RFC 0005, RFC 0006

## Summary

bmx installs, builds, and runs software from source repositories. The binary is `bmx`. Managed state lives under `~/.bmx`.

## Motivation

People want to run a git repo as an app without a language-specific package manager and without leaving build artifacts in the working tree. `cargo install`, Homebrew, and ad hoc clone-and-build scripts each solve one language or one host, then disagree on where the binary lives and how to undo an install.

Install from source is a trust boundary. `~/.bmx/trust.toml` allow and deny source prefixes, may require signed commits, and holds signer allowlists. Integrity check defaults true (run and exec compare live HEAD to `install.toml` and fail closed). Hooks default off. `bmx.run` executable paths stay under the install tree.

Build and run isolation are separate (`off`, `auto`, `docker`, `podman`, `nerdctl`). Search backends (GitHub, GitLab, AUR, Homebrew) discover sources without cloning and probe for a root file bmx already recognizes. Optional `[registries]` entries in `config.toml` are git URL aliases only.

This design does not offer multi-version side-by-side installs, a shared compile cache, or pluggable registries beyond static URL prefixes. Adding any of them is a new RFC.

## Guide-level explanation

`bmx <app-or-repo>` installs if needed and runs. State is `~/.bmx/config.toml`, `apps/<app-id>/repo/`, and `apps/<app-id>/install.toml`. `bmx trust` mutates `trust.toml`. `bmx isolation set-default-run` sets run isolation. `bmx undo` and `bmx history` read the local audit log.

## Drawbacks

One home cache and no side-by-side versions. Isolation is not a strong sandbox. Search heuristics miss repos without a recognized root file.

## Rationale and alternatives

A language-agnostic CLI with one home cache keeps install layout independent of Cargo, npm, or gem. Wrapping each language's package manager would inherit that manager's cache and would still leave source-trust policy to the user. A daemon or shared compile cache would help rebuilds and would add a long-lived process this design does not run.

## Prior art

`cargo install`, Homebrew, nix, and git clone scripts. Contract detail is RFC 0003 through RFC 0006.

## Unresolved questions

Whether a second implementation of trust.toml or isolation modes needs a schema RFC (RFC 0004, RFC 0005).
