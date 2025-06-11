use std::{collections::HashMap, ffi::OsStr, fmt::Display, fs, path::PathBuf};

use percent_encoding::percent_decode_str;
use pulldown_cmark::{Event, LinkType, MetadataBlockKind, Options, Parser, Tag, TextMergeStream};
use thiserror::Error;
use yaml_rust2::{Yaml, YamlLoader};

use crate::link::Link;

#[derive(Debug, Clone)]
/// A path that is guaranteed to be a Markdown file
pub struct MarkdownPath {
    /// The path of the directory the document is in
    base_path: PathBuf,
    /// The path to the file
    path: PathBuf,
}

impl Eq for MarkdownPath {}
impl PartialEq for MarkdownPath {
    fn eq(&self, other: &Self) -> bool {
        self.canonicalised_path() == other.canonicalised_path()
    }
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
            Ok(MarkdownPath { base_path, path })
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

impl From<MarkdownPath> for PathBuf {
    fn from(value: MarkdownPath) -> Self {
        value.canonicalised_path()
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
    #[error("the frontmatter for the document `{path}` cannot be parsed because {reason}")]
    FrontmatterParseFailed { path: PathBuf, reason: String },
}

/// A single Markdown document
/// TODO: Implement metadata parsing
#[derive(Debug)]
pub struct Document {
    path: MarkdownPath,
    links: Vec<Link>,
    metadata: HashMap<Yaml, Yaml>,
}

impl Document {
    #[inline]
    pub fn path(&self) -> MarkdownPath {
        self.path.clone()
    }
    #[inline]
    pub fn insert_link(&mut self, link: Link) {
        self.links.push(link);
    }
    #[inline]
    pub fn insert_metadata(&mut self, key: Yaml, value: Yaml) {
        self.metadata.insert(key, value);
    }

    pub fn new(base_path: PathBuf, path: PathBuf) -> Result<Self, ParseError> {
        let path = MarkdownPath::new(base_path, path)?;
        let mut document = Document {
            path: path.clone(),
            links: Vec::new(),
            metadata: HashMap::new(),
        };
        let contents = fs::read_to_string(path.canonicalised_path()).unwrap();
        let mut options = Options::empty();
        options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
        let mut iter = TextMergeStream::new(Parser::new_ext(&contents, options)).peekable();

        while let Some(event) = iter.next() {
            match (event, iter.peek()) {
                // Parse link
                (
                    Event::Start(Tag::Link {
                        link_type: LinkType::Inline,
                        dest_url,
                        title: _,
                        id: _,
                    }),
                    Some(Event::Text(text)),
                ) => {
                    document.insert_link(Link {
                        file: path.clone(),
                        text: text.clone().into_string(),
                        url: dest_url.into_string(),
                    });
                }
                // Parse frontmatter
                (
                    Event::Start(Tag::MetadataBlock(MetadataBlockKind::YamlStyle)),
                    Some(Event::Text(text)),
                ) => {
                    let parsed = YamlLoader::load_from_str(text.clone().into_string().as_str())
                        .map_err(|e| ParseError::FrontmatterParseFailed {
                            path: path.clone().into(),
                            reason: e.to_string(),
                        })?;
                    let root =
                        parsed
                            .first()
                            .ok_or_else(|| ParseError::FrontmatterParseFailed {
                                path: path.clone().into(),
                                reason: "Cannot get the root of the frontmatter".into(),
                            })?;
                    let hash =
                        root.as_hash()
                            .ok_or_else(|| ParseError::FrontmatterParseFailed {
                                path: path.clone().into(),
                                reason: "Top-level is not a mapping".into(),
                            })?;
                    hash.iter()
                        .for_each(|(k, v)| document.insert_metadata(k.to_owned(), v.to_owned()));
                }
                _ => {}
            }
        }

        Ok(dbg!(document))
    }
    pub fn has_link_to(&self, path: &MarkdownPath) -> bool {
        self.links.iter().any(|link| link.points_to(path))
    }
}
