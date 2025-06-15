# zk - a command-line Zettelkasten bookkeeping utility

This project is a command-line utility for managing Zettelkasten-style notes. It is intended primarily for personal use

## Roadmap

- [x] Getting links and backlinks to and from a note
- [x] Performing queries on frontmatter attributes
- [x] Perform full-text search
  - **Note:** The lexing of notes probably still needs to be improved.
- [x] Calculate the "importance" of a note based on Katz centrality or PageRank
- [ ] Creating templates for notes
- [ ] Providing an LSP
- [ ] Updating links upon renaming a note

## Examples

### Searching for a note

`zk` can be used in conjunction with `jq` and `fzf` (or other fuzzy finders of your choice) to perform a full-text search on the notes, sorted in order of relevance (based on string similarity) and importance (based on PageRank).

```bash
#!/usr/bin/env bash

EDITOR_CMD="${EDITOR:-vim}"

if   [ "$(tput colors)" -ge 256 ];  then SCORE_CLR="$(tput setaf 244)"
elif [ "$(tput colors)" -ge 16 ];   then SCORE_CLR="$(tput setaf 8)"
else
     SCORE_CLR="$(tput dim)"
fi
RESET="$(tput sgr0)"

./target/release/zk -d ~/Documents/Notes/Contents --json search "$1" |
jq -r '
  .[]
  | [
      (.combined|tostring) ,
      (.document.metadata.title.String
         // (.document.path|split("/")|last|rtrimstr(".md"))) ,
      .document.path
    ] | @tsv
' | sort -rg -k1,1 |
sed -e "s/^[^\t]*/${SCORE_CLR}&${RESET}/" |
fzf --ansi --delimiter=$'\t' \
    --with-nth=2,1 \
    --prompt='Notes ❯ ' \
    --preview 'bat --style=plain --color=always --line-range :200 {3} ' \
    --preview-window=right:75%:wrap \
    --bind "enter:execute($EDITOR_CMD \$(echo {} | cut -f3) > /dev/tty)+abort"
```

## Prior art

- [zk-org/zk](https://github.com/zk-org/zk) - a plaintext note-taking assistant, written in Go.
- [sirupsen/zk](https://github.com/sirupsen/zk) - a collection of scripts, both in shell and Ruby, for managing a Zettelkasten.
