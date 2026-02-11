#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchItem {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub path: String,
    normalized_title: String,
}

impl SearchItem {
    pub fn new(id: &str, kind: &str, title: &str, path: &str) -> Self {
        Self::from_owned(
            id.to_string(),
            kind.to_string(),
            title.to_string(),
            path.to_string(),
        )
    }

    pub fn from_owned(id: String, kind: String, title: String, path: String) -> Self {
        let normalized_title = normalize_for_search(&title);
        Self {
            id,
            kind,
            title,
            path,
            normalized_title,
        }
    }

    pub fn normalized_title(&self) -> &str {
        &self.normalized_title
    }
}

pub fn normalize_for_search(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}
