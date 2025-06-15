{
  self,
  pkgs,
  lib,
  inputs,
  ...
}: rec {
  default = zk;
  zk = pkgs.callPackage ./zk.nix {
    inherit
      pkgs
      inputs
      lib
      self
      ;
  };

  zks = pkgs.callPackage ./zks.nix {inherit zk;};
}
