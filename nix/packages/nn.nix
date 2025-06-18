{
  lib,
  writeShellApplication,
  n,
  vim,
  util-linux,
  coreutils,
  shellcheck,
  template ? "${../../extras/templates/default.md}",
  name ? "nn",
}:
writeShellApplication {
  inherit name;
  runtimeInputs = [
    n
    vim
    util-linux
    coreutils
  ];

  checkPhase = ''
    runHook preCheck
    ${lib.getExe shellcheck} $target
    runHook postCheck
  '';

  text = ''
    # Exit on error
    set -e

    NOTES_DIR="''${NOTES_DIR:-''${XDG_DOCUMENTS_DIR:-$HOME/Documents}/Notes/Contents}"
    EDITOR="''${EDITOR:-${lib.getExe vim}}"

    ID="$(uuidgen)"
    DATE="$(date +"%Y-%m-%d %H\:%M")"

    # CORRECTED LINE: Use "$*" to join all arguments into a single string.
    TITLE="$*"

    # Handle the case where no title is given
    if [ -z "$TITLE" ]; then
      echo "Error: Please provide a title for the note." >&2
      exit 1
    fi

    FILE="$(${lib.getExe n} -d "$NOTES_DIR" new -t "${template}" -v "title:$TITLE,id:$ID,created:$DATE" "$TITLE")"

    if [ -z "$FILE" ]; then
      echo "Error: Failed to create new note. 'n' command returned an empty path." >&2
      exit 1
    fi

    exec "$EDITOR" "$FILE"
  '';
}
