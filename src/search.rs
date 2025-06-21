//! We use the BM25 algorithm to search for the given query in the vault.
//!
//! From Wikipedia:
//!
//! Given a query Q containing keywords q_1 to q_n, the BM25 score of a document D is
//!
//! score(D, Q) = sum of IDF(q_i)
//!                 * (f(q_i, D) * (k_1 + 1))
//!                     / (f(q_i, D) + k_1 * (1 - b + b * (|D| / avgdl)))
//!
//! for i = 1..n, where
//!
//! - f(q_i, D) is how often the term q_i appears in document D,
//! - avgdl is the average document length in the list of documents,
//! - and IDF(q_i) is the inverse document frequency, defined as
//!
//! IDF(q_i) = ln((N - n(q_i) + 0.5) / (n(q_i) + 0.5) + 1),
//!
//! where
//!
//! - N is the total number of notes in the vault, and
//! - n(q_i) is the total number of documents containing q_i
//!
//! k_1 and b are optimisation parameters, with the usual values being k_1 in [1.2, 2.0] and b =
//! 0.75.
//!
//! References:
//!
//! - https://en.wikipedia.org/wiki/Okapi_BM25
//! - https://emschwartz.me/understanding-the-bm25-full-text-search-algorithm/
use std::collections::{HashMap, HashSet};

use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};
use rust_stemmers::{Algorithm, Stemmer};
use serde::Serialize;

#[derive(Serialize, Debug)]
/// The precomputed statistics on the vault
///
/// * `docs`: the stripped-down contents of the documents in the  vault
/// * `avgdl`: the average length of the documents in the vault
/// * `idf`: the inverse document frequency
pub struct Corpus {
    docs: Vec<String>,
    avgdl: f32,
    idf: HashMap<String, f32>,
    df: HashMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct BM25Score(f32);
impl From<BM25Score> for f32 {
    fn from(value: BM25Score) -> Self {
        let BM25Score(val) = value;
        val
    }
}

impl Corpus {
    /// Because I don't know what's going on here, I'll just randomly choose k_1 as 1.6.
    pub const K1: f32 = 1.6;
    pub const B: f32 = 0.75;

    /// Initilise a new corpus and calculate its statistics
    // NOTE: Figure out if we can guarantee that this document is definitely found in the corpus
    pub fn new(docs: Vec<String>) -> Self {
        let stemmer = Stemmer::create(Algorithm::English);
        // Find the average length of a document in the corpus
        let avgdl = docs
            .iter()
            .map(|doc| doc.split_whitespace().count() as f32)
            .sum::<f32>()
            / docs.len() as f32;

        // Calculate the document frequency
        let mut df: HashMap<String, u32> = HashMap::new();
        for doc in &docs {
            let unique_terms: HashSet<String> = doc
                .split_whitespace()
                .par_bridge()
                .map(|tok| stemmer.stem(tok).to_string())
                .collect();

            for term in unique_terms {
                *df.entry(term).or_default() += 1;
            }
        }

        // Calculate the inverse document frequency of each token from the document frequency
        let idf = df
            .par_iter()
            .map(|(term, num_occurrence)| {
                let num_docs = docs.len() as f32;
                let idf: f32 = {
                    let num_occurrence: f32 = *num_occurrence as f32;
                    let res: f32 = (num_docs - num_occurrence + 0.5) / (num_occurrence + 0.5) + 1.0;
                    res.ln()
                };
                (term.clone(), idf)
            })
            .collect();
        Self {
            docs,
            avgdl,
            idf,
            df,
        }
    }

    /// Calculate the BM25 score of a `document` given the `query`
    pub fn score(&self, query: &str, document: &str) -> BM25Score {
        let document_length = document.split_whitespace().count() as f32;
        let norm = Self::K1 * (1f32 - Self::B + Self::B * document_length / self.avgdl);

        let stemmer = Stemmer::create(Algorithm::English);
        // Find out how many times each term shows up in the given document
        let tf: HashMap<String, usize> = document
            .split_whitespace()
            .map(|term| stemmer.stem(term).to_string())
            .fold(
                HashMap::new(),
                |mut frequencies: HashMap<String, usize>, term| {
                    *frequencies.entry(term).or_default() += 1;
                    frequencies
                },
            );

        // Calculate the BM25 score of each term in the query
        let res = query
            .split_whitespace()
            .map(|term| {
                let term = stemmer.stem(term).to_string();
                let frequency = *tf.get(term.as_str()).unwrap_or(&0) as f32;
                let idf = *self.idf.get(term.as_str()).unwrap_or(&0f32);
                idf * ((frequency * (Self::K1 + 1f32)) / (frequency + norm))
            })
            .sum();
        BM25Score(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use proptest::prelude::*;
    use rand::rng;
    use rand::seq::{IndexedRandom, SliceRandom};

    /// ASCII lowercase word, length 1‒10
    fn word() -> impl Strategy<Value = String> {
        proptest::collection::vec(proptest::char::range('a', 'z'), 1..=10)
            .prop_map(|chars| chars.into_iter().collect())
    }

    /// Document = 1‒`max_len` words
    fn document(max_len: usize) -> impl Strategy<Value = String> {
        proptest::collection::vec(word(), 1..=max_len).prop_map(|words| words.join(" "))
    }

    /// Non-empty corpus of up to 20 docs (varied lengths)
    fn corpus() -> impl Strategy<Value = Vec<String>> {
        proptest::collection::vec(document(25), 1..=20)
    }
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100_000))]
        #[test]
        fn score_is_never_negative(
            docs in corpus(),
            query in document(10)
        ) {
            let c = Corpus::new(docs.clone());
            for d in &docs {
                let score: f32 = c.score(&query, d).into();
                prop_assert!(score >= 0.0);
            }
        }

        #[test]
        fn empty_query_gives_zero(
            docs in corpus(),
            doc_idx in 0usize..
        ) {
            let c = Corpus::new(docs.clone());
            let d = &docs[doc_idx % docs.len()];
            let score: f32 = c.score("", d).into();
            prop_assert_eq!(score, 0.0);
        }

        #[test]
        fn more_occurences_raise_score(
            base in proptest::collection::vec(word(), 1..=5),
            extra in word()
        ) {
            let mut short = base.clone(); short.push(extra.clone());
            let mut long  = short.clone(); long.push(extra.clone());

            let docs = vec![short.join(" "), long.join(" ")];
            let corpus = Corpus::new(docs.clone());

            let qs = extra.as_str();
            let s1 = corpus.score(qs, &docs[0]);
            let s2 = corpus.score(qs, &docs[1]);
            prop_assert!(s2 > s1);
        }

        #[test]
        fn longer_doc_penalised(
            term in word(),
            filler in proptest::collection::vec(word(), 5..=15)
        ) {
            let short = term.clone();
            let long = format!("{} {}", short, filler.join(" "));
            let corpus = Corpus::new(vec![short.clone(), long.clone()]);

            let s_short = corpus.score(&term, &short);
            let s_long  = corpus.score(&term, &long);
            prop_assert!(s_short > s_long);
        }

        #[test]
        fn query_permutation_yields_same_score(
            docs in corpus(),
            q_words in proptest::collection::vec(word(), 1..=5)
        ) {
            let corpus = Corpus::new(docs.clone());

            let q1 = q_words.join(" ");
            let mut shuffled = q_words.clone();
            shuffled.shuffle(&mut rng());
            let q2 = shuffled.join(" ");

            for d in &docs {
                let s1: f32 = corpus.score(&q1, d).into();
                let s2: f32 = corpus.score(&q2, d).into();
                assert_relative_eq!(s1, s2, epsilon = 1e-6_f32);
            }
        }

        #[test]
        fn missing_terms_give_zero(
            (docs, missing_term) in corpus().prop_flat_map(|docs| {
                let stemmer = Stemmer::create(Algorithm::English);
                let existing_stems: HashSet<String> = docs
                    .iter()
                    .flat_map(|d| d.split_whitespace())
                    .map(|t| stemmer.stem(t).to_string())
                    .collect();

                (
                    Just(docs),
                    word().prop_filter(
                        "candidate term must not be in the corpus",
                        move |candidate| !existing_stems.contains(&stemmer.stem(candidate).to_string())
                    )
                )
            })
        ) {
            let c = Corpus::new(docs.clone());
            for d in &docs {
                let score: f32 = c.score(&missing_term, d).into();
                prop_assert_eq!(score, 0.0);
            }
        }

        #[test]
        fn rarer_term_has_higher_idf(
            docs in corpus()
        ) {
            let corpus = Corpus::new(docs);
            let terms: Vec<String> = corpus.idf.keys().cloned().collect();

            prop_assume!(terms.len() >= 2, "Need at least two terms to compare");

            for i in 0..terms.len() {
                for j in (i + 1)..terms.len() {
                    let t1 = &terms[i];
                    let t2 = &terms[j];

                    let df1 = corpus.df[t1];
                    let df2 = corpus.df[t2];

                    let idf1 = corpus.idf[t1];
                    let idf2 = corpus.idf[t2];

                    if df1 < df2 {
                        prop_assert!(
                            idf1 > idf2,
                            "Term '{}' (df={}) is rarer than '{}' (df={}), so its IDF ({}) should be > ({})",
                            t1, df1, t2, df2, idf1, idf2
                        );
                    } else if df2 < df1 {
                        prop_assert!(
                            idf2 > idf1,
                            "Term '{}' (df={}) is rarer than '{}' (df={}), so its IDF ({}) should be > ({})",
                            t2, df2, t1, df1, idf2, idf1
                        );
                    } else {
                        let tolerance = 1e-9;
                        prop_assert!(
                            (idf1 - idf2).abs() < tolerance,
                            "Terms '{}' and '{}' have the same DF ({}), so their IDFs ({}, {}) should be equal",
                            t1, t2, df1, idf1, idf2
                        );
                    }
                }
            }
        }
    }
}
