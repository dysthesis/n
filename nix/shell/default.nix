{
  pkgs,
  self,
  ...
}:
pkgs.mkShell {
  name = "zk";

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
      zk
      zks
    ]);
}
