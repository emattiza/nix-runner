{
  description = "Nix shell shebang utility";

  inputs.flake-utils.url = "github:numtide/flake-utils";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/release-22.05";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";

  outputs = {
    self,
    flake-utils,
    nix,
    nixpkgs,
    rust-overlay,
    ...
  } @ inputs: (
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          overlays = (builtins.attrValues self.overlays) ++ [(import rust-overlay)];
          inherit system;
        };
        rust-version = "1.65.0";
        rust = pkgs.rust-bin.stable."${rust-version}".default.override {extensions = ["rust-src"];};
      in rec {
        devShells = {
          default = (
            pkgs.mkShell {
              nativeBuildInputs = [
                pkgs.bash
                pkgs.nix
                pkgs.git
                rust
                pkgs.rust-analyzer
              ];
            }
          );
        };

        apps = rec {
          default = nix-runner;
          nix-runner = {
            type = "app";
            program = "${packages.nix-runner}/bin/nix-runner";
          };
        };

        packages = nixpkgs.lib.filterAttrs (n: v: nixpkgs.lib.isDerivation v) pkgs.nix-runner;
      }
    )
    // {
      overlays = {
        packages = import ./pkgs {inherit self inputs;};
      };
    }
  );
}
