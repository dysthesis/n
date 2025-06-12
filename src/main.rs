mod cli;
mod document;
mod link;
mod path;
mod vault;

use rayon::iter::{IntoParallelIterator, ParallelIterator};

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

            match path {
                Some(path) => {
                    let full_path = MarkdownPath::new(base_path, path).unwrap();
                    let document = vault.get_document(&full_path).unwrap();
                    if args.json {
                        println!("{}", serde_json::to_string(document).unwrap());
                    } else {
                        println!("{document}");
                    }
                }
                None => {
                    if args.json {
                        println!("{}", serde_json::to_string(&vault).unwrap());
                    } else {
                        println!("{vault}");
                    }
                }
            }
        }
        Subcommand::Backlinks(path) => {
            let base_path = args.vault_dir;
            let full_path = MarkdownPath::new(base_path, path).unwrap();
            let backlinks = vault.find_backlinks(&full_path);
            if args.json {
                println!("{}", serde_json::to_string(&backlinks).unwrap());
            } else {
                let formatted_links: Vec<String> = backlinks
                    .into_par_iter()
                    .map(|val| val.to_string())
                    .collect();

                let mut formatted_links = tabled::Table::new(formatted_links);
                formatted_links.with(tabled::settings::style::Style::rounded());

                println!("{formatted_links}");
            }
        }
        Subcommand::Links(path) => {
            let base_path = args.vault_dir;
            let full_path = MarkdownPath::new(base_path, path).unwrap();
            let document = vault.get_document(&full_path).unwrap();
            let links = document.links();
            if args.json {
                println!("{}", serde_json::to_string(&links).unwrap());
            } else {
                println!("{links:?}");
            }
        }
    }
}
