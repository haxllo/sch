use crate::model::SearchItem;

pub fn search(items: &[SearchItem], query: &str, limit: usize) -> Vec<SearchItem> {
    let q = query.to_lowercase().replace(' ', "");
    let mut out: Vec<SearchItem> = items
        .iter()
        .filter(|i| {
            i.title
                .to_lowercase()
                .replace('_', "")
                .contains(&q[..2.min(q.len())])
        })
        .cloned()
        .collect();
    out.truncate(limit);
    out
}
