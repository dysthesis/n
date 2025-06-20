use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use owo_colors::OwoColorize;
use percent_encoding::percent_decode_str;
use serde::Serialize;

use crate::{path::MarkdownPath, pos::Pos};

#[derive(Debug, Serialize, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
/// A link in a Markdown file
pub struct Link {
    text: String,
    url: String,
    position: Pos,
}

impl Link {
    pub fn new(text: String, url: String, position: Pos) -> Self {
        Self {
            text,
            url,
            position,
        }
    }
    /// Check if the link points to the given Markdown document
    pub fn points_to(&self, target: &MarkdownPath) -> bool {
        if let Some(path) = self.to_markdown_path(
            target
                .path()
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .to_path_buf(),
        ) {
            return &path == target;
        }
        false
    }

    #[inline]
    pub fn to_markdown_path(&self, base_path: PathBuf) -> Option<MarkdownPath> {
        if let Err(url::ParseError::RelativeUrlWithoutBase) = url::Url::parse(self.url.as_str()) {
            MarkdownPath::new(base_path, PathBuf::from(self.url.clone())).ok()
        } else {
            None
        }
    }
    #[inline]
    pub fn pos(&self) -> Pos {
        self.position.clone()
    }
}

impl Display for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let url = percent_decode_str(self.url.as_ref())
            .decode_utf8_lossy()
            .to_string();
        write!(f, "{}", url.bright_blue().underline())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100_000))]
        #[test]
        fn test_constructor_and_getters(
            text: String,
            url: String,
            pos: Pos
        ) {
            let link = Link::new(text.clone(), url.clone(), pos.clone());
            prop_assert_eq!(link.text.clone(), text);
            prop_assert_eq!(link.url.clone(), url);
            prop_assert_eq!(link.pos(), pos);
        }
        #[test]
        fn test_absolute_urls_are_none(
            // Generator for http/https/file URLs
            url in "(http|https|ftp)://[a-zA-Z0-9./]+",
            base_path: PathBuf,
            text: String,
            pos: Pos
        ) {
            let link = Link::new(text, url, pos);
            prop_assert!(link.to_markdown_path(base_path).is_none());
        }
        #[test]
        fn test_relative_non_md_urls_are_none(
            // Generator for relative paths not ending in .md/.markdown
            url in r"([a-zA-Z0-9./]+)\.(txt|html|png|jpg)",
            base_path: PathBuf,
            text: String,
            pos: Pos
        ) {
            let link = Link::new(text, url, pos);
            prop_assert!(link.to_markdown_path(base_path).is_none());
        }
        #[test]
        fn test_relative_md_urls_resolve(
            // Generator for relative paths ending in .md
            url in r"([a-zA-Z0-9./]+)\.md",
            base_path: PathBuf,
            text: String,
            pos: Pos
        ) {
            let link = Link::new(text, url.clone(), pos);
            let expected_path = base_path.join(url);
            // We need to canonicalize or clean the path to handle `.` and `..` correctly
            // for a robust comparison. Let's assume MarkdownPath handles this.
            let expected_md_path = MarkdownPath::new(base_path.clone(), expected_path).unwrap();

            let resolved_path = link.to_markdown_path(base_path).unwrap();

            prop_assert_eq!(resolved_path, expected_md_path);
        }
        #[test]
        fn test_points_to_simple_filename(
            target: MarkdownPath,
            text: String,
            pos: Pos
        ) {
            // Ensure target has a filename
            prop_assume!(target.path().file_name().is_some());
            let filename = target.path().file_name().unwrap().to_string_lossy().to_string();

            let link = Link::new(text, filename, pos);
            prop_assert!(link.points_to(&target));
        }
        #[test]
        fn test_points_to_complex_relative_path(
            target: MarkdownPath,
            link: Link
        ) {
            let path = target.path();
            // The base path used inside `points_to`
            let base_path = path.parent().unwrap_or_else(|| Path::new(""));

            // Manually predict the outcome
            let predicted_outcome = if let Some(resolved_md_path) = link.to_markdown_path(base_path.to_path_buf()) {
                // Normalize both paths for a fair comparison
                resolved_md_path.path() == target.path()
            } else {
                false
            };

            prop_assert_eq!(link.points_to(&target), predicted_outcome);
        }
    }
}
