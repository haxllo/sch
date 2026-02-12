#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchItem {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub path: String,
}

impl SearchItem {
    pub fn new(id: &str, kind: &str, title: &str, path: &str) -> Self {
        Self {
            id: id.to_string(),
            kind: kind.to_string(),
            title: title.to_string(),
            path: path.to_string(),
        }
    }
}
