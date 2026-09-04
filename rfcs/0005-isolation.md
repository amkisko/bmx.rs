# RFC 0005: Build and run isolation

- Feature Name: isolation
- Type: Standards Track
- Status: Stable
- Created: 2026-08-18
- Author: Andrei Makarov
- Relates: RFC 0002, RFC 0003

## Summary

Build isolation and run isolation are separate. Modes are `off`, `auto`, `docker`, `podman`, and `nerdctl`. `bmx.run` paths stay under the install tree. Isolation is not a strong security sandbox.

## Motivation

A host build that writes into the working tree, or a run that executes a path outside the cache, is a different product than this CLI. Changing isolation defaults without a numbered RFC changes what `bmx <app>` executes.

## Guide-level explanation

Set `build_isolation` and `run_isolation` in `config.toml`. Persist run isolation with `bmx isolation set-default-run MODE`. Inspect with `bmx isolation show-run`. Per invocation, `--isolate-run` forces `auto` runtime isolation; `--no-isolate-run` forces host execution.

When build isolation is on, installs and updates run in the container. Image comes from `BMX_ISOLATION_IMAGE` or `ghcr.io/catthehacker/ubuntu:full-latest`. That default is Linux-oriented.

`bmx.run` and `bmx.workdir` in `bmx.toml` must be relative paths under the repo (no absolute paths or `..`).

## Reference-level explanation

Hooks stay off unless `hooks_enabled` is true (RFC 0004). Path containment under the install tree is the shipped bound. Isolation commands: `bmx isolation set-default|show|set-default-run|show-run`.

## Security considerations

Isolation is not a strong security sandbox. Path containment under the install tree is the bound this RFC claims.

## Registrar

Modes: `off`, `auto`, `docker`, `podman`, `nerdctl`. Flags: `--isolate-run`, `--no-isolate-run`. Env: `BMX_ISOLATION_IMAGE`.

## Drawbacks

Default container image is Linux-oriented. `off` remains available and is weaker. Stronger sandboxes (VM, gVisor) are out of scope.

## Rationale and alternatives

One isolation switch for build and run would force containers on every invocation. Always-on Docker would fail hosts without a runtime. Doing nothing leaves builds writing into the caller working tree.

## Prior art

GitHub Actions `container:`, cargo `--offline` in a clean tree, Flatpak and Docker run. RFC 0003 owns `config.toml` keys this RFC names.

## Unresolved questions

Whether a stronger sandbox is a new RFC rather than a default flip here.
