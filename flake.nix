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
                # copied from wgpu repo
                # necessary for building wgpu in 3rd party packages (in most cases)
                libxkbcommon
                wayland
                xorg.libX11
                xorg.libXcursor
                xorg.libXrandr
                xorg.libXi
                alsa-lib
                libGL
                libxkbcommon
                wayland
                fontconfig
                freetype
                shaderc
                directx-shader-compiler
                pkg-config
                cmake
                mold # could use any linker, needed for rustix (but mold is fast)

                libGL
                vulkan-headers
                vulkan-loader
                vulkan-tools
                vulkan-tools-lunarg
                vulkan-extension-layer
                vulkan-validation-layers # don't need them *strictly* but immensely helpful

                # necessary for developing (all of) wgpu itself
                cargo-nextest
                cargo-fuzz

                # nice for developing wgpu itself
                typos

                # if you don't already have rust installed through other means,
                # this shell.nix can do that for you with this below
                yq # for tomlq below

                # nice tools
                gdb
                rr
                evcxr
                valgrind
                renderdoc
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
            LD_LIBRARY_PATH = lib.makeLibraryPath [
              libGL
              libxkbcommon
              wayland
            ];
          };
        }
    );
}
