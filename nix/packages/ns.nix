{
  bash,
  lib,
  writeShellScriptBin,
  vim,
  glow,
  fzf,
  jq,
  n,
}:
let
  inherit (lib) getExe;
in
writeShellScriptBin "ns" ''
  SHELL="${getExe bash}"
  EDITOR="''${EDITOR:-${getExe vim}}"
  NOTES_DIR="''${NOTES_DIR:-$HOME/Documents/Notes/Contents}"

  if   [ "$(tput colors)" -ge 256 ];  then SCORE_CLR="$(tput setaf 244)"
  elif [ "$(tput colors)" -ge 16 ];   then SCORE_CLR="$(tput setaf 8)"
  else
       SCORE_CLR="$(tput dim)"
  fi
  RESET="$(tput sgr0)"

  preview() {
    [[ $(file --mime "$1") =~ text ]] &&
      CLICOLOR_FORCE=1 COLORTERM=truecolor ${getExe glow} -p -w 100 -s dark "$1" ||
      { [[ -d "$1" ]] && eza -1 --icons -TL 2 {}; }
  }
  export -f preview

  ${getExe n} -d "$NOTES_DIR" --json search "$@" |
  ${getExe jq} -r '
    .[]
    | [
        (.combined|tostring) ,
        (.document.metadata.title.String
           // (.document.path|split("/")|last|rtrimstr(".md"))) ,
        .document.path
      ] | @tsv
  ' | sort -rg -k1,1 |
  sed -e "s/^[^\t]*/''${SCORE_CLR}&''${RESET}/" |
  ${getExe fzf} --ansi --delimiter=$'\t' \
      --with-nth=2,1 \
      --prompt='Search results ❯ ' \
      --preview 'bash -c "$(declare -f preview); preview {3}"' \
      --preview-window=right:75%:wrap \
      --bind "enter:execute($EDITOR "{3}" > /dev/tty)+abort"
''
