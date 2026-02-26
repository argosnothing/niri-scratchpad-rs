{
  description = "Niri Scratchpad";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustPlatform = pkgs.makeRustPlatform {
          cargo = pkgs.rust-bin.beta.latest.default;
          rustc = pkgs.rust-bin.beta.latest.default;
        };
      in {
        packages.default = rustPlatform.buildRustPackage {
          pname = "niri-scratchpad";
          version = "1.1.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = false;
          };
          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
          buildInputs = with pkgs; [
            openssl
          ];
        };

        devShells.default = with pkgs;
          mkShell {
            buildInputs = [
              openssl
              pkg-config
              eza
              fd

              rust-bin.beta.latest.default
              rust-bin.beta.latest.rust-src
              rust-bin.beta.latest.rust-analyzer
            ];

            shellHook = ''
              alias ls=eza
              alias find=fd

              # point rust-analyzer at std sources
              export RUST_SRC_PATH="${pkgs.rust-bin.beta.latest.rust-src}/lib/rustlib/src/rust/library"
            '';
          };
      }
    );
}
