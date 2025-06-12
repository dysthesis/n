use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};
use proptest::prelude::*;
use serde::Serialize;
use std::{
    ffi::OsStr,
    fmt::Display,
    fs,
    hash::Hash,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PathError {
    #[error("the path `{path}` is not a Markdown file")]
    NotMarkdown { path: PathBuf },
    #[error("could not canonicalise the path `{path}` because {reason}")]
    CanonicalisationFailed { path: PathBuf, reason: String },
}

impl Display for MarkdownPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.canonical_path.to_string_lossy())
    }
}

#[derive(Debug, Clone, Hash, PartialOrd, Ord)]
/// A path that is guaranteed to be a Markdown file
pub struct MarkdownPath {
    /// The path of the directory the document is in
    base_path: PathBuf,
    /// The path to the file
    leaf: PathBuf,
    /// The full path to the document
    canonical_path: PathBuf,
}
impl Serialize for MarkdownPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
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
            let leaf: PathBuf = percent_decode_str(path.to_string_lossy().as_ref())
                .decode_utf8_lossy()
                .as_ref()
                .into();

            let joined_path = base_path.join(&leaf);
            let canonical_path =
                fs::canonicalize(&joined_path).map_err(|e| PathError::CanonicalisationFailed {
                    path: joined_path,
                    reason: e.to_string(),
                })?;
            Ok(MarkdownPath {
                base_path,
                leaf,
                canonical_path,
            })
        } else {
            Err(PathError::NotMarkdown { path })
        }
    }

    #[inline]
    pub fn base(&self) -> PathBuf {
        self.base_path.clone()
    }
    #[inline]
    pub fn leaf(&self) -> PathBuf {
        self.leaf.clone()
    }
    #[inline]
    pub fn path(&self) -> PathBuf {
        self.canonical_path.clone()
    }

    // WARN: For testing purposes only!
    #[allow(dead_code)]
    fn new_unchecked(base_path: PathBuf, path: PathBuf) -> Result<Self, PathError> {
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

            let canonical_path = base_path.join(&path);
            Ok(MarkdownPath {
                base_path,
                leaf: path,
                canonical_path,
            })
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
    /// https://url.spec.whatwg.org/#fragment-percent-encode-set
    const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
    let encoded = utf8_percent_encode(&path.to_string_lossy(), FRAGMENT).to_string();
    PathBuf::from(encoded)
}

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
