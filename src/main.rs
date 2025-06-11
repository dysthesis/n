use std::{ffi::OsStr, fs, path::PathBuf};

use pulldown_cmark::{Event, LinkType, Parser, Tag, TextMergeStream};
use thiserror::Error;

// WARN: For testing purpposes
static NOTES_DIR: &str = "/home/demiurge/Documents/Notes/Contents/";

#[derive(Error, Debug)]
enum ParseError {
    #[error("the path `{path}` is not a Markdown file")]
    NotMarkdown { path: PathBuf },
}

#[derive(Debug)]
/// A link in a Markdown file
struct Link {
    file: MarkdownFile,
    text: String,
    url: String,
}

#[derive(Debug, Clone)]
struct MarkdownFile(PathBuf);

impl TryFrom<PathBuf> for MarkdownFile {
    type Error = ParseError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        if value.extension().and_then(OsStr::to_str) == Some("md") {
            Ok(MarkdownFile(value))
        } else {
            Err(ParseError::NotMarkdown { path: value })
        }
    }
}

impl MarkdownFile {
    #[inline]
    pub fn path(&self) -> PathBuf {
        self.0.clone()
    }
}

impl From<MarkdownFile> for PathBuf {
    fn from(value: MarkdownFile) -> Self {
        value.0
    }
}

fn main() {
    let notes_dir = PathBuf::from(NOTES_DIR).read_dir().unwrap();
    notes_dir
        // Make sure that we're only opening markdown files...
        .filter_map(|path| match path {
            Ok(file) => MarkdownFile::try_from(file.path()).ok(),
            _ => None,
        })
        // ...extract the URLs in each file
        .flat_map(find_urls_in_file)
        // ...and print them
        .for_each(|link| {
            println!(
                "The link in file {} with text {} points to {}",
                link.file.path().to_string_lossy(),
                link.text,
                link.url
            )
        });
}

/// Given the path to a file, parses it and returns all of the links contained in the file
fn find_urls_in_file(path: MarkdownFile) -> Vec<Link> {
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
