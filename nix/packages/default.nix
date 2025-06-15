{
  self,
  pkgs,
  lib,
  inputs,
  ...
}: rec {
  default = n;
  n = pkgs.callPackage ./n.nix {
    inherit
      pkgs
      inputs
      lib
      self
      ;
  };

  ns = pkgs.callPackage ./ns.nix {inherit n;};
}
