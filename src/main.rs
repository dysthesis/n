mod document;
mod link;
mod vault;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::vault::Vault;

// WARN: For testing purpposes
static NOTES_DIR: &str = "/home/demiurge/Documents/Notes/Contents/";

fn main() {
    let vault = Vault::new(NOTES_DIR.into()).unwrap();
    vault.documents().iter().for_each(|document| {
        println!("--------------------------------------------");
        println!(
            "The document {} is referenced by:",
            &document
                .get_metadata("title".to_string())
                .unwrap()
                .as_str()
                .unwrap()
        );
        let backlinks = vault.find_backlinks(&document.path());
        backlinks
            .par_iter()
            .map(|link| {
                vault
                    .get_document(link)
                    .unwrap()
                    .get_metadata("title".to_string())
                    .unwrap()
                    .as_str()
                    .unwrap()
            })
            .for_each(|backlink| println!("  - {backlink}"));
    });
}
