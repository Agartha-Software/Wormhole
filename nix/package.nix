{ pkgs, self, ... }:

let
  cargoDeps = pkgs.stdenv.mkDerivation {
    pname = "wormhole-deps";
    version = "0.0.0";

    src = self;

    doCheck = false;
    dontFixup = true;
    nativeBuildInputs = with pkgs; [ cargo rustc cacert wget ];

    buildPhase = ''
      runHook preBuild
      export CARGO_HOME=$PWD/.cargo
      cargo vendor --locked > vendor-config.toml
      runHook postBuild
    '';

    installPhase = ''
      runHook preInstall
      mkdir $out
      mv vendor $out
      mv vendor-config.toml $out
      runHook postInstall
    '';

    outputHashAlgo = "sha256";
    outputHashMode = "recursive";
    outputHash = "sha256-7bSZIKmxcEoNc+jaKgGA8RtqEO0zJeq0I5fXhQJU/bk=";
  };

in pkgs.stdenv.mkDerivation {
  pname = "wormhole";
  version = "0.1.0";

  src = self;
  cargoDeps = cargoDeps;

  buildInputs = with pkgs; [ rustc cargo fuse3 pkg-config ];

  buildPhase = ''
    runHook preBuild
    export CARGO_HOME=$PWD/.cargo
    export RUST_SRC_PATH=${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target

    export CARGO_HOME=$PWD/.cargo
    mkdir -p $CARGO_HOME
    ln -s ${cargoDeps}/vendor ./vendor
    ln -s ${cargoDeps}/vendor-config.toml $CARGO_HOME/config.toml

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
