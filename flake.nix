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
        musl = riscv-toolchain.musl.overrideAttrs (old: {
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
        musl-dev = musl.dev;
      in
      with pkgs;
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
            riscv-toolchain.buildPackages.gcc
          ];
          depsTargetTarget = [
            musl
            musl-dev
          ];
          shellHook = ''
            rm -rf musl
            ln -sf ${musl}/src musl
          '';
        };
      }
    );
}
