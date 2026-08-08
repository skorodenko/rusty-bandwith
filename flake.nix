{
  description = "Rust devshell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    nixpkgs-unstable.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      nixpkgs,
      nixpkgs-unstable,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        pkgs-unstable = import nixpkgs-unstable {
          inherit system;
        };
        overlays = [
          (import rust-overlay)
        ];
      in
      {
        devShells.default =
          with pkgs;
          mkShell {
            packages = [
              pkgs-unstable.rust-analyzer
              rust-bin.beta.latest.default
              pkgs-unstable.sccache

              pkg-config
              libjxl
              openssl
              alsa-lib
              wayland
              libxkbcommon
              vulkan-loader
              libGL
              udev
            ];

            shellHook = ''
              export RUST_LOG=DEBUG
              export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${
                pkgs.lib.makeLibraryPath [
                  pkgs.libxkbcommon
                  pkgs.wayland
                  pkgs.vulkan-loader
                  pkgs.libGL
                ]
              }"
            '';
          };
      }
    );
}
