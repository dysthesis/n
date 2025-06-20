{
  pkgs,
  self,
  ...
}:
pkgs.mkShell {
  name = "n";

  packages = with pkgs;
    [
      nixd
      alejandra
      statix
      deadnix
      cargo
      rustToolchains.nightly
      bacon
      cargo-bloat
      cargo-audit
      cargo-pgo
      cargo-expand
      cargo-unused-features
      cargo-nextest
      bolt_20
    ]
    ++ (with self.packages.${pkgs.system}; [
      # n
      # ns
      # nn
    ]);
}
