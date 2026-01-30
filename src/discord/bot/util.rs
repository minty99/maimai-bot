pub(crate) fn normalize_for_match(s: &str) -> String {
    s.to_ascii_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
}

pub(crate) fn top_title_matches(search: &str, titles: &[String], limit: usize) -> Vec<String> {
    let search_norm = normalize_for_match(search.trim());
    let mut scored = titles
        .iter()
        .map(|t| (t, levenshtein(&search_norm, &normalize_for_match(t))))
        .collect::<Vec<_>>();
    scored.sort_by_key(|(_, d)| *d);
    scored
        .into_iter()
        .take(limit.max(1))
        .map(|(t, _)| t.clone())
        .collect()
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }

    let mut prev = (0..=b.len()).collect::<Vec<usize>>();
    let mut cur = vec![0usize; b.len() + 1];

    for (i, &ac) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, &bc) in b.iter().enumerate() {
            let cost = usize::from(ac != bc);
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }

    prev[b.len()]
}

pub(crate) fn latest_credit_len(tracks: &[Option<i64>]) -> usize {
    match tracks.iter().position(|t| *t == Some(1)) {
        Some(idx) => idx + 1,
        None => tracks.len().min(4),
    }
}
