use std::{ffi::OsStr, fmt::Display, fs, path::PathBuf};

use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};
use pulldown_cmark::{Event, LinkType, Parser, Tag, TextMergeStream};
use thiserror::Error;

use crate::link::Link;

#[derive(Debug, Clone, PartialEq, Eq)]
/// A path that is guaranteed to be a Markdown file
pub struct MarkdownPath {
    /// The path of the directory the document is in
    base_path: PathBuf,
    /// The path to the file
    path: PathBuf,
}

impl MarkdownPath {
    pub fn new(base_path: PathBuf, path: PathBuf) -> Result<Self, ParseError> {
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
            Ok((MarkdownPath { base_path, path }))
        } else {
            Err(ParseError::NotMarkdown { path })
        }
    }
    #[inline]
    pub fn base_path(&self) -> PathBuf {
        self.base_path.clone()
    }
    #[inline]
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }
    #[inline]
    pub fn canonicalised_path(&self) -> PathBuf {
        self.base_path().join(self.path())
    }
}

impl Display for MarkdownPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.canonicalised_path().to_string_lossy())
    }
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("the path `{path}` is not a Markdown file")]
    NotMarkdown { path: PathBuf },
}

/// A single Markdown document
/// TODO: Implement metadata parsing
#[derive(Debug)]
pub struct Document {
    path: MarkdownPath,
    links: Vec<Link>,
}

impl Document {
    #[inline]
    pub fn path(&self) -> MarkdownPath {
        self.path.clone()
    }
    pub fn new(base_path: PathBuf, path: PathBuf) -> Result<Self, ParseError> {
        /// Given the path to a file, parses it and returns all of the links contained in the file
        pub fn get_links(path: &MarkdownPath) -> Vec<Link> {
            let mut results = Vec::new();
            let document = fs::read_to_string(path.canonicalised_path()).unwrap();
            let mut iter = TextMergeStream::new(Parser::new(&document)).peekable();
            while let Some(event) = iter.next() {
                if let Event::Start(Tag::Link {
            link_type: LinkType::Inline,
            dest_url,
            title: _,
            id: _,
        }) = event
            // The label of the link is contained in the next event
            && let Some(Event::Text(text)) = iter.peek()
                {
                    results.push(Link {
                        file: path.clone(),
                        text: text.to_owned().into_string(),
                        url: dest_url.into_string(),
                    });
                }
            }

            results
        }
        let path = MarkdownPath::new(base_path, path)?;
        let links = get_links(&path);

        Ok(Document { path, links })
    }
    pub fn has_link_to(&self, path: &MarkdownPath) -> bool {
        self.links.iter().any(|link| link.points_to(path))
    }
}
