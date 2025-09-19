{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    pwndbg = {
      url = "github:pwndbg/pwndbg";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      pwndbg,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain;
        riscv-toolchain = pkgs.pkgsCross.riscv64-musl;

        musl-riscv = riscv-toolchain.musl.overrideAttrs (old: {
          configureFlags = (builtins.filter (f: f != "--enable-shared") old.configureFlags) ++ [
            "--disable-optimize"
          ];
          separateDebugInfo = false;
          dontStrip = true;
          postPatch = old.postPatch + ''
            # copy sources to $out/src so gdb can find them
            mkdir -p $out/src
            cp -r ./ $out/src/
          '';
        });

        gcc-riscv-debug = riscv-toolchain.buildPackages.wrapCCWith {
          cc = riscv-toolchain.buildPackages.gcc;
          bintools = riscv-toolchain.buildPackages.wrapBintoolsWith {
            bintools = riscv-toolchain.buildPackages.binutils;
            libc = musl-riscv;
          };
        };

        basePackages = with pkgs; [
          qemu
          cargo-nextest
          rustToolchain
          just
        ];

        commonEnv = {
          # Needed for bindgen
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        };

        # helper to build devShells
        mkDevShell =
          {
            extraInputs,
            shellHook ? "",
          }:
          riscv-toolchain.mkShellNoCC (
            commonEnv
            // {
              nativeBuildInputs = extraInputs ++ basePackages;
              inherit shellHook;
            }
          );

        hookWithMusl = ''
          rm -rf musl headers/linux_headers
          ln -sf ${musl-riscv}/src musl
          ln -sf ${musl-riscv.linuxHeaders}/ headers/linux_headers
        '';

        hookHeadersOnly = ''
          rm -rf headers/linux_headers
          ln -sf ${musl-riscv.linuxHeaders}/ headers/linux_headers
        '';
      in
      {
        devShells.default = mkDevShell {
          extraInputs = [
            pkgs.gdb
            pkgs.tmux
            pwndbg.packages.${system}.default
            gcc-riscv-debug.cc
            gcc-riscv-debug.bintools
          ];
          shellHook = hookWithMusl;
        };

        devShells.ci = mkDevShell {
          extraInputs = [
            riscv-toolchain.buildPackages.gcc
            riscv-toolchain.buildPackages.binutils
          ];
          shellHook = hookHeadersOnly;
        };
      }
    );
}
