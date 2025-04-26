{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
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

        buildInputs = with pkgs; [
          qemu
          gdb
          cargo-nextest
          tmux
          pkg-config
          libmpc
          mpfr
          gmp
          texinfo
          flex
          bison
          isl
        ];
        nativeBuildInputs = with pkgs; [
          rustToolchain
          just
          bash
          gnumake
        ];
      in
      with pkgs;
      {
        devShells.default = mkShell {
          inherit buildInputs nativeBuildInputs;

          hardeningDisable = [ "format" ];
          shellHook = ''
            export PATH="$(pwd)/toolchain/binutils-bin/bin:$(pwd)/toolchain/gcc-bin/bin:$PATH";
          '';
        };
      }
    );
}
