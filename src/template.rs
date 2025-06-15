use std::{collections::HashMap, fs, io, path::PathBuf};

use once_cell::sync::Lazy;
use regex::{Captures, Regex};

#[derive(Debug)]
pub struct Template {
    text: String,
    variables: HashMap<String, String>,
}

impl Template {
    /// Initialise a new instance of `Template`. If the fields are already parsed in, then the
    /// resulting vector can be passed in. Otherwise, an empty vector is initialised and fields can
    /// be inserted with the `.add_field()` method.
    pub fn new(text: String, fields: Option<String>) -> Self {
        let fields = fields.unwrap_or_default();
        let fields = fields
            // Split the input into pairs...
            .split(",")
            // and split the pairs into keys and values
            .map(|pair| {
                let splitted: Vec<&str> = pair.split(":").collect();
                (
                    splitted.get(0).unwrap().to_string(),
                    splitted.get(1).unwrap().to_string(),
                )
            })
            .collect();
        Self {
            text,
            variables: fields,
        }
    }

    /// Replace the variables in the template with the appropriate values
    pub fn render(&self) -> String {
        /// Regex to find `{{template}}` substrings to replace
        static REGEX: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*\}\}").unwrap());
        REGEX
            .replace_all(&self.text, |caps: &Captures<'_>| {
                self.variables
                    .get(caps.get(1).unwrap().as_str())
                    .cloned()
                    .unwrap_or("".to_string())
            })
            .to_string()
    }

    /// Write the rendered result to the given file name
    pub fn write(&self, path: PathBuf) -> io::Result<()> {
        fs::write(path, self.render())
    }
}
