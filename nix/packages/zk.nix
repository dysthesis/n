{
  rustPlatform,
  cargo,
  rustc,
  pkg-config,
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
