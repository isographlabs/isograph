{
  description = "Dev shell for Isograph (Rust, Deno, Node)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        darwinFrameworks = pkgs.lib.optionals pkgs.stdenv.isDarwin [
          pkgs.darwin.apple_sdk.frameworks.Security
          pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          pkgs.darwin.apple_sdk.frameworks.AppKit
          pkgs.darwin.apple_sdk.frameworks.CoreServices
        ];
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            pkgs.rustc
            pkgs.cargo
            pkgs.rustfmt
            pkgs.clippy
            pkgs.pkg-config
            pkgs.openssl
            pkgs.deno
            pkgs.nodejs_20
          ] ++ darwinFrameworks;

          RUST_BACKTRACE = 1;
          OPENSSL_NO_VENDOR = 1;

          shellHook = ''
            echo "Dev shell: cargo, rustc, deno, node available."
          '';
        };
      });
}

