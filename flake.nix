{
  description = "A basic Rust devshell for NixOS users developing Crio";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
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
      in
        with pkgs; {
          devShells.default = mkShell {
            buildInputs =
              [
                openssl
                pkg-config
                clang
                cargo-make
                trunk
                tinymist
                typst
                typstyle
                # it actually doesn't work, you need to install it globally
                lato
                # for extracting images
                #
                poppler_utils
                (rust-bin
                  .selectLatestNightlyWith (toolchain:
                    toolchain
                    .default
                    .override {
                      targets = ["wasm32-unknown-unknown"];
                      extensions = ["rust-src" "rust-analyzer" "clippy"];
                    }))
              ]
              ++ pkgs.lib.optionals pkg.stdenv.isDarwin [
                darwin.apple_sdk.frameworks.SystemConfiguration
              ];

            shellHook = ''
            '';
          };
        }
    );
}
