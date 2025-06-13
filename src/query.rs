use crate::document::Document;

pub enum Query {
    Contains { key: String, value: String },
    Not(Box<Query>),
    And(Box<Query>, Box<Query>),
    Or(Box<Query>, Box<Query>),
    Xor(Box<Query>, Box<Query>),
}

impl Query {
    pub fn matches(&self, document: &Document) -> bool {
        match self {
            Query::Contains { key, value } => document
                .get_metadata(key)
                .map_or_else(|| false, |target| target.contains(value)),
            Query::Not(query) => !query.matches(document),
            Query::And(left, right) => left.matches(document) && right.matches(document),
            Query::Or(left, right) => left.matches(document) || right.matches(document),
            Query::Xor(left, right) => left.matches(document) ^ right.matches(document),
        }
    }
}
