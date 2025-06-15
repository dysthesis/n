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
      cargo-expand
    ]
    ++ (with self.packages.${pkgs.system}; [
      n
      ns
      nn
    ]);
}
