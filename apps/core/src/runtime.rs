use crate::config::{self, ConfigError};
use crate::core_service::{CoreService, LaunchTarget, ServiceError};
use crate::hotkey_runtime::HotkeyRuntimeError;
#[cfg(target_os = "windows")]
use crate::overlay_state::{HotkeyAction, OverlayState};
#[cfg(target_os = "windows")]
use crate::hotkey_runtime::{default_hotkey_registrar, run_message_loop, HotkeyRegistration};
#[cfg(target_os = "windows")]
use crate::windows_overlay::NativeOverlayShell;
use std::io::{self, BufRead, Write};

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
        println!("[swiftfind-core] event loop running (WM_HOTKEY)");
        run_message_loop(|_| {
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
        })?;
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
fn run_console_launcher_flow(service: &CoreService, result_limit: usize) -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    run_console_launcher_flow_with_io(service, result_limit, &mut reader, &mut writer)
}

fn run_console_launcher_flow_with_io<R, W>(
    service: &CoreService,
    result_limit: usize,
    input: &mut R,
    output: &mut W,
) -> Result<(), String>
where
    R: BufRead,
    W: Write,
{
    write!(output, "[swiftfind-core] launcher> query: ").map_err(|e| e.to_string())?;
    output.flush().map_err(|e| e.to_string())?;

    let mut query_line = String::new();
    input
        .read_line(&mut query_line)
        .map_err(|e| format!("failed reading query: {e}"))?;
    let query = query_line.trim();

    if query.is_empty() {
        writeln!(output, "[swiftfind-core] launcher canceled (empty query)")
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    let results = service
        .search(query, result_limit)
        .map_err(|e| format!("search failed: {e}"))?;

    if results.is_empty() {
        writeln!(output, "[swiftfind-core] launcher no matches for '{query}'")
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    writeln!(output, "[swiftfind-core] launcher results:")
        .and_then(|_| {
            for (index, item) in results.iter().enumerate() {
                writeln!(output, "  {}. {} ({})", index + 1, item.title, item.path)?;
            }
            Ok(())
        })
        .map_err(|e: io::Error| e.to_string())?;

    write!(
        output,
        "[swiftfind-core] launcher> select [1-{}] (enter to cancel): ",
        results.len()
    )
    .map_err(|e| e.to_string())?;
    output.flush().map_err(|e| e.to_string())?;

    let mut selection_line = String::new();
    input
        .read_line(&mut selection_line)
        .map_err(|e| format!("failed reading selection: {e}"))?;
    let selection = selection_line.trim();
    if selection.is_empty() {
        writeln!(output, "[swiftfind-core] launcher canceled (no selection)")
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    let selected_number = selection
        .parse::<usize>()
        .map_err(|_| format!("invalid selection: '{selection}'"))?;
    if selected_number == 0 || selected_number > results.len() {
        return Err(format!(
            "selection out of range: {selected_number} (results={})",
            results.len()
        ));
    }

    let selected = &results[selected_number - 1];
    service
        .launch(LaunchTarget::Id(&selected.id))
        .map_err(|error| {
            let _ = writeln!(output, "[swiftfind-core] launcher error: {error}");
            format!("launch failed: {error}")
        })?;

    writeln!(
        output,
        "[swiftfind-core] launcher launched: {} ({})",
        selected.title, selected.path
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run_console_launcher_flow_with_io;
    use crate::config::Config;
    use crate::core_service::CoreService;
    use crate::index_store::open_memory;
    use crate::model::SearchItem;
    use std::io::Cursor;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn launcher_flow_searches_and_launches_selected_item() {
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

        let mut input = Cursor::new("code\n1\n");
        let mut output = Vec::new();
        run_console_launcher_flow_with_io(&service, 20, &mut input, &mut output)
            .expect("launcher flow should succeed");

        let output_text = String::from_utf8(output).expect("output should be utf8");
        assert!(output_text.contains("launcher results:"));
        assert!(output_text.contains("launcher launched: Code Launcher"));

        std::fs::remove_file(&launch_path).expect("temp launch file should be removed");
    }

    #[test]
    fn launcher_flow_reports_launch_errors() {
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

        let mut input = Cursor::new("missing\n1\n");
        let mut output = Vec::new();
        let error = run_console_launcher_flow_with_io(&service, 20, &mut input, &mut output)
            .expect_err("launcher flow should return launch failure");

        assert!(error.contains("launch failed:"));
        let output_text = String::from_utf8(output).expect("output should be utf8");
        assert!(output_text.contains("launcher error:"));
    }
}
