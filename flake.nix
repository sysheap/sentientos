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
          autoconf269
          automake115x
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

          # Needed for bindgen
          LIBCLANG_PATH = "${lib.getLib llvmPackages.libclang}/lib";

          hardeningDisable = [ "format" ];
          shellHook = ''
            export PATH="$(pwd)/toolchain/bin/bin:$PATH";
          '';
        };
      }
    );
}
