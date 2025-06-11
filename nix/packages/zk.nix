{
  self,
  lib,
  pkgs,
  rustPlatform,
  symlinkJoin,
  cargo,
  rustc,
  luajit,
  pkg-config,
  makeWrapper,
  buildEnv,
  ...
}:
rustPlatform.buildRustPackage rec {
  name = "zk";
  version = "0.1.0";

  nativeBuildInputs = [
    cargo
    rustc
    pkg-config
  ];

  src = ../../.;
  cargoLock.lockFile = "${src}/Cargo.lock";
}
