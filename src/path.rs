use owo_colors::OwoColorize;
use serde::Serialize;
use std::{
    ffi::OsStr,
    fmt::Display,
    fs,
    hash::Hash,
    path::{Path, PathBuf},
};
use thiserror::Error;
use url::Url;

use crate::percent_encode::StringExt;

#[derive(Debug, Error)]
pub enum PathError {
    #[error("the path `{path}` is not a Markdown file")]
    NotMarkdown { path: PathBuf },
    #[error("could not canonicalise the path `{path}` because {reason}")]
    CanonicalisationFailed { path: PathBuf, reason: String },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
/// A path that is guaranteed to be a Markdown file
pub struct MarkdownPath(PathBuf);
impl Serialize for MarkdownPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.path().to_string_lossy())
    }
}

impl MarkdownPath {
    pub fn new(base_path: PathBuf, path: PathBuf) -> Result<Self, PathError> {
        if path.extension().and_then(OsStr::to_str) == Some("md") {
            // TODO: Figure out a better way to encapsulate this decoding logic
            let base_path: PathBuf = base_path.to_string_lossy().percent_decode().as_ref().into();

            let leaf: PathBuf = path.to_string_lossy().percent_decode().as_ref().into();

            let joined_path = base_path.join(&leaf);
            let canonical_path =
                fs::canonicalize(&joined_path).map_err(|e| PathError::CanonicalisationFailed {
                    path: joined_path,
                    reason: e.to_string(),
                })?;
            Ok(MarkdownPath(canonical_path))
        } else {
            Err(PathError::NotMarkdown { path })
        }
    }

    #[inline]
    pub fn path(&self) -> PathBuf {
        self.0.clone()
    }

    // WARN: For testing purposes only!
    #[allow(dead_code)]
    fn new_unchecked(base_path: PathBuf, path: PathBuf) -> Result<Self, PathError> {
        if path.extension().and_then(OsStr::to_str) == Some("md") {
            // TODO: Figure out a better way to encapsulate this decoding logic
            let base_path: PathBuf = base_path.to_string_lossy().percent_decode().as_ref().into();
            let path: PathBuf = path.to_string_lossy().percent_decode().as_ref().into();

            let canonical_path = base_path.join(&path);
            Ok(MarkdownPath(canonical_path))
        } else {
            Err(PathError::NotMarkdown { path })
        }
    }
}

#[allow(dead_code)]
fn maybe_encode(path: &Path, do_encode: bool) -> PathBuf {
    if !do_encode {
        return path.to_path_buf();
    }
    let encoded = path.to_string_lossy().percent_encode().to_string();
    PathBuf::from(encoded)
}

impl Display for MarkdownPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display = self
            .path()
            .to_string_lossy()
            .bright_blue()
            .underline()
            .bold()
            .to_string();
        write!(f, "{display}")
    }
}

impl TryFrom<MarkdownPath> for Url {
    type Error = ();

    fn try_from(value: MarkdownPath) -> Result<Self, Self::Error> {
        Url::from_file_path(value.path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100_000))]
        #[test]
        /// Given the same base path and leaf, two MarkdownPaths must always be equal
        fn equivalence(
            // Random directory (can contain separators).
            base in any::<PathBuf>(),
            // Random leaf component w/out separators, then suffixed.
            stem in proptest::string::string_regex("[A-Za-z0-9_]{1,16}").unwrap(),
            // Independent toggles.
            encode_base in any::<bool>(),
            encode_leaf in any::<bool>(),
        ) {
            // Ensure the file is recognisably Markdown.
            let file = PathBuf::from(format!("{stem}.md"));
            // Compose possibly-encoded arguments.
            let b1 = maybe_encode(&base, encode_base);
            let p1 = maybe_encode(&file, encode_leaf);

            // The property under test.
            let lhs = MarkdownPath::new_unchecked(b1, p1).unwrap();
            let rhs = MarkdownPath::new_unchecked(base.clone(), file).unwrap();

            prop_assert_eq!(lhs, rhs);
        }
        #[test]

        /// Given the same base path and leaf, two MarkdownPaths must always have the same hash
        fn hash_equivalence(
            // Random directory (can contain separators).
            base in any::<PathBuf>(),
            // Random leaf component w/out separators, then suffixed.
            stem in proptest::string::string_regex("[A-Za-z0-9_]{1,16}").unwrap(),
            // Independent toggles.
            encode_base in any::<bool>(),
            encode_leaf in any::<bool>(),
        ) {
            use std::hash::DefaultHasher;
            // Ensure the file is recognisably Markdown.
            let file = PathBuf::from(format!("{stem}.md"));
            // Compose possibly-encoded arguments.
            let b1 = maybe_encode(&base, encode_base);
            let p1 = maybe_encode(&file, encode_leaf);

            // The property under test.
            let lhs = MarkdownPath::new_unchecked(b1, p1).unwrap();
            let rhs = MarkdownPath::new_unchecked(base.clone(), file).unwrap();

            prop_assert_eq!(lhs.hash(&mut DefaultHasher::new()), rhs.hash(&mut DefaultHasher::new()));
        }
    }
}
