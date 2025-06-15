{
  lib,
  writeShellScriptBin,
  vim,
  fzf,
  jq,
  zk,
}: let
  inherit (lib) getExe;
in
  writeShellScriptBin "zks" ''
    EDITOR_CMD="''${EDITOR:-${getExe vim}}"

    if   [ "$(tput colors)" -ge 256 ];  then SCORE_CLR="$(tput setaf 244)"
    elif [ "$(tput colors)" -ge 16 ];   then SCORE_CLR="$(tput setaf 8)"
    else
         SCORE_CLR="$(tput dim)"
    fi
    RESET="$(tput sgr0)"

    ${getExe zk} -d ~/Documents/Notes/Contents --json search "$@" |
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
        --preview 'bat --style=plain --color=always --line-range :200 {3} ' \
        --preview-window=right:75%:wrap \
        --bind "enter:execute($EDITOR_CMD \$(echo {} | cut -f3) > /dev/tty)+abort"
  ''
