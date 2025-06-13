use std::{collections::BTreeMap, fmt::Display, fs, hash::Hash, path::PathBuf};

use owo_colors::OwoColorize;
use pulldown_cmark::{Event, LinkType, MetadataBlockKind, Options, Parser, Tag, TextMergeStream};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::Serialize;
use thiserror::Error;
use yaml_rust2::{Yaml, YamlLoader};

use crate::{link::Link, path::MarkdownPath};

type HashMap<K, V> = BTreeMap<K, V>;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("the path `{path}` is invalid because {reason}")]
    InvalidPath { path: PathBuf, reason: String },
    #[error("failed to read file `{path}` because {reason}")]
    FailedToReadFile { path: PathBuf, reason: String },
    #[error("the frontmatter for the document `{path}` cannot be parsed because {reason}")]
    FrontmatterParseFailed { path: PathBuf, reason: String },
    #[error("the key of the YAML frontmatter must be a string. `{key:?}` was received instead")]
    KeyIsNotString { key: Value },
}

#[derive(Clone, Debug, Serialize, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum Value {
    Real(String),
    Integer(i64),
    String(String),
    Boolean(bool),
    Array(Vec<Value>),
    Hash(BTreeMap<Value, Value>),
    Alias(usize),
    Null,
    Bad,
}
impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display_str = match self {
            Value::Real(val) => val,
            Value::Integer(val) => &val.to_string(),
            Value::String(val) => val,
            Value::Boolean(val) => &val.to_string(),
            Value::Array(values) => {
                let formatted: Vec<String> = values.par_iter().map(|val| val.to_string()).collect();
                let mut table = tabled::Table::new(formatted);
                table.with(tabled::settings::style::Style::rounded());
                &table.to_string()
            }
            Value::Hash(btree_map) => {
                let formatted: HashMap<String, String> = btree_map
                    .par_iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                let mut table = tabled::Table::new(formatted);
                table.with(tabled::settings::style::Style::rounded());
                &table.to_string()
            }
            Value::Alias(val) => &val.to_string(),
            Value::Null => &"null".dimmed().to_string(),
            Value::Bad => &"bad value".bright_red().to_string(),
        };
        write!(f, "{display_str}")
    }
}

impl From<Yaml> for Value {
    fn from(value: Yaml) -> Self {
        match value {
            Yaml::Real(val) => Self::Real(val),
            Yaml::Integer(val) => Self::Integer(val),
            Yaml::String(val) => Self::String(val),
            Yaml::Boolean(val) => Self::Boolean(val),
            Yaml::Array(values) => {
                Self::Array(values.into_par_iter().map(|value| value.into()).collect())
            }
            Yaml::Hash(map) => {
                let map = map.into_iter().map(|(k, v)| (k.into(), v.into())).fold(
                    HashMap::new(),
                    |mut acc, (k, v): (Value, Value)| {
                        acc.insert(k, v);
                        acc
                    },
                );
                Self::Hash(map)
            }
            Yaml::Alias(val) => Self::Alias(val),
            Yaml::Null => Self::Null,
            Yaml::BadValue => Self::Bad,
        }
    }
}

/// A single Markdown document
/// TODO: Implement metadata parsing
#[derive(Debug, Serialize, Clone)]
pub struct Document {
    path: MarkdownPath,
    links: Vec<Link>,
    metadata: HashMap<String, Value>,
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
    pub fn links(&self) -> Vec<Link> {
        self.links.clone()
    }
    #[inline]
    pub fn insert_metadata(&mut self, key: Yaml, value: Yaml) -> Result<(), ParseError> {
        let key = if let Yaml::String(val) = key {
            Ok(val)
        } else {
            Err(ParseError::KeyIsNotString { key: key.into() })
        }?;
        self.metadata.insert(key, value.into());
        Ok(())
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
                    hash.iter().for_each(|(k, v)| {
                        _ = document.insert_metadata(k.to_owned(), v.to_owned());
                    });
                }
                _ => {}
            }
        }

        Ok(document)
    }
    pub fn has_link_to(&self, path: &MarkdownPath) -> bool {
        self.links.iter().any(|link| link.points_to(path))
    }
    #[inline]
    pub fn get_metadata(&self, key: String) -> Option<&Value> {
        self.metadata.get(&key)
    }
    #[inline]
    pub fn metadata(&self) -> HashMap<String, Value> {
        self.metadata.clone()
    }
}

impl Display for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatted_metadata: HashMap<String, String> = self
            .metadata()
            .into_par_iter()
            .map(|(k, v)| (k, v.to_string()))
            .collect();
        let mut formatted_metadata = tabled::Table::new(formatted_metadata);
        formatted_metadata.with(tabled::settings::style::Style::rounded());

        let formatted_links: Vec<String> = self
            .links()
            .into_par_iter()
            .map(|val| val.to_string())
            .collect();
        let mut formatted_links = tabled::Table::new(formatted_links);
        formatted_links.with(tabled::settings::style::Style::rounded());
        let display = format!(
            r#"{}
Metadata:
{}

Links:
{}"#,
            self.path(),
            formatted_metadata,
            formatted_links
        );
        write!(f, "{display}")
    }
}
