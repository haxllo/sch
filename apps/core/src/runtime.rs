use crate::config::{self, ConfigError};
use crate::core_service::{CoreService, LaunchTarget, ServiceError};
use crate::hotkey_runtime::HotkeyRuntimeError;
#[cfg(target_os = "windows")]
use crate::hotkey_runtime::{default_hotkey_registrar, HotkeyRegistration};
#[cfg(target_os = "windows")]
use crate::overlay_state::{HotkeyAction, OverlayState};
#[cfg(target_os = "windows")]
use crate::windows_overlay::{
    is_instance_window_present, signal_existing_instance_quit, signal_existing_instance_show,
    NativeOverlayShell, OverlayEvent, OverlayRow,
};
use std::sync::atomic::{AtomicBool, Ordering};

const ACTION_OPEN_LOGS_ID: &str = "__swiftfind_action_open_logs__";
static STDIO_LOGGING_ENABLED: AtomicBool = AtomicBool::new(true);

#[derive(Debug)]
pub enum RuntimeError {
    Args(String),
    Config(ConfigError),
    Service(ServiceError),
    Hotkey(HotkeyRuntimeError),
    Overlay(String),
    Startup(crate::startup::StartupError),
    Io(std::io::Error),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Args(error) => write!(f, "argument error: {error}"),
            Self::Config(error) => write!(f, "config error: {error}"),
            Self::Service(error) => write!(f, "service error: {error}"),
            Self::Hotkey(error) => write!(f, "hotkey runtime error: {error:?}"),
            Self::Overlay(error) => write!(f, "overlay error: {error}"),
            Self::Startup(error) => write!(f, "startup error: {error}"),
            Self::Io(error) => write!(f, "io error: {error}"),
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

impl From<crate::startup::StartupError> for RuntimeError {
    fn from(value: crate::startup::StartupError) -> Self {
        Self::Startup(value)
    }
}

impl From<std::io::Error> for RuntimeError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeCommand {
    Run,
    Status,
    Quit,
    Restart,
    EnsureConfig,
    SyncStartup,
    DiagnosticsBundle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeOptions {
    pub command: RuntimeCommand,
    pub background: bool,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            command: RuntimeCommand::Run,
            background: false,
        }
    }
}

pub fn parse_cli_args(args: &[String]) -> Result<RuntimeOptions, String> {
    let mut options = RuntimeOptions::default();
    for arg in args {
        match arg.as_str() {
            "--background" => options.background = true,
            "--foreground" => options.background = false,
            "--status" => options.command = RuntimeCommand::Status,
            "--quit" => options.command = RuntimeCommand::Quit,
            "--restart" => options.command = RuntimeCommand::Restart,
            "--ensure-config" => options.command = RuntimeCommand::EnsureConfig,
            "--sync-startup" => options.command = RuntimeCommand::SyncStartup,
            "--diagnostics-bundle" => options.command = RuntimeCommand::DiagnosticsBundle,
            "--help" | "-h" => {
                return Err(
                    "usage: swiftfind-core [--background|--foreground] [--status|--quit|--restart|--ensure-config|--sync-startup|--diagnostics-bundle]".to_string(),
                )
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }

    if options.command != RuntimeCommand::Run && options.background {
        return Err("background mode is only valid with normal run mode".to_string());
    }

    Ok(options)
}

pub fn run() -> Result<(), RuntimeError> {
    run_with_options(RuntimeOptions::default())
}

pub fn run_with_options(options: RuntimeOptions) -> Result<(), RuntimeError> {
    configure_stdio_logging(options);

    if let Err(error) = crate::logging::init() {
        log_warn(&format!("[swiftfind-core] logging init warning: {error}"));
    }

    #[cfg(target_os = "windows")]
    if options.background && options.command == RuntimeCommand::Run {
        return spawn_background_process();
    }

    match options.command {
        RuntimeCommand::Status => return command_status(),
        RuntimeCommand::Quit => return command_quit(),
        RuntimeCommand::Restart => return command_restart(),
        RuntimeCommand::EnsureConfig => return command_ensure_config(),
        RuntimeCommand::SyncStartup => return command_sync_startup(),
        RuntimeCommand::DiagnosticsBundle => return command_diagnostics_bundle(),
        RuntimeCommand::Run => {}
    }

    let config = config::load(None)?;
    #[cfg(target_os = "windows")]
    let mut first_run_onboarding = false;
    if !config.config_path.exists() {
        config::write_user_template(&config, &config.config_path)?;
        #[cfg(target_os = "windows")]
        {
            first_run_onboarding = true;
        }
        log_info(&format!(
            "[swiftfind-core] wrote user config template to {}",
            config.config_path.display()
        ));
    }
    log_info(&format!(
        "[swiftfind-core] startup mode={} hotkey={} config_path={} index_db_path={}",
        runtime_mode(),
        config.hotkey,
        config.config_path.display(),
        config.index_db_path.display(),
    ));

    let service = CoreService::new(config.clone())?.with_runtime_providers();
    let index_report = service.rebuild_index_incremental_with_report()?;
    log_info(&format!(
        "[swiftfind-core] startup indexed_items={} discovered={} upserted={} removed={}",
        index_report.indexed_total,
        index_report.discovered_total,
        index_report.upserted_total,
        index_report.removed_total,
    ));
    for provider in &index_report.providers {
        log_info(&format!(
            "[swiftfind-core] index_provider name={} discovered={} upserted={} removed={} skipped={} elapsed_ms={}",
            provider.provider,
            provider.discovered,
            provider.upserted,
            provider.removed,
            provider.skipped,
            provider.elapsed_ms,
        ));
    }

    #[cfg(target_os = "windows")]
    {
        // Opt into per-monitor DPI awareness to avoid bitmap-scaled blur on high-DPI systems.
        unsafe {
            let _ = windows_sys::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(
                windows_sys::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
            );
        }

        if let Ok(exe) = std::env::current_exe() {
            if let Err(error) = crate::startup::set_enabled(config.launch_at_startup, &exe) {
                log_warn(&format!("[swiftfind-core] startup sync warning: {error}"));
            }
        }

        let _single_instance = match acquire_single_instance_guard() {
            Ok(guard) => guard,
            Err(error) => return Err(RuntimeError::Overlay(error)),
        };
        if _single_instance.is_none() {
            let _ = signal_existing_instance_show();
            log_info("[swiftfind-core] runtime already active; signaled existing instance");
            return Ok(());
        }

        let mut overlay_state = OverlayState::default();
        let overlay = NativeOverlayShell::create().map_err(RuntimeError::Overlay)?;
        overlay.set_help_config_path(config.config_path.to_string_lossy().as_ref());
        overlay.set_hotkey_hint(&config.hotkey);
        log_info("[swiftfind-core] native overlay shell initialized (hidden)");

        let mut registrar = default_hotkey_registrar();
        let registration = registrar.register_hotkey(&config.hotkey)?;
        log_registration(&registration);
        log_info("[swiftfind-core] event loop running (native overlay)");

        let max_results = config.max_results as usize;
        let mut current_results: Vec<crate::model::SearchItem> = Vec::new();
        let mut selected_index = 0_usize;
        let mut last_query = String::new();

        overlay
            .run_message_loop_with_events(|event| match event {
                OverlayEvent::Hotkey(_) => {
                    log_info("[swiftfind-core] hotkey_event received");
                    overlay_state.set_visible(overlay.is_visible());
                    let action = overlay_state.on_hotkey(overlay.has_focus());
                    match action {
                        HotkeyAction::ShowAndFocus | HotkeyAction::FocusExisting => {
                            overlay.show_and_focus();
                            if overlay.query_text().trim().is_empty() {
                                set_idle_overlay_state(&overlay);
                                if first_run_onboarding {
                                    overlay.set_status_text(&onboarding_hint(&config.hotkey));
                                    first_run_onboarding = false;
                                }
                            }
                        }
                        HotkeyAction::Hide => {
                            overlay.hide();
                            reset_overlay_session(
                                &overlay,
                                &mut current_results,
                                &mut selected_index,
                            );
                            last_query.clear();
                        }
                    }
                }
                OverlayEvent::ExternalShow => {
                    overlay_state.set_visible(overlay.is_visible());
                    overlay.show_and_focus();
                    if overlay.query_text().trim().is_empty() {
                        set_idle_overlay_state(&overlay);
                        if first_run_onboarding {
                            overlay.set_status_text(&onboarding_hint(&config.hotkey));
                            first_run_onboarding = false;
                        }
                    }
                }
                OverlayEvent::ExternalQuit => {
                    overlay.hide_now();
                    last_query.clear();
                    unsafe {
                        windows_sys::Win32::UI::WindowsAndMessaging::PostQuitMessage(0);
                    }
                }
                OverlayEvent::Escape => {
                    if overlay_state.on_escape() {
                        overlay.hide_now();
                        reset_overlay_session(&overlay, &mut current_results, &mut selected_index);
                        last_query.clear();
                    }
                }
                OverlayEvent::QueryChanged(query) => {
                    let trimmed = query.trim();
                    if trimmed.is_empty() {
                        current_results.clear();
                        selected_index = 0;
                        last_query.clear();
                        set_idle_overlay_state(&overlay);
                        return;
                    }
                    if trimmed == last_query {
                        return;
                    }
                    last_query = trimmed.to_string();

                    match search_overlay_results(&service, trimmed, max_results) {
                        Ok(mut results) => {
                            prepend_runtime_actions(trimmed, max_results, &mut results);
                            dedupe_overlay_results(&mut results);
                            current_results = results;
                            selected_index = 0;
                            if current_results.is_empty() {
                                overlay.set_results(&[], 0);
                                overlay.set_status_text("No results");
                            } else {
                                let rows = overlay_rows(&current_results);
                                overlay.set_results(&rows, selected_index);
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

                    selected_index =
                        next_selection_index(selected_index, current_results.len(), direction);
                    overlay.set_selected_index(selected_index);
                }
                OverlayEvent::Submit => {
                    if current_results.is_empty() {
                        overlay.set_status_text("No launchable result selected.");
                        return;
                    }

                    if let Some(list_selection) = overlay.selected_index() {
                        selected_index = list_selection.min(current_results.len() - 1);
                    }

                    match launch_overlay_selection(&service, &current_results, selected_index) {
                        Ok(()) => {
                            overlay.set_status_text("");
                            overlay.hide_now();
                            overlay_state.on_escape();
                            reset_overlay_session(
                                &overlay,
                                &mut current_results,
                                &mut selected_index,
                            );
                            last_query.clear();
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
        log_info("[swiftfind-core] non-windows runtime mode: no global hotkey loop");
        Ok(())
    }
}

fn command_ensure_config() -> Result<(), RuntimeError> {
    let cfg = config::load(None)?;
    if !cfg.config_path.exists() {
        config::write_user_template(&cfg, &cfg.config_path)?;
        log_info(&format!(
            "[swiftfind-core] wrote user config template to {}",
            cfg.config_path.display()
        ));
    }
    log_info(&format!(
        "[swiftfind-core] config ready at {}",
        cfg.config_path.display()
    ));
    Ok(())
}

fn command_sync_startup() -> Result<(), RuntimeError> {
    #[cfg(target_os = "windows")]
    {
        let cfg = config::load(None)?;
        let exe = std::env::current_exe()?;
        crate::startup::set_enabled(cfg.launch_at_startup, &exe)?;
        log_info(&format!(
            "[swiftfind-core] startup registration synced: enabled={}",
            cfg.launch_at_startup
        ));
        return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    {
        log_info("[swiftfind-core] startup sync is unsupported on this platform");
        Ok(())
    }
}

fn command_status() -> Result<(), RuntimeError> {
    #[cfg(target_os = "windows")]
    {
        let state = inspect_runtime_process_state();
        let running = state.has_overlay_window;
        log_info(&format!(
            "[swiftfind-core] status: {}",
            if running {
                "running"
            } else if !state.other_runtime_pids.is_empty() {
                "degraded (process without overlay window)"
            } else {
                "stopped"
            }
        ));
        if !state.other_runtime_pids.is_empty() {
            log_warn(&format!(
                "[swiftfind-core] status detected runtime_pids_without_window={:?} recommendation=run --restart",
                state.other_runtime_pids
            ));
        }
        if let Some(snapshot) = load_status_diagnostics_snapshot() {
            if let Some(line) = snapshot.startup_index_line {
                log_info(&format!("[swiftfind-core] status last_indexing {line}"));
            }
            if let Some(line) = snapshot.last_provider_line {
                log_info(&format!("[swiftfind-core] status last_provider {line}"));
            }
            if let Some(line) = snapshot.last_icon_cache_line {
                log_info(&format!("[swiftfind-core] status last_icon_cache {line}"));
            }
        }
        return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    {
        log_info("[swiftfind-core] status: unsupported on this platform");
        Ok(())
    }
}

fn command_diagnostics_bundle() -> Result<(), RuntimeError> {
    let cfg = config::load(None)?;
    let output_dir = write_diagnostics_bundle(&cfg)?;
    log_info(&format!(
        "[swiftfind-core] diagnostics bundle written to {}",
        output_dir.display()
    ));
    Ok(())
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct StatusDiagnosticsSnapshot {
    startup_index_line: Option<String>,
    last_provider_line: Option<String>,
    last_icon_cache_line: Option<String>,
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn load_status_diagnostics_snapshot() -> Option<StatusDiagnosticsSnapshot> {
    let log_path = crate::logging::logs_dir().join("swiftfind.log");
    let content = std::fs::read_to_string(&log_path).ok()?;
    parse_status_diagnostics_snapshot(&content)
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn parse_status_diagnostics_snapshot(content: &str) -> Option<StatusDiagnosticsSnapshot> {
    let startup_index_line = latest_line_with_token(content, "startup indexed_items=");
    let last_provider_line = latest_line_with_token(content, "index_provider name=");
    let last_icon_cache_line = latest_line_with_token(content, "overlay_icon_cache reason=");

    if startup_index_line.is_none() && last_provider_line.is_none() && last_icon_cache_line.is_none()
    {
        return None;
    }

    Some(StatusDiagnosticsSnapshot {
        startup_index_line,
        last_provider_line,
        last_icon_cache_line,
    })
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn latest_line_with_token(content: &str, token: &str) -> Option<String> {
    content
        .lines()
        .rev()
        .find(|line| line.contains(token))
        .map(str::to_string)
}

fn command_quit() -> Result<(), RuntimeError> {
    #[cfg(target_os = "windows")]
    {
        match stop_runtime_instance(std::time::Duration::from_secs(3))? {
            StopRuntimeOutcome::AlreadyStopped => {
                log_info("[swiftfind-core] quit skipped (not running)");
                Ok(())
            }
            StopRuntimeOutcome::Graceful => {
                log_info("[swiftfind-core] quit completed (graceful)");
                Ok(())
            }
            StopRuntimeOutcome::Forced => {
                log_warn("[swiftfind-core] quit required forced process termination");
                Ok(())
            }
            StopRuntimeOutcome::Failed => Err(RuntimeError::Overlay(
                "quit failed: runtime is still active after graceful and forced attempts"
                    .to_string(),
            )),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        log_info("[swiftfind-core] quit is unsupported on this platform");
        Ok(())
    }
}

fn command_restart() -> Result<(), RuntimeError> {
    #[cfg(target_os = "windows")]
    {
        match stop_runtime_instance(std::time::Duration::from_secs(3))? {
            StopRuntimeOutcome::Failed => {
                return Err(RuntimeError::Overlay(
                    "restart failed: existing runtime could not be stopped".to_string(),
                ));
            }
            StopRuntimeOutcome::Forced => {
                log_warn("[swiftfind-core] restart required forced process termination");
            }
            StopRuntimeOutcome::Graceful | StopRuntimeOutcome::AlreadyStopped => {}
        }
        run_with_options(RuntimeOptions::default())
    }

    #[cfg(not(target_os = "windows"))]
    {
        run_with_options(RuntimeOptions::default())
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeProcessState {
    has_overlay_window: bool,
    other_runtime_pids: Vec<u32>,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StopRuntimeOutcome {
    AlreadyStopped,
    Graceful,
    Forced,
    Failed,
}

#[cfg(target_os = "windows")]
fn inspect_runtime_process_state() -> RuntimeProcessState {
    RuntimeProcessState {
        has_overlay_window: is_instance_window_present(),
        other_runtime_pids: runtime_process_pids_excluding_current().unwrap_or_default(),
    }
}

#[cfg(target_os = "windows")]
fn stop_runtime_instance(timeout: std::time::Duration) -> Result<StopRuntimeOutcome, RuntimeError> {
    let mut state = inspect_runtime_process_state();
    if !state.has_overlay_window && state.other_runtime_pids.is_empty() {
        return Ok(StopRuntimeOutcome::AlreadyStopped);
    }

    if state.has_overlay_window {
        let _ = signal_existing_instance_quit().map_err(RuntimeError::Overlay)?;
        if wait_until_overlay_window_closed(timeout) {
            state = inspect_runtime_process_state();
            if state.other_runtime_pids.is_empty() {
                return Ok(StopRuntimeOutcome::Graceful);
            }
        }
    }

    let forced = force_terminate_other_runtime_processes()?;
    std::thread::sleep(std::time::Duration::from_millis(250));
    let post = inspect_runtime_process_state();
    if !post.has_overlay_window && post.other_runtime_pids.is_empty() {
        if forced {
            Ok(StopRuntimeOutcome::Forced)
        } else {
            Ok(StopRuntimeOutcome::Graceful)
        }
    } else {
        Ok(StopRuntimeOutcome::Failed)
    }
}

#[cfg(target_os = "windows")]
fn wait_until_overlay_window_closed(timeout: std::time::Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if !is_instance_window_present() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(120));
    }
    !is_instance_window_present()
}

#[cfg(target_os = "windows")]
fn force_terminate_other_runtime_processes() -> Result<bool, RuntimeError> {
    let current_pid = unsafe { windows_sys::Win32::System::Threading::GetCurrentProcessId() };
    let command = format!(
        "taskkill /F /T /FI \"IMAGENAME eq swiftfind-core.exe\" /FI \"PID ne {}\" >NUL 2>&1",
        current_pid
    );
    let status = std::process::Command::new("cmd")
        .arg("/C")
        .arg(command)
        .status()
        .map_err(RuntimeError::Io)?;
    Ok(status.success())
}

#[cfg(target_os = "windows")]
fn runtime_process_pids_excluding_current() -> Result<Vec<u32>, RuntimeError> {
    let current_pid = unsafe { windows_sys::Win32::System::Threading::GetCurrentProcessId() };
    let output = std::process::Command::new("cmd")
        .arg("/C")
        .arg("tasklist /FI \"IMAGENAME eq swiftfind-core.exe\" /FO LIST /NH")
        .output()
        .map_err(RuntimeError::Io)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut pids = parse_tasklist_pid_lines(&stdout);
    pids.retain(|pid| *pid != current_pid);
    Ok(pids)
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn parse_tasklist_pid_lines(content: &str) -> Vec<u32> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if !trimmed.to_ascii_lowercase().starts_with("pid:") {
                return None;
            }
            let value = trimmed.split(':').nth(1)?.trim();
            value.parse::<u32>().ok()
        })
        .collect()
}

fn write_diagnostics_bundle(cfg: &config::Config) -> Result<std::path::PathBuf, RuntimeError> {
    let support_dir = config::stable_app_data_dir().join("support");
    std::fs::create_dir_all(&support_dir)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let bundle_dir = support_dir.join(format!("diagnostics-{stamp}"));
    std::fs::create_dir_all(&bundle_dir)?;

    let running_state = runtime_state_summary();
    let summary = format!(
        "swiftfind diagnostics bundle\ngenerated_epoch_secs={stamp}\nruntime_state={running_state}\nconfig_path={}\nindex_db_path={}\nlogs_dir={}\n",
        cfg.config_path.display(),
        cfg.index_db_path.display(),
        crate::logging::logs_dir().display()
    );
    std::fs::write(bundle_dir.join("summary.txt"), summary)?;

    if cfg.config_path.exists() {
        let _ = std::fs::copy(&cfg.config_path, bundle_dir.join("config.raw.jsonc"));
    }

    let sanitized_cfg = serde_json::json!({
        "version": cfg.version,
        "max_results": cfg.max_results,
        "hotkey": cfg.hotkey,
        "launch_at_startup": cfg.launch_at_startup,
        "discovery_roots_count": cfg.discovery_roots.len(),
        "discovery_exclude_roots_count": cfg.discovery_exclude_roots.len()
    });
    let encoded = serde_json::to_string_pretty(&sanitized_cfg)
        .map_err(|e| RuntimeError::Args(format!("failed to encode sanitized config: {e}")))?;
    std::fs::write(bundle_dir.join("config.sanitized.json"), encoded)?;

    copy_recent_logs_to_bundle(&crate::logging::logs_dir(), &bundle_dir.join("logs"))?;

    Ok(bundle_dir)
}

fn copy_recent_logs_to_bundle(
    source_logs_dir: &std::path::Path,
    target_logs_dir: &std::path::Path,
) -> Result<(), RuntimeError> {
    std::fs::create_dir_all(target_logs_dir)?;
    if !source_logs_dir.exists() {
        return Ok(());
    }

    let mut entries = std::fs::read_dir(source_logs_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with(".log"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    entries.sort_by_key(|path| {
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    entries.reverse();

    for path in entries.into_iter().take(5) {
        if let Some(name) = path.file_name() {
            let _ = std::fs::copy(&path, target_logs_dir.join(name));
        }
    }

    Ok(())
}

fn runtime_state_summary() -> String {
    #[cfg(target_os = "windows")]
    {
        let state = inspect_runtime_process_state();
        if state.has_overlay_window {
            return "running".to_string();
        }
        if !state.other_runtime_pids.is_empty() {
            return format!(
                "degraded(process_without_overlay_window pids={:?})",
                state.other_runtime_pids
            );
        }
        "stopped".to_string()
    }

    #[cfg(not(target_os = "windows"))]
    {
        "unsupported_platform".to_string()
    }
}

#[cfg(target_os = "windows")]
fn spawn_background_process() -> Result<(), RuntimeError> {
    use std::os::windows::process::CommandExt;

    let exe = std::env::current_exe()?;
    let mut command = std::process::Command::new(exe);
    command.arg("--foreground");
    command.env("SWIFTFIND_SUPPRESS_STDIO", "1");
    command.creation_flags(0x00000008 | 0x00000200 | 0x08000000);
    command.stdin(std::process::Stdio::null());
    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());
    command.spawn()?;
    log_info("[swiftfind-core] background process started");
    Ok(())
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
fn overlay_rows(results: &[crate::model::SearchItem]) -> Vec<OverlayRow> {
    results
        .iter()
        .map(|item| OverlayRow {
            kind: item.kind.clone(),
            title: item.title.clone(),
            path: overlay_subtitle(item),
            icon_path: item.path.clone(),
        })
        .collect()
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn dedupe_overlay_results(results: &mut Vec<crate::model::SearchItem>) {
    let app_title_keys: std::collections::HashSet<String> = results
        .iter()
        .filter(|item| item.kind.eq_ignore_ascii_case("app"))
        .filter_map(|item| {
            let key = normalize_title_key(&item.title);
            if key.is_empty() {
                None
            } else {
                Some(key)
            }
        })
        .collect();

    let mut seen_app_titles = std::collections::HashSet::new();
    let mut seen_other_paths = std::collections::HashSet::new();

    results.retain(|item| {
        if item.kind.eq_ignore_ascii_case("app") {
            let key = normalize_title_key(&item.title);
            if key.is_empty() {
                return true;
            }
            return seen_app_titles.insert(key);
        }

        if item.kind.eq_ignore_ascii_case("file")
            && is_windows_shortcut_path(&item.path)
            && app_title_keys.contains(&shortcut_base_title_key(&item.title))
        {
            return false;
        }

        let key = normalize_path_key(&item.path);
        if key.is_empty() {
            return true;
        }
        seen_other_paths.insert(key)
    });
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn normalize_title_key(title: &str) -> String {
    crate::model::normalize_for_search(title.trim())
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn shortcut_base_title_key(title: &str) -> String {
    let trimmed = title.trim();
    if trimmed.len() >= 4 && trimmed[trimmed.len() - 4..].eq_ignore_ascii_case(".lnk") {
        normalize_title_key(&trimmed[..trimmed.len() - 4])
    } else {
        normalize_title_key(trimmed)
    }
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn is_windows_shortcut_path(path: &str) -> bool {
    let trimmed = path.trim();
    trimmed.len() >= 4 && trimmed[trimmed.len() - 4..].eq_ignore_ascii_case(".lnk")
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn normalize_path_key(path: &str) -> String {
    let trimmed = path.trim();
    let mut normalized = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if ch == '/' {
            normalized.push('\\');
        } else if ch.is_ascii_uppercase() {
            normalized.push(ch.to_ascii_lowercase());
        } else {
            normalized.push(ch);
        }
    }
    normalized
}

#[cfg(target_os = "windows")]
fn overlay_subtitle(item: &crate::model::SearchItem) -> String {
    if item.kind.eq_ignore_ascii_case("app") {
        return String::new();
    }
    if item.kind.eq_ignore_ascii_case("action") {
        return "Open SwiftFind logs folder".to_string();
    }
    abbreviate_path(&item.path)
}

#[cfg(target_os = "windows")]
fn abbreviate_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.contains("://") {
        return trimmed.to_string();
    }

    let normalized = trimmed.replace('/', "\\");
    let mut parts: Vec<&str> = normalized.split('\\').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return normalized;
    }

    // Strip filesystem roots (e.g. "C:") so the subtitle remains relative-looking.
    if parts.first().is_some_and(|part| part.ends_with(':')) {
        parts.remove(0);
    }

    if parts.is_empty() {
        return String::new();
    }

    let tail_count = parts.len().min(3);
    let joined_tail = parts[parts.len() - tail_count..].join("\\");
    if parts.len() > 3 {
        format!("...\\{joined_tail}")
    } else {
        joined_tail
    }
}

#[cfg(target_os = "windows")]
fn set_idle_overlay_state(overlay: &NativeOverlayShell) {
    overlay.set_results(&[], 0);
    overlay.set_status_text("");
}

#[cfg(target_os = "windows")]
fn reset_overlay_session(
    overlay: &NativeOverlayShell,
    current_results: &mut Vec<crate::model::SearchItem>,
    selected_index: &mut usize,
) {
    overlay.clear_query_text();
    current_results.clear();
    *selected_index = 0;
    set_idle_overlay_state(overlay);
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn next_selection_index(current: usize, len: usize, direction: i32) -> usize {
    if len == 0 {
        return 0;
    }

    let max = len - 1;
    if direction < 0 {
        current.saturating_sub(1)
    } else if direction > 0 {
        (current + 1).min(max)
    } else {
        current.min(max)
    }
}

#[cfg(target_os = "windows")]
fn log_registration(registration: &HotkeyRegistration) {
    match registration {
        HotkeyRegistration::Native(id) => {
            log_info(&format!(
                "[swiftfind-core] hotkey registered native_id={id}"
            ));
        }
        HotkeyRegistration::Noop(label) => {
            log_info(&format!("[swiftfind-core] hotkey registered noop={label}"));
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
    if selected.id == ACTION_OPEN_LOGS_ID {
        return crate::logging::open_logs_folder()
            .map_err(|error| format!("open logs folder failed: {error}"));
    }
    service
        .launch(LaunchTarget::Id(&selected.id))
        .map_err(|error| format!("launch failed: {error}"))
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn prepend_runtime_actions(query: &str, limit: usize, results: &mut Vec<crate::model::SearchItem>) {
    if limit == 0 {
        return;
    }

    let normalized = query.trim().to_ascii_lowercase();
    if !normalized.starts_with("log") {
        return;
    }
    if results.iter().any(|item| item.id == ACTION_OPEN_LOGS_ID) {
        return;
    }

    let logs_path = crate::logging::logs_dir();
    results.insert(
        0,
        crate::model::SearchItem::new(
            ACTION_OPEN_LOGS_ID,
            "action",
            "Open SwiftFind Logs Folder",
            logs_path.to_string_lossy().as_ref(),
        ),
    );
    if results.len() > limit {
        results.truncate(limit);
    }
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn onboarding_hint(hotkey: &str) -> String {
    format!("Welcome to SwiftFind. Hotkey: {hotkey}. If this conflicts, click ? to edit config.")
}

fn configure_stdio_logging(options: RuntimeOptions) {
    let suppress_from_env = std::env::var("SWIFTFIND_SUPPRESS_STDIO")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let suppress_for_background = options.command == RuntimeCommand::Run && options.background;
    STDIO_LOGGING_ENABLED.store(
        !(suppress_from_env || suppress_for_background),
        Ordering::Relaxed,
    );
}

fn should_log_to_stdio() -> bool {
    STDIO_LOGGING_ENABLED.load(Ordering::Relaxed)
}

fn log_info(message: &str) {
    if should_log_to_stdio() {
        println!("{message}");
    }
    crate::logging::info(message);
}

fn log_warn(message: &str) {
    if should_log_to_stdio() {
        eprintln!("{message}");
    }
    crate::logging::warn(message);
}

#[cfg(test)]
mod tests {
    use super::{
        dedupe_overlay_results, launch_overlay_selection, next_selection_index, parse_cli_args,
        parse_status_diagnostics_snapshot, parse_tasklist_pid_lines, prepend_runtime_actions,
        search_overlay_results, RuntimeCommand, RuntimeOptions, ACTION_OPEN_LOGS_ID,
    };
    use crate::config::Config;
    use crate::core_service::CoreService;
    use crate::index_store::open_memory;
    use crate::model::SearchItem;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn overlay_search_returns_ranked_results() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("swiftfind-overlay-search-{unique}.tmp"));
        std::fs::write(&path, b"ok").expect("temp file should be created");

        let service = CoreService::with_connection(Config::default(), open_memory().unwrap())
            .expect("service should initialize");
        service
            .upsert_item(&SearchItem::new(
                "item-1",
                "app",
                "Visual Studio Code",
                path.to_string_lossy().as_ref(),
            ))
            .expect("item should upsert");

        let results = search_overlay_results(&service, "code", 20).expect("search should succeed");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "item-1");

        std::fs::remove_file(path).expect("temp file should be removed");
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
        let item = SearchItem::new(
            "missing",
            "file",
            "Missing Item",
            missing_path.to_string_lossy().as_ref(),
        );
        service
            .upsert_item(&SearchItem::new(
                "missing",
                "file",
                "Missing Item",
                missing_path.to_string_lossy().as_ref(),
            ))
            .expect("item should upsert");

        let results = vec![item];
        let error =
            launch_overlay_selection(&service, &results, 0).expect_err("launch should fail");

        assert!(error.contains("launch failed:"));
    }

    #[test]
    fn overlay_launch_selection_rejects_out_of_range_index() {
        let service = CoreService::with_connection(Config::default(), open_memory().unwrap())
            .expect("service should initialize");
        let results = vec![SearchItem::new("item-1", "app", "One", "C:\\One.exe")];

        let error =
            launch_overlay_selection(&service, &results, 1).expect_err("selection should fail");

        assert!(error.contains("selected index out of range"));
    }

    #[test]
    fn overlay_launch_selection_rejects_empty_results() {
        let service = CoreService::with_connection(Config::default(), open_memory().unwrap())
            .expect("service should initialize");

        let error =
            launch_overlay_selection(&service, &[], 0).expect_err("empty selection should fail");

        assert_eq!(error, "no result selected");
    }

    #[test]
    fn selection_index_bounds_are_stable() {
        assert_eq!(next_selection_index(0, 0, 1), 0);
        assert_eq!(next_selection_index(0, 3, -1), 0);
        assert_eq!(next_selection_index(1, 3, -1), 0);
        assert_eq!(next_selection_index(1, 3, 1), 2);
        assert_eq!(next_selection_index(2, 3, 1), 2);
        assert_eq!(next_selection_index(1, 3, 0), 1);
        assert_eq!(next_selection_index(5, 3, 0), 2);
    }

    #[test]
    fn parses_background_run_args() {
        let args = vec!["--background".to_string()];
        let options = parse_cli_args(&args).expect("args should parse");
        assert_eq!(
            options,
            RuntimeOptions {
                command: RuntimeCommand::Run,
                background: true,
            }
        );
    }

    #[test]
    fn parses_lifecycle_commands() {
        let args = vec!["--status".to_string()];
        let options = parse_cli_args(&args).expect("status should parse");
        assert_eq!(options.command, RuntimeCommand::Status);
        assert!(!options.background);
    }

    #[test]
    fn parses_diagnostics_bundle_command() {
        let args = vec!["--diagnostics-bundle".to_string()];
        let options = parse_cli_args(&args).expect("diagnostics command should parse");
        assert_eq!(options.command, RuntimeCommand::DiagnosticsBundle);
        assert!(!options.background);
    }

    #[test]
    fn rejects_background_with_non_run_commands() {
        let args = vec!["--quit".to_string(), "--background".to_string()];
        let error = parse_cli_args(&args).expect_err("invalid combination should fail");
        assert!(error.contains("background mode"));
    }

    #[test]
    fn prepends_logs_action_for_log_query() {
        let mut results = vec![SearchItem::new("x", "file", "Example", "C:\\Example.txt")];
        prepend_runtime_actions("logs", 5, &mut results);
        assert_eq!(results[0].id, ACTION_OPEN_LOGS_ID);
        assert_eq!(results[0].kind, "action");
    }

    #[test]
    fn does_not_prepend_logs_action_for_non_log_query() {
        let mut results = vec![SearchItem::new("x", "file", "Example", "C:\\Example.txt")];
        prepend_runtime_actions("code", 5, &mut results);
        assert_ne!(results[0].id, ACTION_OPEN_LOGS_ID);
    }

    #[test]
    fn dedupes_duplicate_app_titles_for_overlay() {
        let mut results = vec![
            SearchItem::new("a1", "app", "Steam", "C:\\One\\Steam.lnk"),
            SearchItem::new("a2", "app", "Steam", "C:\\Two\\Steam.lnk"),
            SearchItem::new("a3", "app", "Calculator", "C:\\Calc.lnk"),
        ];
        dedupe_overlay_results(&mut results);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "Steam");
        assert_eq!(results[1].title, "Calculator");
    }

    #[test]
    fn dedupes_non_app_entries_by_normalized_path() {
        let mut results = vec![
            SearchItem::new("f1", "file", "Doc A", "C:/Users/Admin/Docs/test.txt"),
            SearchItem::new("f2", "file", "Doc B", "C:\\Users\\Admin\\Docs\\test.txt"),
            SearchItem::new("f3", "file", "Doc C", "C:\\Users\\Admin\\Docs\\other.txt"),
        ];
        dedupe_overlay_results(&mut results);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "f1");
        assert_eq!(results[1].id, "f3");
    }

    #[test]
    fn dedupes_lnk_file_when_matching_app_title_exists() {
        let mut results = vec![
            SearchItem::new("a1", "app", "Framer", "C:\\ProgramData\\Framer.lnk"),
            SearchItem::new("f1", "file", "Framer.lnk", "C:\\Users\\Admin\\Desktop\\Framer.lnk"),
            SearchItem::new(
                "f2",
                "file",
                "Framer Notes.lnk",
                "C:\\Users\\Admin\\Desktop\\Framer Notes.lnk",
            ),
        ];

        dedupe_overlay_results(&mut results);
        let ids: Vec<&str> = results.iter().map(|item| item.id.as_str()).collect();

        assert_eq!(ids, vec!["a1", "f2"]);
    }

    #[test]
    fn parses_status_diagnostics_snapshot_from_log_content() {
        let content = "\
[1] [INFO] [swiftfind-core] startup indexed_items=310 discovered=320 upserted=16 removed=4
[2] [INFO] [swiftfind-core] index_provider name=start-menu-apps discovered=120 upserted=4 removed=1 elapsed_ms=42
[3] [INFO] [swiftfind-core] overlay_icon_cache reason=cache_clear hits=12 misses=8 load_failures=1 evictions=0 cleared_entries=9
";

        let snapshot = parse_status_diagnostics_snapshot(content).expect("snapshot should parse");
        assert!(
            snapshot
                .startup_index_line
                .as_deref()
                .unwrap_or_default()
                .contains("startup indexed_items=310")
        );
        assert!(
            snapshot
                .last_provider_line
                .as_deref()
                .unwrap_or_default()
                .contains("index_provider name=start-menu-apps")
        );
        assert!(
            snapshot
                .last_icon_cache_line
                .as_deref()
                .unwrap_or_default()
                .contains("overlay_icon_cache reason=cache_clear")
        );
    }

    #[test]
    fn returns_none_for_status_snapshot_without_diagnostics_tokens() {
        let content = "[1] [INFO] [swiftfind-core] status: running\n";
        assert!(parse_status_diagnostics_snapshot(content).is_none());
    }

    #[test]
    fn parses_tasklist_pid_lines_from_list_output() {
        let content = "\
Image Name:   swiftfind-core.exe
PID:          1124
Session Name: Console

Image Name:   swiftfind-core.exe
PID:          2208
Session Name: Console
";
        let pids = parse_tasklist_pid_lines(content);
        assert_eq!(pids, vec![1124, 2208]);
    }
}
