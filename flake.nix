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
          hardeningDisable = [ "fortify" ];
          separateDebugInfo = false;
          dontStrip = true;
          postPatch = old.postPatch + ''
            # copy sources to $out/src so gdb can find them
            mkdir -p $out/src
            cp -r ./ $out/src/
          '';
        });

        gcc-riscv = riscv-toolchain.buildPackages.wrapCCWith {
          cc = riscv-toolchain.buildPackages.gcc;
          bintools = riscv-toolchain.buildPackages.wrapBintoolsWith {
            bintools = riscv-toolchain.buildPackages.binutils;
            libc = musl-riscv;
          };
        };

        coreutils = riscv-toolchain.pkgsStatic.coreutils.overrideAttrs (old: {
          stdenv = riscv-toolchain.overrideCC riscv-toolchain.stdenv gcc-riscv;
          hardeningDisable = [ "fortify" ];
          separateDebugInfo = false;
          dontStrip = true;
          env.NIX_CFLAGS_COMPILE = old.env.NIX_CFLAGS_COMPILE + " -O0 -ggdb";
          postPatch = old.postPatch + ''
            # copy sources to $out/src so gdb can find them
            mkdir -p $out/src
            cp -r ./ $out/src/
          '';
        });

        basePackages = [
          pkgs.qemu
          pkgs.cargo-nextest
          pkgs.just
          rustToolchain
          gcc-riscv.cc
          gcc-riscv.bintools
        ];

        commonEnv = {
          # Needed for bindgen
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          COREUTILS = coreutils;
        };

        hook = ''
          rm -rf musl coreutils headers/linux_headers
          ln -sf ${musl-riscv}/src musl
          ln -sf ${coreutils}/src coreutils
          ln -sf ${musl-riscv.linuxHeaders}/ headers/linux_headers
        '';

        # helper to build devShells
        mkDevShell =
          {
            extraInputs ? [ ],
          }:
          riscv-toolchain.mkShellNoCC (
            commonEnv
            // {
              nativeBuildInputs = extraInputs ++ basePackages;
              shellHook = hook;
            }
          );
      in
      {
        devShells.default = mkDevShell {
          extraInputs = [
            pkgs.gdb
            pkgs.tmux
            pwndbg.packages.${system}.default
            pkgs.typos-lsp
          ];
        };

        devShells.ci = mkDevShell {
        };
      }
    );
}
