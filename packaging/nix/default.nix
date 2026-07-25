# Standalone default.nix for Nix (non-flake) installs.
# Usage: nix-build -A bmx (when using flake) or adapt for nix-env.

{ pkgs ? import <nixpkgs> {} }:

pkgs.rustPlatform.buildRustPackage {
  pname = "bmx";
  version = "0.1.4";
  src = ./.;
  cargoLock.lockFile = ./Cargo.lock;
}
