pub struct Template {
    text: String,
    fields: Vec<(String, String)>,
}

impl Template {
    /// Initialise a new instance of `Template`. If the fields are already parsed in, then the
    /// resulting vector can be passed in. Otherwise, an empty vector is initialised and fields can
    /// be inserted with the `.add_field()` method.
    pub fn new(text: String, fields: Option<Vec<(String, String)>>) -> Self {
        let fields = fields.unwrap_or_default();
        Self { text, fields }
    }

    /// Insert a new field to the template
    pub fn add_field(&mut self, key: String, value: String) {
        self.fields.push((key, value));
    }

    pub fn render(&self) {
        todo!()
    }
}
