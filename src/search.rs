use std::collections::{HashMap, HashSet};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Serialize;

/// We use the BM25 algorithm to search for the given query in the vault.
///
/// From Wikipedia:
///
/// Given a query Q containing keywords q_1 to q_n, the BM25 score of a document D is
///
/// score(D, Q) = sum of IDF(q_i)
///                 * (f(q_i, D) * (k_1 + 1))
///                     / (f(q_i, D) + k_1 * (1 - b + b * (|D| / avgdl)))
///
/// for i = 1..n, where
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
/// References:
///
/// - https://en.wikipedia.org/wiki/Okapi_BM25
/// - https://emschwartz.me/understanding-the-bm25-full-text-search-algorithm/
#[derive(Serialize, Debug)]
pub struct Corpus {
    docs: Vec<String>,
    avgdl: f32,
    idf: HashMap<String, f32>,
}

impl Corpus {
    /// Because I don't know what's going on here, I'll just randomly choose k_1 as 1.6.
    pub const K1: f32 = 1.6;
    pub const B: f32 = 0.75;

    /// Initilise a new corpus and calculate its statistics
    // NOTE: Figure out if we can guarantee that this document is definitely found in the corpus
    pub fn new(docs: Vec<String>) -> Self {
        // Find the average length of a document in the corpus
        let avgdl = docs
            .iter()
            .map(|doc| doc.split_whitespace().count() as f32)
            .sum::<f32>()
            / docs.len() as f32;

        // Calculate the document frequency
        let df = docs
            .par_iter()
            // Normalise the text to make it case-insensitive, and flatten it into a set of all
            // tokens
            .flat_map(|doc| {
                doc.split_whitespace()
                    .map(str::to_ascii_lowercase)
                    .collect::<HashSet<_>>()
            })
            // Calculate the occurrence of each token
            .fold(HashMap::new, |mut acc: HashMap<String, f32>, curr| {
                *acc.entry(curr).or_default() += 1f32;
                acc
            })
            .reduce(HashMap::new, |mut a, b| {
                b.iter().for_each(|(k, v)| {
                    *a.entry(k.to_string()).or_default() += v;
                });
                a
            });

        // Calculate the inverse document frequency of each token from the document frequency
        let idf = df
            .into_iter()
            .map(|(term, num_occurrence)| {
                let num_docs = docs.len() as f32;
                let idf = ((num_docs - num_occurrence + 0.5 / (num_occurrence + 0.5)) + 1.0).ln();
                (term, idf)
            })
            .collect();
        Self { docs, avgdl, idf }
    }

    /// Calculate the BM25 score of a `document` given the `query`
    pub fn score(&self, query: &str, document: &str) -> f32 {
        let document_length = document.split_whitespace().count() as f32;
        let norm = Self::K1 * (1f32 - Self::B + Self::B * document_length / self.avgdl);

        // Find out how many times each term shows up in the given document
        let tf: HashMap<&str, usize> = document.split_whitespace().fold(
            HashMap::new(),
            |mut frequencies: HashMap<&str, usize>, term| {
                *frequencies.entry(term).or_default() += 1;
                frequencies
            },
        );

        // Calculate the BM25 score of each term in the query
        query
            .split_whitespace()
            .map(|term| {
                let frequency = *tf.get(term).unwrap_or(&0) as f32;
                let idf = *self.idf.get(term).unwrap_or(&0f32);
                idf * ((frequency * (Self::K1 + 1f32)) / (frequency + norm))
            })
            .sum()
    }
}
