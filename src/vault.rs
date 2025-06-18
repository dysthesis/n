use std::{collections::HashMap, fmt::Display, path::PathBuf};

use owo_colors::OwoColorize;
use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};
use serde::Serialize;
use thiserror::Error;

use crate::{document::Document, link::Link, path::MarkdownPath, query::Query, search::Corpus};

/// A collection of notes
#[derive(Debug, Serialize)]
pub struct Vault {
    path: PathBuf,
    documents: HashMap<MarkdownPath, Document>,
    corpus: Corpus,
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
    #[error("cannot read the file  because {reason}")]
    ReadFileFailed { reason: String },
    #[error("the file `{path}` in the vault cannot be initialised as a document because {reason}")]
    CannotInitialiseDocument { path: PathBuf, reason: String },
}

impl Vault {
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

    pub fn resolve_link(&self, link: Link) -> Option<MarkdownPath> {
        link.to_markdown_path(self.path())
    }

    pub fn new(
        base_path: PathBuf,
    ) -> Result<(Self, Vec<VaultInitialisationError>), VaultInitialisationError> {
        let (documents, ignorable_errors): (
            HashMap<MarkdownPath, Document>,
            Vec<VaultInitialisationError>,
        ) = base_path
            .read_dir()
            .map_err(|reason| VaultInitialisationError::ReadDirFailed {
                path: base_path.clone(),
                reason: reason.to_string(),
            })?
            .par_bridge()
            .map(|path| match path {
                Ok(file) => Document::new(base_path.clone(), file.path().clone()).map_err(|e| {
                    VaultInitialisationError::CannotInitialiseDocument {
                        path: file.path(),
                        reason: e.to_string(),
                    }
                }),
                Err(e) => Err(VaultInitialisationError::ReadFileFailed {
                    reason: e.to_string(),
                }),
            })
            .fold(
                || (HashMap::new(), Vec::new()),
                |(mut res, mut err), val| {
                    match val {
                        Ok(doc) => {
                            res.insert(doc.path(), doc);
                        }
                        Err(e) => err.push(e),
                    }
                    (res, err)
                },
            )
            .reduce(
                || (HashMap::new(), Vec::new()),
                |(mut res_acc, mut err_acc), (res_curr, err_curr)| {
                    res_acc.extend(res_curr);
                    err_acc.extend(err_curr);
                    (res_acc, err_acc)
                },
            );

        // TODO: We can maybe log the error instead of entirely crashing out. Maybe we can return a
        // tuple of (Vault, Vec<VaultInitialisationError>)?
        // if !errors.is_empty() {
        //     return Err(VaultInitialisationError::Multiple { errors });
        // }

        let corpus = Corpus::new(
            documents
                .par_iter()
                .map(|(_, doc)| doc.stripped().unwrap())
                .collect(),
        );

        Ok((
            Vault {
                path: base_path,
                documents,
                corpus,
            },
            ignorable_errors,
        ))
    }

    pub fn search(&self, query: String) -> HashMap<Document, f32> {
        let documents = &self.documents;
        documents
            .par_iter()
            .map(|(_, doc)| {
                (
                    doc,
                    self.corpus
                        .score(query.as_str(), doc.stripped().unwrap().as_str()),
                )
            })
            .map(|(k, v)| (k.to_owned(), v))
            .collect()
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
