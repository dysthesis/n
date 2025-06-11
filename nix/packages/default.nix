{
  self,
  pkgs,
  lib,
  inputs,
  ...
}: rec {
  default = zk;
  zk = pkgs.callPackage ./mdq.nix {inherit pkgs inputs lib self;};
}
