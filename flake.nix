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
        kani = import ./nix/kani.nix { inherit pkgs; };

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

        dash = riscv-toolchain.dash.overrideAttrs (old: {
          hardeningDisable = [ "fortify" ];
          separateDebugInfo = false;
          dontStrip = true;
        });


        basePackages = [
          pkgs.qemu
          pkgs.cargo-nextest
          pkgs.just
          (pkgs.python3.withPackages (ps: [
            ps.pygdbmi
            ps.mcp
          ]))
          rustToolchain
          riscv-toolchain.buildPackages.gcc
          riscv-toolchain.buildPackages.binutils
          kani
        ];

        commonEnv = {
          # Needed for bindgen
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        };

        hook = ''
          rm -rf musl headers/linux_headers headers/musl_headers

          ln -sf ${musl-riscv}/src musl
          ln -sf ${musl-riscv.linuxHeaders}/ headers/linux_headers
          ln -sf ${musl-riscv.dev}/include headers/musl_headers

          mkdir -p kernel/compiled_userspace_nix
          ln -sf "${dash}/bin/dash" "./kernel/compiled_userspace_nix/dash"
          ln -sf "${dash}/bin/dash" "./kernel/compiled_userspace_nix/sh"

          just mcp-server
        '';

      in
      {
        devShells.default = pkgs.mkShell (
          commonEnv
          // {
            nativeBuildInputs = [
              pkgs.gdb
              pkgs.tmux
              pwndbg.packages.${system}.default
              pkgs.typos-lsp
              pkgs.dtc
            ]
            ++ basePackages;
            shellHook = hook;
          }
        );
      }
    );
}
