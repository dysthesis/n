use std::path::PathBuf;

use serde::Serialize;

use crate::path::MarkdownPath;

#[derive(Debug, Serialize, Clone)]
/// A link in a Markdown file
pub struct Link {
    pub _file: MarkdownPath,
    pub _text: String,
    pub url: String,
}

impl Link {
    /// Check if the link points to the given Markdown document
    pub fn points_to(&self, target: &MarkdownPath) -> bool {
        // If url cannot parse thsi link, it's either broken or points to a local file...
        if let Err(url::ParseError::RelativeUrlWithoutBase) = url::Url::parse(self.url.as_str())
            // ...and if we can parse it as a MarkdownPath, it's probably a markdown path.
            && let Ok(path) = MarkdownPath::new(target.base(), PathBuf::from(self.url.clone()))
        {
            return &path == target;
        }
        false
    }
}
