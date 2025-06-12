mod cli;
mod document;
mod link;
mod path;
mod vault;

use crate::{cli::Args, path::MarkdownPath, vault::Vault};

fn main() {
    let args = Args::parse().unwrap();
    println!("{args:?}");
    let vault = Vault::new(args.vault_dir.clone()).unwrap();
    // println!("{vault:?}");
    match args.subcommand {
        cli::Subcommand::Inspect(path) => {
            let base_path = args.vault_dir;
            let full_path = MarkdownPath::new(base_path, path).unwrap();
            let document = dbg!(vault.get_document(&full_path)).unwrap();
            if args.json {
                println!("{}", serde_json::to_string(document).unwrap());
            } else {
                println!("{document:?}");
            }
        }
    }
}
