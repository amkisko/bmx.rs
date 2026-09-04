# RFC 0004: Trust policy

- Feature Name: trust-policy
- Type: Standards Track
- Status: Stable
- Created: 2026-08-18
- Author: Andrei Makarov
- Relates: RFC 0002, RFC 0003

## Summary

`~/.bmx/trust.toml` allow and deny source prefixes, may require signed commits, and holds signer allowlists. `integrity_check` defaults true. Hooks default off. `--trust` requires explicit consent.

## Motivation

Install from source is a trust boundary. Changing fail-closed integrity, hook defaults, or the trust file schema without a numbered RFC changes what a later `bmx <app>` will execute.

## Guide-level explanation

`bmx trust list|show|add-key|remove-key|set-signed|set-allow|import-repo|check` mutate or inspect `trust.toml`. `remove-key` has alias `revoke`.

When `trust.toml` exists, install, reinstall, and run enforce the matching rule (longest `match_prefix`, else `[default]`). `allow = false` blocks the source. `require_signed_commit = true` requires `git verify-commit HEAD`. Non-empty `allowed_signing_keys` must match the signer.

`--trust` displays HEAD review details and asks for consent before writing keys. `--no-input` does not imply trust consent; set `BMX_TRUST_ASSUME_YES=1`. `BMX_TRUST_REQUIRED=1` refuses install or run when `trust.toml` is missing.

Integrity check on run and exec compares live HEAD to `install.toml` and fails closed. Hooks in `bmx.toml` run only when `hooks_enabled = true`.

## Reference-level explanation

SSH-signed commits use per-source `allowedSignersFile` under `$BMX_HOME/trust/allowed_signers/`. bmx does not modify user git configuration. `bmx trust check` compares trusted keys against a compromised-key feed and exits non-zero on a hit.

## Security considerations

Third-party source is checked out and built on the machine. `--trust` is not silent auto-trust. Integrity check fails closed.

## Registrar

Trust file keys: `allow`, `require_signed_commit`, `allowed_signing_keys`, `match_prefix`. Env: `BMX_TRUST_ASSUME_YES`, `BMX_TRUST_REQUIRED`.

## Drawbacks

Operators must write or import trust policy. Missing `trust.toml` is open unless `BMX_TRUST_REQUIRED=1`. Compromised-key feed checks need network.

## Rationale and alternatives

Silent auto-trust would match `cargo install --git` and would skip consent. Default-off integrity would let a dirty tree run. Doing nothing leaves source-trust policy to the user gitconfig.

## Prior art

Git `allowedSignersFile` and `git verify-commit`. cargo and go install from git with no host trust file. RFC 0005 contains `bmx.run` paths under the install tree.

## Unresolved questions

Whether a second implementation of trust.toml needs a schema RFC before this file grows new keys.
