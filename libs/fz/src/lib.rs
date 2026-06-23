use memchr::memchr;

const GAP_SCORE: i32 = -3;
const BONUS_WORD_BOUNDARY: i32 = 15;
const BONUS_CAMEL: i32 = 10;
const BONUS_CONSECUTIVE: i32 = 20;

pub fn fzrank(pattern: &str, candidates: &[String]) -> Vec<(usize, i32)> {
    let pbytes = pattern.to_lowercase().into_bytes();
    if pbytes.is_empty() {
        return candidates.iter().enumerate().map(|(i, s)| (i, s.len() as i32 * 2 + BONUS_CONSECUTIVE)).collect();
    }

    candidates
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let text_lc = text.to_lowercase();
            let text_bytes = text_lc.into_bytes();
            let score = score_text(&pbytes, &text_bytes);
            (i, score)
        })
        .collect()
}

pub fn score_text(pattern: &[u8], text: &[u8]) -> i32 {
    let m = pattern.len();
    let n = text.len();
    if m == 0 || n == 0 {
        return 0;
    }

    let bof = word_boundaries(text);

    let mut matches: Vec<Vec<usize>> = Vec::with_capacity(m);
    for &pch in pattern {
        let mut hit: Vec<usize> = Vec::new();
        let mut pos = 0;
        while let Some(p) = memchr(pch, &text[pos..]) {
            hit.push(pos + p);
            pos += p + 1;
        }
        if hit.is_empty() {
            return 0;
        }
        matches.push(hit);
    }

    let mut dp: Vec<i32> = vec![i32::MIN; n];
    for &pos in &matches[0] {
        let bonus = if pos == 0 {
            20
        } else if bof.contains(&pos) {
            BONUS_WORD_BOUNDARY
        } else {
            0
        };
        dp[pos] = bonus;
    }

    for i in 1..m {
        let hit = &matches[i];
        let mut next = vec![i32::MIN; n];

        for &pos in hit {
            let base_bonus = if bof.contains(&pos) {
                BONUS_WORD_BOUNDARY
            } else if pos > 0 && text[pos].is_ascii_lowercase() && text[pos - 1].is_ascii_uppercase() {
                BONUS_CAMEL
            } else {
                0
            };

            for prev in 0..pos {
                let base = dp[prev];
                if base == i32::MIN {
                    continue;
                }
                let gap = pos - prev;
                let gap_penalty = if gap <= 1 { 0 } else { GAP_SCORE * gap as i32 };
                let consecutive = if gap == 1 { BONUS_CONSECUTIVE } else { 0 };
                let sc = base + gap_penalty + consecutive + base_bonus;
                if sc > next[pos] {
                    next[pos] = sc;
                }
            }
        }

        dp = next;
    }

    let mut quality = 0i32;
    let mut last = n;

    for hit in &matches {
        for &pos in hit.iter().rev() {
            if pos < last {
                let gap = last.saturating_sub(pos + 1);
                let gap_adj = if gap == 0 { 0 } else { (-3 * gap as i32).saturating_sub(1) };
                let leading_bonus = if pos == 0 { 20 } else { 0 };
                quality += 1
                    + gap_adj
                    + bof.contains(&pos) as i32 * BONUS_WORD_BOUNDARY
                    + if pos > 0 && text[pos].is_ascii_lowercase() && text[pos - 1].is_ascii_uppercase() {
                        BONUS_CAMEL
                    } else {
                        0
                    }
                    + leading_bonus;
                last = pos;
                break;
            }
        }
    }

    let complexity: i32 = if m == 1 { 1 } else { m as i32 * (m as i32 - 1) / 2 };

    if complexity == 0 {
        return 0;
    }

    let density = (quality * 100) / complexity;
    let sparse_penalty = if n > m * 2 { 100 * m as i32 * 200 / (n as i32 + 100) } else { 100 };

    (density * sparse_penalty) / 100
}

fn word_boundaries(text: &[u8]) -> Vec<usize> {
    let mut bof = Vec::new();
    for (i, &b) in text.iter().enumerate() {
        if i == 0 || is_word_boundary(b, text[i - 1]) {
            bof.push(i);
        }
    }
    bof
}

#[inline]
fn is_word_boundary(curr: u8, prev: u8) -> bool {
    match (prev.is_ascii_alphanumeric(), curr.is_ascii_alphanumeric()) {
        (false, true) => true,
        (true, false) => true,
        (true, true) => curr.is_ascii_uppercase() && prev.is_ascii_lowercase(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> String { text.into() }

    fn scores(query: &str, cands: &[String]) -> Vec<i32> {
        let mut r = fzrank(query, cands);
        r.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        r.into_iter().map(|(_, s)| s).collect()
    }

    #[test]
    fn no_match() {
        assert_eq!(scores("xyz", &[s("foobar")])[0], 0);
    }

    #[test]
    fn empty_query() {
        assert!(scores("", &[s("abc")])[0] > 0);
    }

    #[test]
    fn leading_char_better() {
        let sc = scores("h", &[s("hello"), s("goodbye hello")]);
        assert!(sc[0] > sc[1]);
    }

    #[test]
    fn consecutive_char_better() {
        let sc = scores("hl", &[s("hello"), s("hxxl")]);
        assert!(sc[0] > sc[1]);
    }

    #[test]
    fn word_boundary_better() {
        let sc = scores("ab", &[s("hello world"), s("foo bar bar")]);
        assert!(sc[1] > sc[0]);
    }

    #[test]
    fn subsequence_in_longer_candidate() {
        let sc = scores("hl", &[s("he"), s("hexxxhl")]);
        assert!(sc[0] >= sc[1]);
    }
}
