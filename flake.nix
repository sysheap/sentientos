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
    pwndbg = {
      url = "github:pwndbg/pwndbg";
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
      pwndbg,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [
          (import rust-overlay)
        ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain;
        riscv-toolchain = pkgs.pkgsCross.riscv64-musl;
        musl-riscv = riscv-toolchain.musl.overrideAttrs (old: {
          configureFlags = (builtins.filter (flag: flag != "--enable-shared") old.configureFlags) ++ [
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
        gcc-riscv = riscv-toolchain.wrapCCWith {
          cc = riscv-toolchain.gcc;
          bintools = riscv-toolchain.wrapBintoolsWith {
            bintools = riscv-toolchain.binutils;
            libc = musl-riscv;
          };
        };
      in
      {
        devShells.default = riscv-toolchain.mkShell {
          nativeBuildInputs = with pkgs; [
            qemu
            gdb
            cargo-nextest
            tmux
            pwndbg.packages.${system}.default
            rustToolchain
            just
            gcc-riscv
          ];
          shellHook = ''
            rm -rf musl
            ln -sf ${musl-riscv}/src musl
          '';
        };
      }
    );
}
