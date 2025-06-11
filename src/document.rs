use std::{ffi::OsStr, fmt::Display, fs, path::PathBuf};

use pulldown_cmark::{Event, LinkType, Parser, Tag, TextMergeStream};
use thiserror::Error;

use crate::link::Link;

#[derive(Debug, Clone, PartialEq, Eq)]
/// A path that is guaranteed to be a Markdown file
pub struct MarkdownPath(PathBuf);

impl TryFrom<PathBuf> for MarkdownPath {
    type Error = ParseError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        if value.extension().and_then(OsStr::to_str) == Some("md") {
            Ok(MarkdownPath(value))
        } else {
            Err(ParseError::NotMarkdown { path: value })
        }
    }
}

impl From<MarkdownPath> for PathBuf {
    fn from(value: MarkdownPath) -> Self {
        value.0
    }
}

impl Display for MarkdownPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_string_lossy())
    }
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("the path `{path}` is not a Markdown file")]
    NotMarkdown { path: PathBuf },
}

/// A single Markdown document
/// TODO: Implement metadata parsing
pub struct Document {
    path: MarkdownPath,
    links: Vec<Link>,
}

impl Document {
    #[inline]
    pub fn path(&self) -> MarkdownPath {
        self.path.clone()
    }
    pub fn new(path: &PathBuf) -> Result<Self, ParseError> {
        /// Given the path to a file, parses it and returns all of the links contained in the file
        pub fn get_links(path: &MarkdownPath) -> Vec<Link> {
            let mut results = Vec::new();
            let document = fs::read_to_string(path.0.clone()).unwrap();
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
        let path = MarkdownPath::try_from(path.to_path_buf())?;
        let links = get_links(&path);

        Ok(Document { path, links })
    }
    pub fn has_link_to(&self, path: &MarkdownPath) -> bool {
        self.links.iter().any(|link| link.points_to(path))
    }
}
