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
        lib = pkgs.lib;

        rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain;

        riscv-toolchain = pkgs.pkgsCross.riscv64-musl.pkgsStatic.extend (
          final: prev: {
            musl = prev.musl.overrideAttrs (old: {
              configureFlags = old.configureFlags ++ [
                "--disable-optimize"
              ];
              hardeningDisable = [ "fortify" ];
              separateDebugInfo = false;
              dontStrip = true;
              postPatch = old.postPatch + ''
                mkdir -p $out/src
                cp -r ./ $out/src/
              '';
            });
          }
        );

        musl-riscv = riscv-toolchain.musl;

        coreutils = riscv-toolchain.coreutils.overrideAttrs (old: {
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
          riscv-toolchain.buildPackages.gcc
          riscv-toolchain.buildPackages.binutils
        ];

        commonEnv = {
          # Needed for bindgen
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        };

        userBins = [
          "${coreutils}/bin/true"
          "${coreutils}/bin/false"
          "${coreutils}/bin/echo"
        ];

        hook = ''
          rm -rf musl coreutils headers/linux_headers kernel/compiled_userspace_nix

          ln -sf ${musl-riscv}/src musl
          ln -sf ${coreutils}/src coreutils
          ln -sf ${musl-riscv.linuxHeaders}/ headers/linux_headers

          mkdir kernel/compiled_userspace_nix
          for target in ${lib.concatStringsSep " " (map (p: "'${p}'") userBins)}; do
            name="$(basename "$target")"
            ln -sf "$target" "./kernel/compiled_userspace_nix/$name"
          done
        '';

        # helper to build devShells
        mkDevShell =
          {
            extraInputs ? [ ],
          }:
          pkgs.mkShell (
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
