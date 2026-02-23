use crate::model::{normalize_for_search, SearchItem};

pub const ACTION_OPEN_LOGS_ID: &str = "__swiftfind_action_open_logs__";
pub const ACTION_REBUILD_INDEX_ID: &str = "__swiftfind_action_rebuild_index__";
pub const ACTION_CLEAR_CLIPBOARD_ID: &str = "__swiftfind_action_clear_clipboard__";
pub const ACTION_OPEN_CONFIG_ID: &str = "__swiftfind_action_open_config__";
pub const ACTION_DIAGNOSTICS_BUNDLE_ID: &str = "__swiftfind_action_diagnostics_bundle__";

#[derive(Debug, Clone, Copy)]
pub struct BuiltInAction {
    pub id: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub keywords: &'static [&'static str],
}

pub fn built_in_actions() -> &'static [BuiltInAction] {
    &[
        BuiltInAction {
            id: ACTION_OPEN_LOGS_ID,
            title: "Open SwiftFind Logs Folder",
            subtitle: "Open logs directory in File Explorer",
            keywords: &["logs", "log", "debug"],
        },
        BuiltInAction {
            id: ACTION_REBUILD_INDEX_ID,
            title: "Rebuild Search Index",
            subtitle: "Force a full refresh of indexed items",
            keywords: &["rebuild", "index", "refresh"],
        },
        BuiltInAction {
            id: ACTION_CLEAR_CLIPBOARD_ID,
            title: "Clear Clipboard History",
            subtitle: "Delete local clipboard history entries",
            keywords: &["clipboard", "clear", "history"],
        },
        BuiltInAction {
            id: ACTION_OPEN_CONFIG_ID,
            title: "Open SwiftFind Config",
            subtitle: "Open config.json",
            keywords: &["config", "settings", "preferences"],
        },
        BuiltInAction {
            id: ACTION_DIAGNOSTICS_BUNDLE_ID,
            title: "Create Diagnostics Bundle",
            subtitle: "Export logs and sanitized config for support",
            keywords: &["diagnostics", "support", "bundle", "debug"],
        },
    ]
}

pub fn search_actions(query: &str, limit: usize) -> Vec<SearchItem> {
    if limit == 0 {
        return Vec::new();
    }
    let normalized = normalize_for_search(query);
    let mut out = Vec::new();

    for action in built_in_actions() {
        if !normalized.is_empty() {
            let title_match = normalize_for_search(action.title).contains(&normalized);
            let keyword_match = action
                .keywords
                .iter()
                .any(|kw| normalize_for_search(kw).contains(&normalized));
            if !title_match && !keyword_match {
                continue;
            }
        }
        out.push(SearchItem::new(
            action.id,
            "action",
            action.title,
            action.subtitle,
        ));
        if out.len() >= limit {
            break;
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::search_actions;

    #[test]
    fn filters_actions_by_query() {
        let actions = search_actions("diag", 10);
        assert!(actions
            .iter()
            .any(|action| action.id == "__swiftfind_action_diagnostics_bundle__"));
    }
}
