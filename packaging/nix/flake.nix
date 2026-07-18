{
  description = "Command-line tool that installs, builds, and runs software from source repositories";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "bmx";
          version = "0.1.3";
          src = self;
          cargoLock.lockFile = self + "/Cargo.lock";
        };

        apps.default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/bmx";
        };
      });
}
