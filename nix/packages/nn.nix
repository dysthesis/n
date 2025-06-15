{
  lib,
  writeShellScriptBin,
  n,
  vim,
}: let
  inherit (lib) getExe;
in
  writeShellScriptBin "nn" ''
    NOTES_DIR="''${NOTES_DIR:-$HOME/Documents/Notes/Contents}"
    EDITOR="''${EDITOR:-${getExe vim}}"
    ID="$(date +"%Y%m%d%H%M")"
    $EDITOR "$(${getExe n} -d "$NOTES_DIR" new -t "${../../extras/templates/default.md}" -v "title:$@,id:$ID,date:$(date)" "$ID - $@")"
  ''
