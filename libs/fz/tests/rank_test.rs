#[test]
fn billy_jean_ranking() {
    let cands: Vec<String> = [
        "Billie Jean",
        "Billy Jean",
        "Billy Talent",              // billy only - must be excluded
        "Jean Michel Jarre",         // jean only - must be excluded
        "Билли Джин",                // cyrillic - excluded (bytes)
        "ビリー・ジーン",             // katakana - excluded
        "Billie Jean (Single Version)",
        "Jeanny",                     // fuzzy-ish, should fail token AND
    ].iter().map(|s| s.to_string()).collect();
    let r = fz::fzrank("billy jean", &cands);
    println!("results: {:?}", r.iter().map(|(i,s)| (cands[*i].as_str(), *s)).collect::<Vec<_>>());
    assert!(!r.is_empty());
    // Top must be one of the Billie/Billy Jean variants
    assert!(r[0].0 == 0 || r[0].0 == 1 || r[0].0 == 6, "top was {}", cands[r[0].0]);
    // No single-token candidates present
    for (i, _) in &r { assert!(*i != 2 && *i != 3 && *i != 7, "leaked {}", cands[*i]); }
}
