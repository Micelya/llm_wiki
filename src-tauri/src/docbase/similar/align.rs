//! Two texts side by side: which stretches they share and which differ.
//!
//! A patience alignment, written here instead of adding a crate: words
//! that appear exactly once in each text anchor the alignment (the
//! longest run of them in the same order), and what lies between two
//! anchors is aligned again the same way. Only short stretches without
//! anchors fall to the classic longest-common-subsequence table, and a
//! stretch too big for it is taken as changed whole. Long legal texts
//! have plenty of unique words, so this is fast and stays faithful.

use std::collections::HashMap;
use std::ops::Range;

/// Cells of the longest-common-subsequence table above which a stretch
/// without anchors is taken as changed whole.
const MAX_TABLE_CELLS: usize = 4_000_000;

/// A stretch of the alignment: the same words in both, or a change
/// (either side may be empty: a deletion or an insertion).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Same { a: Range<usize>, b: Range<usize> },
    Change { a: Range<usize>, b: Range<usize> },
}

/// Aligns two sequences of word ids.
pub fn align(a: &[u32], b: &[u32]) -> Vec<Op> {
    let mut pairs = Vec::new();
    matches(a, b, 0..a.len(), 0..b.len(), &mut pairs);
    to_ops(&pairs, a.len(), b.len())
}

/// Turns words into ids shared by both texts, so they compare as numbers.
pub fn intern<'a>(a: impl Iterator<Item = &'a str>, b: impl Iterator<Item = &'a str>) -> (Vec<u32>, Vec<u32>) {
    let mut ids: HashMap<&str, u32> = HashMap::new();
    let mut id_of = |word: &'a str| {
        let next = ids.len() as u32;
        *ids.entry(word).or_insert(next)
    };
    let a: Vec<u32> = a.map(&mut id_of).collect();
    let b: Vec<u32> = b.map(&mut id_of).collect();
    (a, b)
}

/// Appends to `out` the matched positions (i in a, j in b), in order.
fn matches(a: &[u32], b: &[u32], ra: Range<usize>, rb: Range<usize>, out: &mut Vec<(usize, usize)>) {
    let (mut a0, mut a1, mut b0, mut b1) = (ra.start, ra.end, rb.start, rb.end);
    while a0 < a1 && b0 < b1 && a[a0] == b[b0] {
        out.push((a0, b0));
        a0 += 1;
        b0 += 1;
    }
    let mut tail = Vec::new();
    while a0 < a1 && b0 < b1 && a[a1 - 1] == b[b1 - 1] {
        a1 -= 1;
        b1 -= 1;
        tail.push((a1, b1));
    }
    if a0 < a1 && b0 < b1 {
        let anchors = unique_anchors(a, b, a0..a1, b0..b1);
        if anchors.is_empty() {
            table(a, b, a0..a1, b0..b1, out);
        } else {
            let (mut pa, mut pb) = (a0, b0);
            for (i, j) in anchors {
                matches(a, b, pa..i, pb..j, out);
                out.push((i, j));
                pa = i + 1;
                pb = j + 1;
            }
            matches(a, b, pa..a1, pb..b1, out);
        }
    }
    out.extend(tail.into_iter().rev());
}

/// Words that appear exactly once in each stretch, keeping the longest
/// run of them that is in the same order in both.
fn unique_anchors(a: &[u32], b: &[u32], ra: Range<usize>, rb: Range<usize>) -> Vec<(usize, usize)> {
    // word -> (times in a, position in a, times in b, position in b)
    let mut seen: HashMap<u32, (u32, usize, u32, usize)> = HashMap::new();
    for i in ra.clone() {
        let entry = seen.entry(a[i]).or_insert((0, i, 0, 0));
        entry.0 += 1;
        entry.1 = i;
    }
    for j in rb {
        if let Some(entry) = seen.get_mut(&b[j]) {
            entry.2 += 1;
            entry.3 = j;
        }
    }
    let mut candidates: Vec<(usize, usize)> = seen
        .values()
        .filter(|(in_a, _, in_b, _)| *in_a == 1 && *in_b == 1)
        .map(|(_, i, _, j)| (*i, *j))
        .collect();
    candidates.sort_unstable();
    longest_increasing(&candidates)
}

/// The longest subsequence of `pairs` (sorted by i) whose j increases.
fn longest_increasing(pairs: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut tails: Vec<usize> = Vec::new(); // index into pairs of the smallest tail of each length
    let mut previous: Vec<Option<usize>> = vec![None; pairs.len()];
    for (index, &(_, j)) in pairs.iter().enumerate() {
        let length = tails.partition_point(|&t| pairs[t].1 < j);
        if length > 0 {
            previous[index] = Some(tails[length - 1]);
        }
        if length == tails.len() {
            tails.push(index);
        } else {
            tails[length] = index;
        }
    }
    let mut out = Vec::with_capacity(tails.len());
    let mut current = tails.last().copied();
    while let Some(index) = current {
        out.push(pairs[index]);
        current = previous[index];
    }
    out.reverse();
    out
}

/// The classic table, for a stretch with no anchors.
fn table(a: &[u32], b: &[u32], ra: Range<usize>, rb: Range<usize>, out: &mut Vec<(usize, usize)>) {
    let (n, m) = (ra.len(), rb.len());
    if n.saturating_mul(m) > MAX_TABLE_CELLS {
        return;
    }
    let width = m + 1;
    // lengths[i][j]: common subsequence of a[i..] and b[j..].
    let mut lengths = vec![0u32; (n + 1) * width];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lengths[i * width + j] = if a[ra.start + i] == b[rb.start + j] {
                lengths[(i + 1) * width + j + 1] + 1
            } else {
                lengths[(i + 1) * width + j].max(lengths[i * width + j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if a[ra.start + i] == b[rb.start + j] {
            out.push((ra.start + i, rb.start + j));
            i += 1;
            j += 1;
        } else if lengths[(i + 1) * width + j] >= lengths[i * width + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
}

fn to_ops(pairs: &[(usize, usize)], len_a: usize, len_b: usize) -> Vec<Op> {
    let mut ops = Vec::new();
    let (mut i, mut j) = (0, 0);
    let mut index = 0;
    while index < pairs.len() {
        let (pi, pj) = pairs[index];
        if pi > i || pj > j {
            ops.push(Op::Change { a: i..pi, b: j..pj });
        }
        let start = index;
        while index + 1 < pairs.len() && pairs[index + 1] == (pairs[index].0 + 1, pairs[index].1 + 1) {
            index += 1;
        }
        let (ei, ej) = pairs[index];
        ops.push(Op::Same { a: pairs[start].0..ei + 1, b: pairs[start].1..ej + 1 });
        i = ei + 1;
        j = ej + 1;
        index += 1;
    }
    if i < len_a || j < len_b {
        ops.push(Op::Change { a: i..len_a, b: j..len_b });
    }
    ops
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(a: &str, b: &str) -> (Vec<Op>, usize) {
        let (ia, ib) = intern(a.split_whitespace(), b.split_whitespace());
        let ops = align(&ia, &ib);
        let same = ops
            .iter()
            .map(|op| match op {
                Op::Same { a, .. } => a.len(),
                Op::Change { .. } => 0,
            })
            .sum();
        (ops, same)
    }

    #[test]
    fn a_changed_word_is_one_change_between_two_equal_stretches() {
        let (ops, same) = run("el plazo es de treinta dias corridos", "el plazo es de sesenta dias corridos");
        assert_eq!(same, 6);
        assert_eq!(
            ops,
            vec![
                Op::Same { a: 0..4, b: 0..4 },
                Op::Change { a: 4..5, b: 4..5 },
                Op::Same { a: 5..7, b: 5..7 },
            ]
        );
    }

    #[test]
    fn insertions_deletions_and_repeated_words() {
        let (ops, same) = run("a b a b c d", "a b x a b c");
        assert_eq!(same, 5);
        assert!(ops.contains(&Op::Change { a: 2..2, b: 2..3 }));
        assert!(ops.contains(&Op::Change { a: 5..6, b: 6..6 }));
        let (_, same) = run("uno dos tres", "cuatro cinco seis");
        assert_eq!(same, 0);
        let (ops, _) = run("", "uno dos");
        assert_eq!(ops, vec![Op::Change { a: 0..0, b: 0..2 }]);
    }

    #[test]
    fn moved_blocks_keep_the_longest_ordered_part() {
        let a = "w1 w2 w3 w4 w5 w6 w7 w8 w9 w10";
        let b = "w6 w7 w1 w2 w3 w4 w5 w8 w9 w10";
        let (_, same) = run(a, b);
        assert_eq!(same, 8);
    }
}
