{
  self,
  pkgs,
  lib,
  inputs,
  ...
}:
rec {
  default = zk;
  zk = pkgs.callPackage ./zk.nix {
    inherit
      pkgs
      inputs
      lib
      self
      ;
  };
}
