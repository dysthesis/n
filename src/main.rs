use std::{ffi::OsStr, fs, path::PathBuf};

use pulldown_cmark::{Event, LinkType, Parser, Tag, TextMergeStream};

// WARN: For testing purpposes
static NOTES_DIR: &str = "/home/demiurge/Documents/Notes/Contents/";

struct Link {
    file: PathBuf,
    text: String,
    url: String,
}

fn main() {
    let notes_dir = PathBuf::from(NOTES_DIR).read_dir().unwrap();
    notes_dir
        // Make sure that we're only opening markdown files...
        .filter_map(|path| match path {
            Ok(file) if file.path().extension().and_then(OsStr::to_str) == Some("md") => Some(file),
            _ => None,
        })
        // ...extract the URLs in each file
        .flat_map(|file| find_urls_in_file(file.path()))
        // ...and print them
        .for_each(|link| {
            println!(
                "The link in file {} with text {} points to {}",
                link.file.to_string_lossy(),
                link.text,
                link.url
            )
        });
}

/// Given the path to a file, parses it and returns all of the links contained in the file
fn find_urls_in_file(path: PathBuf) -> Vec<Link> {
    let mut results = Vec::new();
    let document = fs::read_to_string(path.clone()).unwrap();
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
