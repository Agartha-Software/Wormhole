{ pkgs, ... }:
let
  # The following sources helps downloading the custom winfsp patch
  # If winfsp in one day sourced on the official repo, could dismiss
  # this for a single derivation, like was started on this commit:
  # d968432c3c9b38ddb35da9a20e79dc0a31cf1e74
  aggregatedSource = pkgs.stdenv.mkDerivation {
    pname = "cargo-crates-dl";
    version = "0.0.0";
    src = fetchGit {
      url = "https://github.com/Agartha-Software/Wormhole";
      rev = "55c5ff8953af9f1de1a0a4cda4cf210374e85358";
    };
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
    outputHash = "sha256-VzkwFYMAs1sYcmHbLlDeYPL53NmTqFcoGSTudzcekzs=";
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
