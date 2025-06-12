mod cli;
mod document;
mod link;
mod path;
mod vault;

use crate::{
    cli::{Args, Subcommand},
    path::MarkdownPath,
    vault::Vault,
};

fn main() {
    let args = Args::parse().unwrap();
    let vault = Vault::new(args.vault_dir.clone()).unwrap();
    // TODO: Pretty-print the results
    match args.subcommand {
        Subcommand::Inspect(path) => {
            let base_path = args.vault_dir;
            let full_path = MarkdownPath::new(base_path, path).unwrap();
            let document = vault.get_document(&full_path).unwrap();
            if args.json {
                println!("{}", serde_json::to_string(document).unwrap());
            } else {
                println!("{document:?}");
            }
        }
        Subcommand::Backlinks(path) => {
            let base_path = args.vault_dir;
            let full_path = MarkdownPath::new(base_path, path).unwrap();
            let backlinks = vault.find_backlinks(&full_path);
            if args.json {
                println!("{}", serde_json::to_string(&backlinks).unwrap());
            } else {
                println!("{backlinks:?}");
            }
        }
    }
}
