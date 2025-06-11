mod document;
mod link;
mod vault;

use std::path::PathBuf;

use crate::{document::MarkdownPath, link::Link, vault::Vault};

// WARN: For testing purpposes
static NOTES_DIR: &str = "/home/demiurge/Documents/Notes/Contents/";

fn main() {
    let vault = Vault::new(NOTES_DIR.into()).unwrap();
    vault.documents().iter().for_each(|document| {
        println!("--------------------------------------------");
        let path = document.path();
        println!("The document {} is referenced by:", &path);
        let backlinks = vault.find_backlinks(&path);
        backlinks
            .iter()
            .for_each(|backlink| println!("  - {backlink}"));
    });
}
