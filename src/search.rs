use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};

/// We use the BM25 algorithm to search for the given query in the vault.
///
/// From Wikipedia:
///
/// Given a query Q containing keywords q_1 to q_n, the BM25 score of a document D is
///
/// score(D, Q) = sum of IDF(q_i) * (f(q_i, D) * (k_1 + 1)) / (f(q_i, D) + k_1 * (1 - b + b * (|D|/avgdl))) for i = 1..n,
///
/// where
///
/// - f(q_i, D) is how often the term q_i appears in document D,
/// - avgdl is the average document length in the list of documents,
/// - and IDF(q_i) is the inverse document frequency, defined as
///
/// IDF(q_i) = ln((N - n(q_i) + 0.5) / (n(q_i) + 0.5) + 1),
///
/// where
///
/// - N is the total number of notes in the vault, and
/// - n(q_i) is the total number of documents containing q_i
///
/// k_1 and b are optimisation parameters, with the usual values being k_1 in [1.2, 2.0] and b =
/// 0.75.
///
/// Because I don't know what's going on here, I'll just randomly choose k_1 as 1.6.
pub struct Search(String);

impl Search {
    pub const K1: f32 = 1.6;
    pub const B: f32 = 0.75;

    pub fn frequency(term: &str, document: &str) -> usize {
        document
            .split_whitespace()
            .par_bridge()
            .filter(|word| word == &term)
            .count()
    }

    pub fn average_length(documents: Vec<&str>) -> f32 {
        let length = documents.len();
        let sum = documents
            .par_iter()
            .map(|document| document.split_whitespace().count())
            .reduce(|| 0, |a, b| a + b);
        (length / sum) as f32
    }

    pub fn contains(term: &str, document: &str) -> bool {
        document
            .split_whitespace()
            .par_bridge()
            .any(|word| word == term)
    }

    pub fn inverse_document_frequency(term: &str, documents: Vec<&str>) -> f32 {
        // How many documents contain `term`
        let num_contains = documents
            .par_iter()
            .filter(|doc| Search::contains(term, doc))
            .count() as f32;
        (((documents.len() as f32 - num_contains + 0.5) / (num_contains + 0.5)) + 1f32).ln()
    }

    // TODO: See if there are any possible overflows due to the typecasting
    pub fn score(&self, document: &str, documents: Vec<&str>) -> f32 {
        let avgdl = Search::average_length(documents.clone());
        self.0
            // Separate the query into terms
            .split_whitespace()
            .par_bridge()
            .map(|term| {
                let idf = Search::inverse_document_frequency(term, documents.clone());
                let frequency: f32 = Search::frequency(term, document) as f32;

                idf * ((frequency * (Self::K1 + 1f32))
                    / (frequency
                        + (Self::K1
                            * (1f32 - Self::B + (Self::B * (documents.len() as f32 / avgdl))))))
            })
            .sum()
    }
}
