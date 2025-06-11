use std::{fs, path::PathBuf};

use pulldown_cmark::{Event, LinkType, Parser, Tag, TextMergeStream};

fn main() {
    find_urls_in_file(PathBuf::from(
        "/home/demiurge/Documents/Notes/Contents/sel4.md",
    ));
}

fn find_urls_in_file(path: PathBuf) {
    let document = fs::read_to_string(path).unwrap();
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
            println!("Link with text {text} points to {dest_url}");
        }
    }
}
