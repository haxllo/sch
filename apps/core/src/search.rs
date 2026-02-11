use crate::model::{normalize_for_search, SearchItem};

pub fn search(items: &[SearchItem], query: &str, limit: usize) -> Vec<SearchItem> {
    if limit == 0 || items.is_empty() {
        return Vec::new();
    }

    let normalized_query = normalize_for_search(query);
    if normalized_query.is_empty() {
        return Vec::new();
    }

    let mut scored: Vec<(i64, usize, &SearchItem)> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            score_item(item, &normalized_query).map(|score| (score, index, item))
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

    scored
        .into_iter()
        .take(limit)
        .map(|(_, _, item)| item.clone())
        .collect()
}

fn score_item(item: &SearchItem, normalized_query: &str) -> Option<i64> {
    let text_score = score_normalized_title(item.normalized_title(), normalized_query)?;
    let recency_bonus = recency_bonus(item.last_accessed_epoch_secs);
    let frequency_bonus = frequency_bonus(item.use_count);

    Some(text_score + recency_bonus + frequency_bonus)
}

fn recency_bonus(last_accessed_epoch_secs: i64) -> i64 {
    if last_accessed_epoch_secs <= 0 {
        return 0;
    }

    (last_accessed_epoch_secs / 86_400).clamp(0, 400)
}

fn frequency_bonus(use_count: u32) -> i64 {
    ((use_count as i64) * 12).clamp(0, 400)
}

fn score_normalized_title(normalized_title: &str, query: &str) -> Option<i64> {
    if normalized_title.is_empty() || query.is_empty() {
        return None;
    }

    if let Some(position) = normalized_title.find(query) {
        let prefix_bonus = if position == 0 { 400 } else { 0 };
        let compact_bonus = (query.len() as i64) * 40;
        let position_penalty = position as i64;
        let length_penalty = (normalized_title.len() as i64 - query.len() as i64).abs();
        return Some(10_000 + prefix_bonus + compact_bonus - position_penalty - length_penalty);
    }

    let positions = subsequence_positions(normalized_title, query)?;
    let start_penalty = positions[0] as i64;
    let gap_penalty: i64 = positions
        .windows(2)
        .map(|pair| pair[1].saturating_sub(pair[0] + 1) as i64)
        .sum();
    let length_penalty = (normalized_title.len() as i64 - query.len() as i64).max(0);

    Some(5_000 + (query.len() as i64) * 30 - gap_penalty * 6 - start_penalty - length_penalty)
}

fn subsequence_positions(haystack: &str, needle: &str) -> Option<Vec<usize>> {
    let mut positions = Vec::with_capacity(needle.len());
    let mut next_start = 0;

    for needle_char in needle.chars() {
        let mut found = None;
        for (offset, hay_char) in haystack[next_start..].char_indices() {
            if hay_char == needle_char {
                let absolute = next_start + offset;
                found = Some(absolute);
                next_start = absolute + hay_char.len_utf8();
                break;
            }
        }

        let position = found?;
        positions.push(position);
    }

    Some(positions)
}
