{ pkgs }:
let
  version = "0.67.0";

  kaniToolchain = pkgs.pkgsBuildHost.rust-bin.nightly."2025-11-21".default.override {
    extensions = [ "rustc-dev" "llvm-tools" ];
  };

  src = {
    x86_64-linux = pkgs.fetchzip {
      url = "https://github.com/model-checking/kani/releases/download/kani-${version}/kani-${version}-x86_64-unknown-linux-gnu.tar.gz";
      hash = "sha256-I+GKPEWYXPZimCN79IB9dKiY8+NhP4Y8JjAS7R00XMs=";
    };
    aarch64-linux = pkgs.fetchzip {
      url = "https://github.com/model-checking/kani/releases/download/kani-${version}/kani-${version}-aarch64-unknown-linux-gnu.tar.gz";
      hash = pkgs.lib.fakeHash;
    };
  }.${pkgs.stdenv.hostPlatform.system} or (throw "kani: unsupported platform ${pkgs.stdenv.hostPlatform.system}");
in
pkgs.stdenv.mkDerivation {
  pname = "kani";
  inherit version;

  dontUnpack = true;
  dontConfigure = true;
  dontBuild = true;

  nativeBuildInputs = [ pkgs.autoPatchelfHook ];
  buildInputs = [
    pkgs.stdenv.cc.cc.lib
    kaniToolchain
  ];

  installPhase = ''
    runHook preInstall
    mkdir -p $out
    cp -r ${src}/bin ${src}/lib ${src}/library ${src}/no_core ${src}/playback $out/
    cp ${src}/rust-toolchain-version ${src}/rustc-version $out/

    chmod u+w $out/bin
    ln -s kani-driver $out/bin/cargo-kani
    ln -s kani-driver $out/bin/kani

    ln -s ${kaniToolchain} $out/toolchain
    runHook postInstall
  '';
}
