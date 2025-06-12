use std::{collections::HashMap, path::PathBuf};

use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};
use thiserror::Error;

use crate::{document::Document, path::MarkdownPath};

/// A collection of notes
pub struct Vault {
    _path: PathBuf,
    documents: HashMap<MarkdownPath, Document>,
}

#[derive(Debug, Error)]
pub enum VaultInitialisationError {
    #[error("the directory `{path}` cannot be opened because {reason}")]
    ReadDirFailed { path: PathBuf, reason: String },
    // #[error("the file `{path}` in the vault cannot be initialised as a document because {reason}")]
    // CannotInitialiseDocument { path: PathBuf, reason: String },
}

impl Vault {
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
            _path: base_path,
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
}
