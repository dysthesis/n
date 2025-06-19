use std::{collections::BTreeMap, fmt::Display, fs, hash::Hash, path::PathBuf};

use dashmap::DashSet;
use owo_colors::OwoColorize;
use pulldown_cmark::{
    Event, LinkType, MetadataBlockKind, Options, Parser, Tag, TagEnd, TextMergeStream,
    TextMergeWithOffset,
};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use ropey::Rope;
use serde::Serialize;
use tabled::Tabled;
use thiserror::Error;
use tower_lsp::lsp_types::PositionEncodingKind;
use yaml_rust2::{Yaml, YamlLoader};

use crate::{
    link::Link,
    path::MarkdownPath,
    pos::{Col, Pos, Row},
};

// See if there are any better solutions, and whether we want to use it in the first place.
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
    #[error("failed to get the position because {reason}")]
    PositionNotFound { reason: String },
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

impl Value {
    pub fn contains(&self, needle: &str) -> bool {
        match self {
            Value::Real(val) | Value::String(val) => val == needle,
            Value::Integer(n) => needle.parse::<i64>().map(|m| m == *n).unwrap_or(false),
            Value::Boolean(b) => needle.parse::<bool>().map(|m| m == *b).unwrap_or(false),
            Value::Alias(idx) => needle.parse::<usize>().map(|m| m == *idx).unwrap_or(false),
            Value::Array(values) => values.iter().any(|v| v.contains(needle)),
            Value::Hash(map) => map
                .iter()
                .any(|(k, v)| k.contains(needle) || v.contains(needle)),
            Value::Null | Value::Bad => false,
        }
    }
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
    links: DashSet<Link>,
    metadata: HashMap<String, Value>,
    #[serde(skip_serializing)]
    pub rope: Rope,
}

impl Document {
    #[inline]
    pub fn name(&self) -> String {
        let file_name = self
            .path
            .path()
            .file_stem()
            // A MarkdownPath is guaranteed to exist in the filesystem, at least at the time of
            // creation. This might be susceptible to TOCTOU bugs, though.
            .expect("the file should have a name")
            .to_string_lossy()
            .to_string();
        self.get_metadata(&"title".to_string())
            .map_or_else(|| file_name, |val| val.to_string().to_string())
    }
    #[inline]
    pub fn path(&self) -> MarkdownPath {
        self.path.clone()
    }
    #[inline]
    pub fn insert_link(&mut self, link: Link) {
        self.links.insert(link);
    }
    #[inline]
    pub fn links(&self) -> DashSet<Link> {
        self.links.clone()
    }

    #[inline]
    pub fn get_link_at(&self, row: Row, col: Col) -> Option<Link> {
        self.links().into_iter().find(|link: &Link| {
            // TODO: How do you make a closure async?
            // self.client
            //     .log_message(MessageType::INFO, format!("Checking link {:?}", &link))
            //     .await;
            let row_range: std::ops::Range<Row> = link.pos().row_range();
            let col_range: std::ops::Range<Col> = link.pos().col_range();

            // TODO: Use `.try_into()` instead of `as`, and implement en appropriate error
            // variant for it.
            // Or better yet, refactor Pos to keep track of u32 instead of usize
            row_range.start <= row
                && row_range.end >= row
                && col_range.start <= col
                && col_range.end >= col
        })
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

    pub fn stripped(&self) -> Result<String, ParseError> {
        let contents = self.rope.to_string();
        let mut res = String::new();

        let mut options = Options::empty();
        options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_MATH);
        let mut iter = TextMergeStream::new(Parser::new_ext(&contents, options));

        while let Some(event) = iter.next() {
            match event {
                Event::Text(t) => res.push_str(format!("{t} ").as_str()),
                Event::SoftBreak => res.push(' '),
                Event::HardBreak | Event::Rule => res.push('\n'),
                // Skip unwanted events
                Event::Start(
                    Tag::MetadataBlock(_)
                    | Tag::Superscript
                    | Tag::Subscript
                    | Tag::FootnoteDefinition(_)
                    | Tag::CodeBlock(_)
                    | Tag::Table(_)
                    | Tag::Image {
                        link_type: _,
                        dest_url: _,
                        title: _,
                        id: _,
                    },
                ) => {
                    while let Some(event) = iter.next()
                        && !matches!(event, Event::End(_))
                    {}
                }
                _ => {}
            }
        }

        Ok(res)
    }

    pub fn parse(&mut self) -> Result<(), ParseError> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
        let text = self.rope.to_string();
        let mut iter = TextMergeWithOffset::new(Parser::new_ext(&text, options).into_offset_iter());

        while let Some(event) = iter.next() {
            match event {
                // Parse link
                (
                    Event::Start(Tag::Link {
                        link_type: LinkType::Inline,
                        dest_url,
                        title: _,
                        id: _,
                    }),
                    range,
                ) => {
                    if let Some((Event::Text(text), _)) = iter.next()
                        && let Some((Event::End(TagEnd::Link), _)) = iter.next()
                    {
                        let position =
                            Pos::new(range, &self.path().path(), PositionEncodingKind::UTF16)
                                .map_err(|e| ParseError::PositionNotFound {
                                    reason: e.to_string(),
                                })?;

                        self.insert_link(Link::new(
                            text.clone().into_string(),
                            dest_url.into_string(),
                            position,
                        ));
                    }
                }
                // Parse frontmatter
                (Event::Start(Tag::MetadataBlock(MetadataBlockKind::YamlStyle)), _) => {
                    if let Some((Event::Text(text), _)) = iter.next() {
                        let parsed = YamlLoader::load_from_str(text.clone().into_string().as_str())
                            .map_err(|e| ParseError::FrontmatterParseFailed {
                                path: self.path().path(),
                                reason: e.to_string(),
                            })?;
                        let root =
                            parsed
                                .first()
                                .ok_or_else(|| ParseError::FrontmatterParseFailed {
                                    path: self.path().path(),
                                    reason: "Cannot get the root of the frontmatter".into(),
                                })?;
                        let hash =
                            root.as_hash()
                                .ok_or_else(|| ParseError::FrontmatterParseFailed {
                                    path: self.path().path(),
                                    reason: "Top-level is not a mapping".into(),
                                })?;
                        hash.iter().for_each(|(k, v)| {
                            _ = self.insert_metadata(k.to_owned(), v.to_owned());
                        });
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn new(base_path: PathBuf, path: PathBuf) -> Result<Self, ParseError> {
        let parsed_path = MarkdownPath::new(base_path.clone(), path.clone()).map_err(|e| {
            ParseError::InvalidPath {
                path: base_path.join(path.clone()),
                reason: e.to_string(),
            }
        })?;

        let contents =
            fs::read_to_string(parsed_path.path()).map_err(|e| ParseError::FailedToReadFile {
                path: parsed_path.path(),
                reason: e.to_string(),
            })?;

        let rope = Rope::from_str(&contents);

        let mut document = Document {
            path: parsed_path.clone(),
            links: DashSet::new(),
            metadata: HashMap::new(),
            rope,
        };
        let _ = document.parse();

        Ok(document)
    }
    pub fn has_link_to(&self, path: &MarkdownPath) -> bool {
        self.links.iter().any(|link| link.points_to(path))
    }
    #[inline]
    pub fn get_metadata(&self, key: &String) -> Option<&Value> {
        self.metadata.get(key)
    }
    #[inline]
    pub fn metadata(&self) -> HashMap<String, Value> {
        self.metadata.clone()
    }
}

impl Display for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Tabled)]
        struct Row {
            key: String,
            value: String,
        }

        // Format metadata into a table
        let rows: Vec<Row> = self
            .metadata()
            .into_iter()
            .map(|(key, value)| Row {
                key,
                value: value.to_string(),
            })
            .collect();
        let mut formatted_metadata = tabled::Table::new(rows);
        formatted_metadata.with(tabled::settings::Style::rounded());

        // Format links into a table
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
