{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };
  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain;
        riscv-toolchain = import nixpkgs {
          localSystem = "${system}";
          crossSystem = {
            config = "riscv64-unknown-linux-musl";
          };
        };
        buildInputs = with pkgs; [
          qemu
          gdb
          cargo-nextest
          tmux
        ];
        nativeBuildInputs = with pkgs; [
          rustToolchain
          riscv-toolchain.buildPackages.gcc
          just
        ];
      in
      with pkgs;
      {
        devShells.default = mkShell {
          inherit buildInputs nativeBuildInputs;
        };
      }
    );
}
