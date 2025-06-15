{
  pkgs,
  rustPlatform,
  pkg-config,
  ...
}: let
  rustNightly = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
in
  rustPlatform.buildRustPackage rec {
    name = "n";
    version = "0.1.0";

    nativeBuildInputs = [
      rustNightly
      pkg-config
    ];
    cargo = rustNightly;
    rustc = rustNightly;

    src = ../../.;
    cargoLock.lockFile = "${src}/Cargo.lock";
    meta.mainProgram = "n";
  }
