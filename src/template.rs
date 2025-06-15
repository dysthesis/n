use std::{collections::HashMap, fs, io};

use once_cell::sync::Lazy;
use regex::{Captures, Regex};

pub struct Template {
    text: String,
    variables: HashMap<String, String>,
}

impl Template {
    /// Initialise a new instance of `Template`. If the fields are already parsed in, then the
    /// resulting vector can be passed in. Otherwise, an empty vector is initialised and fields can
    /// be inserted with the `.add_field()` method.
    pub fn new(text: String, fields: Option<HashMap<String, String>>) -> Self {
        let fields = fields.unwrap_or_default();
        Self {
            text,
            variables: fields,
        }
    }

    /// Insert a new field to the template
    pub fn add_field(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
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
    pub fn write(&self, file_name: String) -> io::Result<()> {
        fs::write(format!("{file_name}.txt"), self.render())
    }
}
