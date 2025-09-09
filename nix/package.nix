{ pkgs, self, ... }:
let
  # The following sources helps downloading the custom winfsp patch
  # If winfsp in one day sourced on the official repo, could dismiss
  # this for a single derivation, like was started on this commit:
  # d968432c3c9b38ddb35da9a20e79dc0a31cf1e74
  aggregatedSource = pkgs.stdenv.mkDerivation {
    pname = "cargo-crates-dl";
    version = "0.0.0";
    src = self;
    doCheck = false;
    dontFixup = true;
    nativeBuildInputs = with pkgs; [ cargo rustc cacert wget ];
    buildPhase = ''
      runHook preBuild
      export CARGO_HOME=$PWD/.cargo
      cargo fetch --locked
      runHook postBuild
    '';
    installPhase = ''
      runHook preInstall
      mkdir $out
      cp -r . $out
      runHook postInstall
    '';
    outputHashAlgo = "sha256";
    outputHashMode = "recursive";
    # NOTE for me - find by placing pkgs.lib.fakeHash here, and doing nix build .#wormhole
    outputHash = "sha256-T+RC5V1prNFr+MHuFy8n8hI/afULr1tVQVsXx4I7UTA=";
  };
in pkgs.stdenv.mkDerivation {
  pname = "wormhole";
  version = "0.1.0";

  src = aggregatedSource;

  buildInputs = [ pkgs.rustc pkgs.cargo pkgs.fuse3 pkgs.pkg-config ];

  buildPhase = ''
    runHook preBuild
    export CARGO_HOME=$PWD/.cargo
    export RUST_SRC_PATH=${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    cargo build --frozen --release --all-features
  '';

  installPhase = ''
    mkdir -p $out/bin
    cp target/release/wormhole $out/bin/
    cp target/release/wormholed $out/bin/
  '';

  meta = {
    description = "Simple decentralized file storage";
    license = pkgs.lib.licenses.agpl3Only;
  };
}
