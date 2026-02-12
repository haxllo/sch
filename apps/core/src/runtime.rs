use crate::config::{self, ConfigError};
use crate::core_service::{CoreService, LaunchTarget, ServiceError};
use crate::hotkey_runtime::HotkeyRuntimeError;
#[cfg(target_os = "windows")]
use crate::overlay_state::{HotkeyAction, OverlayState};
#[cfg(target_os = "windows")]
use crate::hotkey_runtime::{default_hotkey_registrar, HotkeyRegistration};
#[cfg(target_os = "windows")]
use crate::windows_overlay::{NativeOverlayShell, OverlayEvent};

#[derive(Debug)]
pub enum RuntimeError {
    Config(ConfigError),
    Service(ServiceError),
    Hotkey(HotkeyRuntimeError),
    Overlay(String),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(error) => write!(f, "config error: {error}"),
            Self::Service(error) => write!(f, "service error: {error}"),
            Self::Hotkey(error) => write!(f, "hotkey runtime error: {error:?}"),
            Self::Overlay(error) => write!(f, "overlay error: {error}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<ConfigError> for RuntimeError {
    fn from(value: ConfigError) -> Self {
        Self::Config(value)
    }
}

impl From<ServiceError> for RuntimeError {
    fn from(value: ServiceError) -> Self {
        Self::Service(value)
    }
}

impl From<HotkeyRuntimeError> for RuntimeError {
    fn from(value: HotkeyRuntimeError) -> Self {
        Self::Hotkey(value)
    }
}

pub fn run() -> Result<(), RuntimeError> {
    let config = config::load(None)?;
    println!(
        "[swiftfind-core] startup mode={} hotkey={} config_path={} index_db_path={}",
        runtime_mode(),
        config.hotkey,
        config.config_path.display(),
        config.index_db_path.display(),
    );

    let service = CoreService::new(config.clone())?.with_runtime_providers();
    let indexed = service.rebuild_index()?;
    println!("[swiftfind-core] startup indexed_items={indexed}");

    #[cfg(target_os = "windows")]
    {
        let _single_instance = match acquire_single_instance_guard() {
            Ok(guard) => guard,
            Err(error) => return Err(RuntimeError::Overlay(error)),
        };
        if _single_instance.is_none() {
            println!("[swiftfind-core] runtime already active; exiting duplicate process");
            return Ok(());
        }

        let mut overlay_state = OverlayState::default();
        let overlay = NativeOverlayShell::create().map_err(RuntimeError::Overlay)?;
        overlay.set_status_text("Ready. Press Alt+Space to open launcher.");
        println!("[swiftfind-core] native overlay shell initialized (hidden)");

        let mut registrar = default_hotkey_registrar();
        let registration = registrar.register_hotkey(&config.hotkey)?;
        log_registration(&registration);
        println!("[swiftfind-core] event loop running (native overlay)");

        let max_results = config.max_results as usize;
        let mut current_results: Vec<crate::model::SearchItem> = Vec::new();
        let mut selected_index = 0_usize;

        overlay
            .run_message_loop_with_events(|event| match event {
                OverlayEvent::Hotkey(_) => {
                    println!("[swiftfind-core] hotkey_event received");
                    overlay_state.set_visible(overlay.is_visible());
                    let action = overlay_state.on_hotkey(overlay.has_focus());
                    match action {
                        HotkeyAction::ShowAndFocus | HotkeyAction::FocusExisting => {
                            overlay.show_and_focus();
                        }
                        HotkeyAction::Hide => {
                            overlay.hide();
                        }
                    }
                }
                OverlayEvent::Escape => {
                    if overlay_state.on_escape() {
                        overlay.hide();
                    }
                }
                OverlayEvent::QueryChanged(query) => {
                    let trimmed = query.trim();
                    if trimmed.is_empty() {
                        current_results.clear();
                        selected_index = 0;
                        overlay.set_results(&[], 0);
                        overlay.set_status_text("");
                        return;
                    }

                    match search_overlay_results(&service, trimmed, max_results) {
                        Ok(results) => {
                            current_results = results;
                            selected_index = 0;
                            let rows = overlay_rows(&current_results);
                            overlay.set_results(&rows, selected_index);
                            if current_results.is_empty() {
                                overlay.set_status_text("No results");
                            } else {
                                overlay.set_status_text("");
                            }
                        }
                        Err(error) => {
                            current_results.clear();
                            selected_index = 0;
                            overlay.set_results(&[], 0);
                            overlay.set_status_text(&format!("Search error: {error}"));
                        }
                    }
                }
                OverlayEvent::MoveSelection(direction) => {
                    if current_results.is_empty() {
                        return;
                    }

                    let max = current_results.len() - 1;
                    if direction < 0 {
                        selected_index = selected_index.saturating_sub(1);
                    } else if direction > 0 {
                        selected_index = (selected_index + 1).min(max);
                    }

                    let rows = overlay_rows(&current_results);
                    overlay.set_results(&rows, selected_index);
                }
                OverlayEvent::Submit => {
                    if current_results.is_empty() {
                        overlay.set_status_text("No result selected");
                        return;
                    }

                    if let Some(list_selection) = overlay.selected_index() {
                        selected_index = list_selection.min(current_results.len() - 1);
                    }

                    match launch_overlay_selection(&service, &current_results, selected_index) {
                        Ok(()) => {
                            overlay.set_status_text("");
                            overlay.hide();
                            overlay_state.on_escape();
                            overlay.clear_query_text();
                            current_results.clear();
                            selected_index = 0;
                            overlay.set_results(&[], 0);
                        }
                        Err(error) => {
                            overlay.set_status_text(&format!("Launch error: {error}"));
                        }
                    }
                }
            })
            .map_err(RuntimeError::Overlay)?;
        registrar.unregister_all()?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("[swiftfind-core] non-windows runtime mode: no global hotkey loop");
        Ok(())
    }
}

fn runtime_mode() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "windows-hotkey-runtime"
    }

    #[cfg(not(target_os = "windows"))]
    {
        "non-windows-noop"
    }
}

#[cfg(target_os = "windows")]
fn overlay_rows(results: &[crate::model::SearchItem]) -> Vec<String> {
    results
        .iter()
        .map(|item| format!("{}  -  {}", item.title, item.path))
        .collect()
}

#[cfg(target_os = "windows")]
fn log_registration(registration: &HotkeyRegistration) {
    match registration {
        HotkeyRegistration::Native(id) => {
            println!("[swiftfind-core] hotkey registered native_id={id}");
        }
        HotkeyRegistration::Noop(label) => {
            println!("[swiftfind-core] hotkey registered noop={label}");
        }
    }
}

#[cfg(target_os = "windows")]
struct SingleInstanceGuard {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(target_os = "windows")]
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

#[cfg(target_os = "windows")]
fn acquire_single_instance_guard() -> Result<Option<SingleInstanceGuard>, String> {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::System::Threading::CreateMutexW;

    let mutex_name = to_wide("Local\\SwiftFindRuntimeSingleton");
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, mutex_name.as_ptr()) };
    if handle.is_null() {
        let error = unsafe { GetLastError() };
        return Err(format!("CreateMutexW failed with error {error}"));
    }

    // ERROR_ALREADY_EXISTS
    let error = unsafe { GetLastError() };
    if error == 183 {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(handle);
        }
        return Ok(None);
    }

    Ok(Some(SingleInstanceGuard { handle }))
}

#[cfg(target_os = "windows")]
fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn search_overlay_results(
    service: &CoreService,
    query: &str,
    result_limit: usize,
) -> Result<Vec<crate::model::SearchItem>, String> {
    service
        .search(query, result_limit)
        .map_err(|error| format!("search failed: {error}"))
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn launch_overlay_selection(
    service: &CoreService,
    results: &[crate::model::SearchItem],
    selected_index: usize,
) -> Result<(), String> {
    if results.is_empty() {
        return Err("no result selected".to_string());
    }

    if selected_index >= results.len() {
        return Err(format!(
            "selected index out of range: {selected_index} (len={})",
            results.len()
        ));
    }

    let selected = &results[selected_index];
    service
        .launch(LaunchTarget::Id(&selected.id))
        .map_err(|error| format!("launch failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{launch_overlay_selection, search_overlay_results};
    use crate::config::Config;
    use crate::core_service::CoreService;
    use crate::index_store::open_memory;
    use crate::model::SearchItem;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn overlay_search_returns_ranked_results() {
        let service = CoreService::with_connection(Config::default(), open_memory().unwrap())
            .expect("service should initialize");
        service
            .upsert_item(&SearchItem::new(
                "item-1",
                "app",
                "Visual Studio Code",
                "C:\\Code.exe",
            ))
            .expect("item should upsert");

        let results = search_overlay_results(&service, "code", 20).expect("search should succeed");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "item-1");
    }

    #[test]
    fn overlay_launch_selection_launches_selected_item() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be valid")
            .as_nanos();
        let launch_path = std::env::temp_dir().join(format!("swiftfind-launch-flow-{unique}.tmp"));
        std::fs::write(&launch_path, b"ok").expect("temp launch file should be created");

        let service = CoreService::with_connection(Config::default(), open_memory().unwrap())
            .expect("service should initialize");
        service
            .upsert_item(&SearchItem::new(
                "item-1",
                "file",
                "Code Launcher",
                launch_path.to_string_lossy().as_ref(),
            ))
            .expect("item should upsert");

        let results = search_overlay_results(&service, "code", 20).expect("search should succeed");
        launch_overlay_selection(&service, &results, 0).expect("launch should succeed");

        std::fs::remove_file(&launch_path).expect("temp launch file should be removed");
    }

    #[test]
    fn overlay_launch_selection_reports_error_for_missing_path() {
        let missing_path = std::env::temp_dir().join("swiftfind-does-not-exist-launch-flow.exe");
        let service = CoreService::with_connection(Config::default(), open_memory().unwrap())
            .expect("service should initialize");
        service
            .upsert_item(&SearchItem::new(
                "missing",
                "file",
                "Missing Item",
                missing_path.to_string_lossy().as_ref(),
            ))
            .expect("item should upsert");

        let results = search_overlay_results(&service, "missing", 20).expect("search should succeed");
        let error = launch_overlay_selection(&service, &results, 0).expect_err("launch should fail");

        assert!(error.contains("launch failed:"));
    }
}
