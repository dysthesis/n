use percent_encoding::percent_decode_str;
use std::{ffi::OsStr, fs, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PathError {
    #[error("the path `{path}` is not a Markdown file")]
    NotMarkdown { path: PathBuf },
    #[error("could not canonicalise the path `{path}` because {reason}")]
    CanonicalisationFailed { path: PathBuf, reason: String },
}

#[derive(Debug, Clone, Hash)]
/// A path that is guaranteed to be a Markdown file
pub struct MarkdownPath {
    /// The path of the directory the document is in
    base_path: PathBuf,
    /// The path to the file
    path: PathBuf,
    /// The full path to the document
    canonical_path: PathBuf,
}

impl Eq for MarkdownPath {}
impl PartialEq for MarkdownPath {
    fn eq(&self, other: &Self) -> bool {
        self.path() == other.path()
    }
}

impl MarkdownPath {
    pub fn new(base_path: PathBuf, path: PathBuf) -> Result<Self, PathError> {
        if path.extension().and_then(OsStr::to_str) == Some("md") {
            // TODO: Figure out a better way to encapsulate this decoding logic
            let base_path: PathBuf = percent_decode_str(base_path.to_string_lossy().as_ref())
                .decode_utf8_lossy()
                .as_ref()
                .into();
            let path: PathBuf = percent_decode_str(path.to_string_lossy().as_ref())
                .decode_utf8_lossy()
                .as_ref()
                .into();

            let joined_path = base_path.join(&path);
            let canonical_path =
                fs::canonicalize(&joined_path).map_err(|e| PathError::CanonicalisationFailed {
                    path: joined_path,
                    reason: e.to_string(),
                })?;
            Ok(MarkdownPath {
                base_path,
                path,
                canonical_path,
            })
        } else {
            Err(PathError::NotMarkdown { path })
        }
    }
    #[inline]
    pub fn base_path(&self) -> PathBuf {
        self.base_path.clone()
    }
    #[inline]
    pub fn path(&self) -> PathBuf {
        self.canonical_path.clone()
    }
}
