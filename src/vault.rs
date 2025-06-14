use std::{collections::HashMap, fmt::Display, path::PathBuf};

use owo_colors::OwoColorize;
use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};
use serde::Serialize;
use thiserror::Error;

use crate::{document::Document, path::MarkdownPath, query::Query, search::Search};

/// A collection of notes
#[derive(Debug, Serialize)]
pub struct Vault {
    path: PathBuf,
    documents: HashMap<MarkdownPath, Document>,
}

impl Display for Vault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path = self.path().to_string_lossy().underline().bold().to_string();
        let documents: Vec<String> = self.documents().par_iter().map(|x| x.to_string()).collect();
        let mut documents = tabled::Table::new(documents);
        documents.with(tabled::settings::Style::rounded());
        write!(
            f,
            r#"{path}
{documents}
        "#
        )
    }
}

#[derive(Debug, Error)]
pub enum VaultInitialisationError {
    #[error("the directory `{path}` cannot be opened because {reason}")]
    ReadDirFailed { path: PathBuf, reason: String },
    // #[error("the file `{path}` in the vault cannot be initialised as a document because {reason}")]
    // CannotInitialiseDocument { path: PathBuf, reason: String },
}

impl Vault {
    pub fn search(&self, query: Search) -> HashMap<MarkdownPath, f32> {
        let documents = &self.documents;
        documents
            .par_iter()
            .map(|(path, doc)| {
                (
                    path,
                    query.score(
                        &doc.stripped().unwrap(),
                        documents
                            .values()
                            .map(|val| val.stripped().unwrap())
                            .collect(),
                    ),
                )
            })
            .map(|(k, v)| (k.to_owned(), v))
            .collect()
    }
    #[inline]
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }
    #[inline]
    pub fn documents(&self) -> Vec<&Document> {
        self.documents.values().collect()
    }
    #[inline]
    pub fn get_document(&self, path: &MarkdownPath) -> Option<&Document> {
        self.documents.get(path)
    }
    pub fn new(base_path: PathBuf) -> Result<Self, VaultInitialisationError> {
        let documents = base_path
            .read_dir()
            .map_err(|reason| VaultInitialisationError::ReadDirFailed {
                path: base_path.clone(),
                reason: reason.to_string(),
            })?
            .par_bridge()
            .filter_map(|path| match path {
                // TODO: Log this error. We don't want one broken file to block the initialisation
                // process, but we also might want to optionally know which file failed.
                Ok(file) => Document::new(base_path.clone(), file.path().clone()).ok(),
                // TODO: This one, too.
                Err(_) => None,
            })
            .map(|document| (document.path(), document))
            .collect();

        Ok(Vault {
            path: base_path,
            documents,
        })
    }

    /// Get the list of documents which references the given document
    pub fn find_backlinks(&self, path: &MarkdownPath) -> Vec<MarkdownPath> {
        self.documents
            .par_iter()
            .filter_map(|(_, document)| {
                if document.has_link_to(path) {
                    return Some(document.path());
                }
                None
            })
            .collect()
    }

    pub fn query(&self, query: Query) -> Vec<&Document> {
        self.documents()
            .par_iter()
            .filter(|doc| query.matches(doc))
            .map(|doc| doc.to_owned())
            .collect()
    }
}
