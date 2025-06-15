{ pkgs, ... }:
pkgs.mkShell {
  name = "zk";

  packages = with pkgs; [
    nixd
    alejandra
    statix
    deadnix
    cargo
    rustToolchains.nightly
    bacon
    cargo-bloat
    cargo-audit
    cargo-expand
  ];
}
