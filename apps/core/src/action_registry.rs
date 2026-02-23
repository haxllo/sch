use crate::model::{normalize_for_search, SearchItem};

pub const ACTION_OPEN_LOGS_ID: &str = "__swiftfind_action_open_logs__";
pub const ACTION_REBUILD_INDEX_ID: &str = "__swiftfind_action_rebuild_index__";
pub const ACTION_CLEAR_CLIPBOARD_ID: &str = "__swiftfind_action_clear_clipboard__";
pub const ACTION_OPEN_CONFIG_ID: &str = "__swiftfind_action_open_config__";
pub const ACTION_DIAGNOSTICS_BUNDLE_ID: &str = "__swiftfind_action_diagnostics_bundle__";
pub const ACTION_WEB_SEARCH_PREFIX: &str = "__swiftfind_action_web_search__:";

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
    search_actions_with_mode(query, limit, false)
}

pub fn search_actions_with_mode(query: &str, limit: usize, command_mode: bool) -> Vec<SearchItem> {
    if limit == 0 {
        return Vec::new();
    }
    let trimmed_query = query.trim();
    let normalized = normalize_for_search(trimmed_query);
    let mut out = Vec::new();

    if command_mode {
        if let Some(web_action) = dynamic_web_search_action(trimmed_query) {
            out.push(web_action);
            if out.len() >= limit {
                return out;
            }
        }
    }

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

fn dynamic_web_search_action(query: &str) -> Option<SearchItem> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return None;
    }
    let encoded = url_encode_component(trimmed);
    let url = format!("https://duckduckgo.com/?q={encoded}");
    let id = format!("{ACTION_WEB_SEARCH_PREFIX}{trimmed}");
    Some(SearchItem::new(
        &id,
        "action",
        &format!("Search Web for \"{trimmed}\""),
        &url,
    ))
}

fn url_encode_component(input: &str) -> String {
    let mut out = String::new();
    for byte in input.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else if byte == b' ' {
            out.push('+');
        } else {
            out.push('%');
            out.push_str(&format!("{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{search_actions, search_actions_with_mode, ACTION_WEB_SEARCH_PREFIX};

    #[test]
    fn filters_actions_by_query() {
        let actions = search_actions("diag", 10);
        assert!(actions
            .iter()
            .any(|action| action.id == "__swiftfind_action_diagnostics_bundle__"));
    }

    #[test]
    fn command_mode_includes_web_search_action() {
        let actions = search_actions_with_mode("rust icons", 10, true);
        assert!(actions
            .iter()
            .any(|action| action.id.starts_with(ACTION_WEB_SEARCH_PREFIX)));
    }

    #[test]
    fn non_command_mode_omits_web_search_action() {
        let actions = search_actions_with_mode("rust icons", 10, false);
        assert!(!actions
            .iter()
            .any(|action| action.id.starts_with(ACTION_WEB_SEARCH_PREFIX)));
    }
}
