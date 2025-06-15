use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::{document::Document, path::MarkdownPath};
use std::{collections::HashMap, path::PathBuf};

/// Rank the vault using the PageRank algoritm, where the ranking of a page `A` is given by
///
/// PR(A) = (1 - d) + d * (PR(T_1)/C(T_1) + ... + PR(T_n) / C(T_n)),
///
/// where
///
/// - `d` is the dampening factor,
/// - `T_1` to `T_n` are pages with links to `A`, and
/// - C(A) is the number of links going out of `A`.
///
/// Since this can cause quite a bit of function calls and result in a stack overflow when done
/// recursively, we imlpement it iteratively
///
/// References:
///
/// - https://research.google/pubs/the-anatomy-of-a-large-scale-hypertextual-web-search-engine/
/// - https://cs.brown.edu/courses/cs016/static/files/assignments/projects/GraphHelpSession.pdf
/// - https://web.stanford.edu/class/cs315b/assignment3.html
/// - https://pi.math.cornell.edu/~mec/Winter2009/RalucaRemus/Lecture3/lecture3.html
pub fn rank(docs: Vec<&Document>, base_path: PathBuf, num_iter: usize, tol: f32) -> Vec<f32> {
    /// The dampening factor of PageRank. This reflects the probability that the user exit the
    /// current document and 'teleport' to a new one.
    pub const D: f32 = 0.85;

    let num_docs = docs.len();

    // "Teleport" refers to the ability for a user to switch to a different document without
    // following a link.
    let teleport = (1.0 - D) / num_docs as f32;

    let idx: HashMap<MarkdownPath, usize> = docs
        .iter()
        .enumerate()
        .map(|(i, d)| (d.path(), i))
        .collect();

    // The list of vertices pointing into each node.
    let mut inbound: Vec<Vec<usize>> = vec![Vec::new(); num_docs];

    // The number of vertices pointing out of each node
    let mut outdeg: Vec<usize> = vec![0; num_docs];

    // Iterate through each document...
    for (src, doc) in docs.iter().enumerate() {
        // ...and go through their links...
        for link in doc.links() {
            if let Some(target) = link.to_markdown_path(base_path.clone())
                && let Some(&dst) = idx.get(&target)
            {
                // ...to find which other documents they point to, and populate the `inbound`
                // and `outdeg` vectors accordingly.
                inbound[dst].push(src);
                outdeg[src] += 1;
            }
        }
    }

    // The PageRank score of each vertex. This always sums up to one (give and take some
    // tolerance level to account for the weirdness of floating-point arithmetic).
    let mut rank = vec![1.0 / num_docs as f32; num_docs];

    for _ in 0..num_iter {
        // How many documents do not point to other documents (have no links).
        let dangling_mass: f32 = rank
            .iter()
            .enumerate()
            .filter(|(i, _)| outdeg[*i] == 0)
            .map(|(_, r)| *r)
            .sum();

        // The rank of a document if it does not have any documents referencing it.
        let base = teleport + D * dangling_mass / num_docs as f32;
        let mut next = vec![base; num_docs];

        next.par_iter_mut().enumerate().for_each(|(dst, val)| {
            // Calculate the rank / out degree of each documents referencing this one.
            let contrib: f32 = inbound[dst]
                .iter()
                .map(|&src| rank[src] / outdeg[src] as f32)
                .sum();
            *val += D * contrib;
        });

        // The sum of the differences between two consecutive ranks
        let delta: f32 = rank.iter().zip(&next).map(|(a, b)| (a - b).abs()).sum();

        rank = next;

        if delta < tol {
            break;
        }
    }
    rank
}
