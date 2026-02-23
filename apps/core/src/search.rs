use crate::config::SearchMode;
use crate::model::{normalize_for_search, SearchItem};
use crate::query_dsl::TimeFilterWindow;
use std::cmp::Ordering;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const SCORE_EXACT: i64 = 30_000;
const SCORE_PREFIX: i64 = 24_000;
const SCORE_SUBSTRING: i64 = 18_000;
const SCORE_FUZZY: i64 = 12_000;

const SOURCE_APP_BONUS: i64 = 700;
const SOURCE_LOCAL_FS_BONUS: i64 = 420;
const SOURCE_ACTION_BONUS: i64 = 350;
const SOURCE_CLIPBOARD_BONUS: i64 = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchFilter {
    pub mode: SearchMode,
    pub kind_filter: Option<String>,
    pub include_groups: Vec<Vec<String>>,
    pub exclude_terms: Vec<String>,
    pub modified_within: Option<TimeFilterWindow>,
    pub created_within: Option<TimeFilterWindow>,
}

impl Default for SearchFilter {
    fn default() -> Self {
        Self {
            mode: SearchMode::All,
            kind_filter: None,
            include_groups: Vec::new(),
            exclude_terms: Vec::new(),
            modified_within: None,
            created_within: None,
        }
    }
}

pub fn search(items: &[SearchItem], query: &str, limit: usize) -> Vec<SearchItem> {
    if normalize_for_search(query).is_empty() {
        return Vec::new();
    }
    search_with_filter(items, query, limit, &SearchFilter::default())
}

pub fn search_with_filter(
    items: &[SearchItem],
    query: &str,
    limit: usize,
    filter: &SearchFilter,
) -> Vec<SearchItem> {
    if limit == 0 || items.is_empty() {
        return Vec::new();
    }

    let normalized_query = normalize_for_search(query);
    let fast_path = is_default_filter(filter) && !normalized_query.is_empty();
    let now_epoch_secs = now_epoch_secs();
    let mut scored: Vec<ScoredItem<'_>> = items
        .iter()
        .filter_map(|item| {
            let score = if fast_path {
                score_item_fast(item, &normalized_query, now_epoch_secs)
            } else {
                score_item(item, &normalized_query, now_epoch_secs, filter)
            };
            score.map(|score| ScoredItem {
                source_rank: source_rank(item),
                score,
                title_len: item.normalized_title().len(),
                item,
            })
        })
        .collect();

    if scored.len() > limit {
        scored.select_nth_unstable_by(limit, compare_scored);
        scored.truncate(limit);
    }
    scored.sort_unstable_by(compare_scored);

    scored
        .into_iter()
        .take(limit)
        .map(|scored| scored.item.clone())
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct ScoredItem<'a> {
    source_rank: u8,
    score: i64,
    title_len: usize,
    item: &'a SearchItem,
}

fn compare_scored(a: &ScoredItem<'_>, b: &ScoredItem<'_>) -> Ordering {
    b.score
        .cmp(&a.score)
        .then_with(|| a.source_rank.cmp(&b.source_rank))
        .then_with(|| a.title_len.cmp(&b.title_len))
        .then_with(|| a.item.normalized_title().cmp(b.item.normalized_title()))
        .then_with(|| a.item.id.cmp(&b.item.id))
}

fn score_item_fast(item: &SearchItem, normalized_query: &str, now_epoch_secs: i64) -> Option<i64> {
    let text_score = score_text(item.normalized_title(), normalized_query)?;
    let source_bonus = source_bonus(item);
    let recency_bonus = recency_bonus(item.last_accessed_epoch_secs, now_epoch_secs);
    let frequency_bonus = frequency_bonus(item.use_count);

    Some(text_score + source_bonus + recency_bonus + frequency_bonus)
}

fn score_item(
    item: &SearchItem,
    normalized_query: &str,
    now_epoch_secs: i64,
    filter: &SearchFilter,
) -> Option<i64> {
    if !matches_mode(item, filter.mode) {
        return None;
    }
    if let Some(kind) = &filter.kind_filter {
        if !matches_kind_filter(item, kind) {
            return None;
        }
    }
    if !matches_term_filters(item, filter) {
        return None;
    }
    if !matches_time_filters(item, filter, now_epoch_secs) {
        return None;
    }

    let text_score = if normalized_query.is_empty() {
        0
    } else {
        score_text(item.normalized_title(), normalized_query).or_else(|| {
            score_text(item.normalized_search_text(), normalized_query).map(|s| s - 1_500)
        })?
    };
    let source_bonus = source_bonus(item);
    let mode_bonus = mode_bonus(item, filter.mode);
    let recency_bonus = recency_bonus(item.last_accessed_epoch_secs, now_epoch_secs);
    let frequency_bonus = frequency_bonus(item.use_count);

    Some(text_score + source_bonus + mode_bonus + recency_bonus + frequency_bonus)
}

fn score_text(normalized_title: &str, query: &str) -> Option<i64> {
    if normalized_title.is_empty() || query.is_empty() {
        return None;
    }

    let length_penalty = (normalized_title.len() as i64 - query.len() as i64).abs();
    let compact_bonus = (query.len() as i64) * 45;

    if normalized_title == query {
        return Some(SCORE_EXACT + compact_bonus - length_penalty);
    }

    if normalized_title.starts_with(query) {
        return Some(SCORE_PREFIX + compact_bonus - length_penalty);
    }

    if let Some(position) = normalized_title.find(query) {
        let position_penalty = (position as i64) * 3;
        return Some(SCORE_SUBSTRING + compact_bonus - position_penalty - length_penalty);
    }

    let (start_penalty, gap_penalty) = subsequence_penalties(normalized_title, query)?;
    Some(SCORE_FUZZY + compact_bonus - gap_penalty * 8 - start_penalty - length_penalty)
}

fn recency_bonus(last_accessed_epoch_secs: i64, now_epoch_secs: i64) -> i64 {
    if last_accessed_epoch_secs <= 0 || now_epoch_secs <= 0 {
        return 0;
    }

    let age_secs = if last_accessed_epoch_secs >= now_epoch_secs {
        0
    } else {
        now_epoch_secs - last_accessed_epoch_secs
    };
    match age_secs {
        0..=3_600 => 260,             // within 1 hour
        3_601..=86_400 => 220,        // within 1 day
        86_401..=604_800 => 170,      // within 7 days
        604_801..=2_592_000 => 110,   // within 30 days
        2_592_001..=7_776_000 => 60,  // within 90 days
        7_776_001..=31_536_000 => 25, // within 1 year
        _ => 0,
    }
}

fn frequency_bonus(use_count: u32) -> i64 {
    ((use_count as i64) * 18).clamp(0, 220)
}

fn mode_bonus(item: &SearchItem, mode: SearchMode) -> i64 {
    match mode {
        SearchMode::All => 0,
        SearchMode::Apps if item.kind.eq_ignore_ascii_case("app") => 550,
        SearchMode::Files
            if item.kind.eq_ignore_ascii_case("file")
                || item.kind.eq_ignore_ascii_case("folder") =>
        {
            550
        }
        SearchMode::Actions if item.kind.eq_ignore_ascii_case("action") => 550,
        SearchMode::Clipboard if item.kind.eq_ignore_ascii_case("clipboard") => 550,
        _ => -2_500,
    }
}

fn source_rank(item: &SearchItem) -> u8 {
    if item.kind.eq_ignore_ascii_case("app") {
        return 0;
    }

    if item.kind.eq_ignore_ascii_case("action") {
        return 1;
    }

    if (item.kind.eq_ignore_ascii_case("file") || item.kind.eq_ignore_ascii_case("folder"))
        && is_local_path(&item.path)
    {
        return 2;
    }

    if item.kind.eq_ignore_ascii_case("clipboard") {
        return 3;
    }

    4
}

fn source_bonus(item: &SearchItem) -> i64 {
    match source_rank(item) {
        0 => SOURCE_APP_BONUS,
        1 => SOURCE_ACTION_BONUS,
        2 => SOURCE_LOCAL_FS_BONUS,
        3 => SOURCE_CLIPBOARD_BONUS,
        _ => 0,
    }
}

fn is_default_filter(filter: &SearchFilter) -> bool {
    filter.mode == SearchMode::All
        && filter.kind_filter.is_none()
        && filter.include_groups.is_empty()
        && filter.exclude_terms.is_empty()
        && filter.modified_within.is_none()
        && filter.created_within.is_none()
}

fn matches_mode(item: &SearchItem, mode: SearchMode) -> bool {
    match mode {
        SearchMode::All => true,
        SearchMode::Apps => item.kind.eq_ignore_ascii_case("app"),
        SearchMode::Files => {
            item.kind.eq_ignore_ascii_case("file") || item.kind.eq_ignore_ascii_case("folder")
        }
        SearchMode::Actions => item.kind.eq_ignore_ascii_case("action"),
        SearchMode::Clipboard => item.kind.eq_ignore_ascii_case("clipboard"),
    }
}

fn matches_kind_filter(item: &SearchItem, kind_filter: &str) -> bool {
    let normalized = kind_filter.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return true;
    }
    if normalized == "app" || normalized == "apps" {
        return item.kind.eq_ignore_ascii_case("app");
    }
    if normalized == "file" || normalized == "files" {
        return item.kind.eq_ignore_ascii_case("file");
    }
    if normalized == "folder" || normalized == "folders" {
        return item.kind.eq_ignore_ascii_case("folder");
    }
    if normalized == "action" || normalized == "actions" {
        return item.kind.eq_ignore_ascii_case("action");
    }
    if normalized == "clipboard" {
        return item.kind.eq_ignore_ascii_case("clipboard");
    }
    item.kind.eq_ignore_ascii_case(&normalized)
}

fn matches_term_filters(item: &SearchItem, filter: &SearchFilter) -> bool {
    let haystack = item.normalized_search_text();
    if filter
        .exclude_terms
        .iter()
        .any(|term| !term.is_empty() && haystack.contains(term))
    {
        return false;
    }

    if filter.include_groups.is_empty() {
        return true;
    }

    filter.include_groups.iter().any(|group| {
        group
            .iter()
            .all(|term| term.is_empty() || haystack.contains(term))
    })
}

fn matches_time_filters(item: &SearchItem, filter: &SearchFilter, now_epoch_secs: i64) -> bool {
    if filter.modified_within.is_none() && filter.created_within.is_none() {
        return true;
    }

    let path = Path::new(item.path.trim());
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };

    if let Some(window) = filter.modified_within {
        let Some(modified_secs) = meta
            .modified()
            .ok()
            .and_then(|v| v.duration_since(UNIX_EPOCH).ok())
            .map(|v| v.as_secs() as i64)
        else {
            return false;
        };
        if !within_window(modified_secs, now_epoch_secs, window) {
            return false;
        }
    }

    if let Some(window) = filter.created_within {
        let Some(created_secs) = meta
            .created()
            .ok()
            .and_then(|v| v.duration_since(UNIX_EPOCH).ok())
            .map(|v| v.as_secs() as i64)
        else {
            return false;
        };
        if !within_window(created_secs, now_epoch_secs, window) {
            return false;
        }
    }

    true
}

fn within_window(value_secs: i64, now_secs: i64, window: TimeFilterWindow) -> bool {
    if value_secs <= 0 || now_secs <= 0 || value_secs > now_secs {
        return false;
    }
    let age = now_secs - value_secs;
    match window {
        TimeFilterWindow::Today => age <= 24 * 60 * 60,
        TimeFilterWindow::Week => age <= 7 * 24 * 60 * 60,
        TimeFilterWindow::Month => age <= 31 * 24 * 60 * 60,
    }
}

fn is_local_path(path: &str) -> bool {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.contains("://") {
        return false;
    }
    if trimmed.starts_with("\\\\") {
        return false;
    }

    let bytes = trimmed.as_bytes();
    if bytes.len() >= 3 && bytes[1] == b':' && (bytes[2] == b'\\' || bytes[2] == b'/') {
        return true;
    }

    trimmed.starts_with('/')
}

fn subsequence_penalties(haystack: &str, needle: &str) -> Option<(i64, i64)> {
    let mut next_start = 0;
    let mut start_penalty: Option<i64> = None;
    let mut previous_position: Option<usize> = None;
    let mut gap_penalty = 0_i64;

    for needle_char in needle.chars() {
        let mut found: Option<(usize, usize)> = None;
        for (offset, hay_char) in haystack[next_start..].char_indices() {
            if hay_char == needle_char {
                let absolute = next_start + offset;
                found = Some((absolute, hay_char.len_utf8()));
                break;
            }
        }

        let (position, char_len) = found?;
        if start_penalty.is_none() {
            start_penalty = Some(position as i64);
        }
        if let Some(previous) = previous_position {
            gap_penalty += position.saturating_sub(previous + 1) as i64;
        }
        previous_position = Some(position);
        next_start = position + char_len;
    }

    Some((start_penalty.unwrap_or(0), gap_penalty))
}

fn now_epoch_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs() as i64)
        .unwrap_or(0)
}
