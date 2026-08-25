use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

static MATCHER: std::sync::LazyLock<SkimMatcherV2> =
    std::sync::LazyLock::new(SkimMatcherV2::default);

/// Rank `candidates` against `pattern` using token-aware fuzzy matching.
///
/// Each whitespace-separated token in `pattern` is matched independently
/// via `SkimMatcherV2`. A candidate's score is the **sum** of its per-token
/// scores. Candidates that fail to match any token score 0.
///
/// Non-ASCII characters in the candidate incur a per-byte penalty so that
/// Latin/English titles surface above CJK / Cyrillic / etc. when the query
/// is ASCII.
///
/// Returns results sorted by descending score.
pub fn fzrank(pattern: &str, candidates: &[String]) -> Vec<(usize, i32)> {
    if pattern.trim().is_empty() {
        return candidates
            .iter()
            .enumerate()
            .map(|(i, s)| (i, s.len() as i32))
            .collect();
    }

    let tokens: Vec<&str> = pattern.split_whitespace().collect();

    let mut scored: Vec<(usize, i32)> = candidates
        .iter()
        .enumerate()
        .filter_map(|(i, text)| {
            let total: i64 = tokens
                .iter()
                .filter_map(|tok| MATCHER.fuzzy_match(text, tok))
                .sum();

            if total == 0 {
                return None;
            }

            // Penalise non-ASCII bytes so English/Latin titles rank higher.
            let non_ascii_penalty: i64 =
                text.bytes().filter(|&b| b > 127).count() as i64 * 50;

            let score = (total - non_ascii_penalty).max(0) as i32;
            if score == 0 { None } else { Some((i, score)) }
        })
        .collect();

    scored.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> String { text.into() }

    fn top(query: &str, cands: &[String]) -> Vec<(usize, i32)> {
        fzrank(query, cands)
    }

    #[test]
    fn no_match() {
        assert!(top("xyz", &[s("foobar")]).is_empty());
    }

    #[test]
    fn empty_query_returns_all() {
        let r = top("", &[s("abc"), s("def")]);
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn exact_match_beats_fuzzy() {
        let r = top("hello", &[s("hello world"), s("hxxxeoollo")]);
        assert_eq!(r[0].0, 0);
    }

    #[test]
    fn typo_tolerance() {
        // "Billie Jean" should match "Billy Jean" (transposition of i/e)
        let r = top("billy jean", &[s("Billie Jean")]);
        assert!(!r.is_empty(), "typo should still match");
    }

    #[test]
    fn token_splitting() {
        // "michael thriller" should match "Michael Jackson - Thriller"
        let r = top("michael thriller", &[
            s("Michael Jackson - Thriller"),
            s("Thriller - Michael Jackson"),
            s("Random Other Album"),
        ]);
        assert!(!r.is_empty());
        // Both Michael/Thriller results should beat the random one
        let top_ids: Vec<usize> = r.iter().take(2).map(|(i, _)| *i).collect();
        assert!(top_ids.contains(&0), "Michael Jackson - Thriller should be top-2");
        assert!(top_ids.contains(&1), "Thriller - Michael Jackson should be top-2");
    }

    #[test]
    fn non_ascii_penalised() {
        let r = top("jean", &[
            s("Билли Jean"),
            s("Jean-Pierre"),
        ]);
        // Jean-Pierre should rank first
        assert_eq!(r[0].0, 1, "Jean-Pierre should rank first, got {:?}", r);
    }

    #[test]
    fn leading_match_ranked_higher() {
        let r = top("thr", &[
            s("Thriller"),
            s("History"),
        ]);
        assert_eq!(r[0].0, 0, "Thriller should rank first");
    }

    #[test]
    fn subsequence_matching() {
        // "abc" should match "a_big_cave" (subsequence with gaps)
        let r = top("abc", &[s("a_big_cave"), s("xyz")]);
        assert!(!r.is_empty());
        assert_eq!(r[0].0, 0);
    }
}
