mod cli;
mod document;
mod link;
mod path;
mod query;
mod search;
mod vault;

use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::{
    cli::{Args, Subcommand},
    path::MarkdownPath,
    query::Query,
    search::Search,
    vault::Vault,
};

fn main() {
    let args = Args::parse().unwrap();
    let vault = Vault::new(args.vault_dir.clone()).unwrap();
    // TODO: Pretty-print the results
    match args.subcommand {
        Subcommand::Search(query) => {
            let mut res: Vec<(String, f32)> = vault
                .search(Search::new(query))
                .into_par_iter()
                .filter(|(_, score)| score > &0f32)
                .map(|(k, v)| {
                    (
                        k.get_metadata(&"title".to_string())
                            .map_or_else(|| "".to_string(), |res| res.to_string()),
                        v,
                    )
                })
                .collect();
            res.sort_unstable_by(|a, b| {
                b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Greater)
            });
            let mut builder = tabled::builder::Builder::new();
            builder.push_record(["Path", "Score"]);
            res.iter()
                .for_each(|(k, v)| builder.push_record([k, &v.to_string()]));
            let mut table = builder.build();
            table.with(tabled::settings::style::Style::rounded());
            println!("{table}");
        }
        Subcommand::Query(query) => {
            let parsed_query = Query::parse(query.as_str()).unwrap();
            let results = vault.query(parsed_query);
            results
                .par_iter()
                .filter_map(|doc| doc.get_metadata(&"title".to_string()))
                .for_each(|title| println!("{title}"));
        }
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
                // Print out the whole vault if no arguments are provided
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
