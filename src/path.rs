use percent_encoding::{
    AsciiSet, CONTROLS, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode,
};
use proptest::prelude::*;
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

    // WARN: For testing purposes only!
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

fn maybe_encode(path: &PathBuf, do_encode: bool) -> PathBuf {
    if !do_encode {
        return path.to_path_buf();
    }
    /// https://url.spec.whatwg.org/#fragment-percent-encode-set
    const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
    let encoded = utf8_percent_encode(&path.to_string_lossy(), FRAGMENT).to_string();
    PathBuf::from(encoded)
}

proptest! {
 #[test]
    fn markdown_equivalence(
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
}
