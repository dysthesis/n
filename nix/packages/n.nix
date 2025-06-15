{
  lib,
  pkgs,
  upx,
  rustPlatform,
  pkg-config,
  optimiseBinSize ? true,
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

    RUSTFLAGS = ["-Zlocation-detail=none"];

    postFixup = lib.optionalString optimiseBinSize ''
      ${lib.getExe upx} --best --lzma $out/bin/${name}
    '';

    src = ../../.;
    cargoLock.lockFile = "${src}/Cargo.lock";
    meta.mainProgram = "n";
  }
