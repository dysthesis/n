use std::{collections::HashMap, fs, path::PathBuf};

use pulldown_cmark::{Event, LinkType, MetadataBlockKind, Options, Parser, Tag, TextMergeStream};
use thiserror::Error;
use yaml_rust2::{Yaml, YamlLoader};

use crate::{link::Link, path::MarkdownPath};

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("the path `{path}` is invalid because {reason}")]
    InvalidPath { path: PathBuf, reason: String },
    #[error("failed to read file `{path}` because {reason}")]
    FailedToReadFile { path: PathBuf, reason: String },
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
        let path = MarkdownPath::new(base_path.clone(), path.clone()).map_err(|e| {
            ParseError::InvalidPath {
                path: base_path.join(path),
                reason: e.to_string(),
            }
        })?;

        let mut document = Document {
            path: path.clone(),
            links: Vec::new(),
            metadata: HashMap::new(),
        };

        let contents =
            fs::read_to_string(path.path()).map_err(|e| ParseError::FailedToReadFile {
                path: path.path(),
                reason: e.to_string(),
            })?;

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
                        _file: path.clone(),
                        _text: text.clone().into_string(),
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
                            path: path.clone().path(),
                            reason: e.to_string(),
                        })?;
                    let root =
                        parsed
                            .first()
                            .ok_or_else(|| ParseError::FrontmatterParseFailed {
                                path: path.clone().path(),
                                reason: "Cannot get the root of the frontmatter".into(),
                            })?;
                    let hash =
                        root.as_hash()
                            .ok_or_else(|| ParseError::FrontmatterParseFailed {
                                path: path.clone().path(),
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
    #[inline]
    pub fn get_metadata(&self, key: String) -> Option<&Yaml> {
        self.metadata.get(&Yaml::String(key))
    }
}
