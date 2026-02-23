use crate::action_registry::{
    search_actions_with_mode, ACTION_CLEAR_CLIPBOARD_ID, ACTION_DIAGNOSTICS_BUNDLE_ID,
    ACTION_OPEN_CONFIG_ID, ACTION_OPEN_LOGS_ID, ACTION_REBUILD_INDEX_ID, ACTION_WEB_SEARCH_PREFIX,
};
use crate::clipboard_history;
use crate::config::{self, Config, ConfigError};
use crate::core_service::{CoreService, LaunchTarget, ServiceError};
use crate::hotkey_runtime::HotkeyRuntimeError;
#[cfg(target_os = "windows")]
use crate::hotkey_runtime::{default_hotkey_registrar, HotkeyRegistration};
#[cfg(target_os = "windows")]
use crate::overlay_state::{HotkeyAction, OverlayState};
use crate::plugin_sdk::{PluginActionKind, PluginRegistry};
use crate::query_dsl::ParsedQuery;
use crate::search::SearchFilter;
#[cfg(target_os = "windows")]
use crate::windows_overlay::{
    is_instance_window_present, signal_existing_instance_quit, signal_existing_instance_show,
    NativeOverlayShell, OverlayEvent, OverlayRow, OverlayRowRole,
};
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(target_os = "windows")]
const STATUS_ROW_NO_RESULTS: &str = "No results";
#[cfg(target_os = "windows")]
const STATUS_ROW_TYPE_TO_SEARCH: &str = "Start typing to search";
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
    if !config.config_path.exists() {
        config::write_user_template(&config, &config.config_path)?;
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
    let plugin_registry = PluginRegistry::load_from_config(&config);
    #[cfg(target_os = "windows")]
    {
        for warning in &plugin_registry.load_warnings {
            log_warn(&format!("[swiftfind-core] plugin_warning {warning}"));
        }
        log_info(&format!(
            "[swiftfind-core] plugins loaded provider_items={} action_items={}",
            plugin_registry.provider_items.len(),
            plugin_registry.action_items.len()
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
                            if config.clipboard_enabled {
                                let _ = clipboard_history::maybe_capture_latest(&config);
                            }
                            if overlay.query_text().trim().is_empty() {
                                set_idle_overlay_state(&overlay);
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
                    if config.clipboard_enabled {
                        let _ = clipboard_history::maybe_capture_latest(&config);
                    }
                    if overlay.query_text().trim().is_empty() {
                        set_idle_overlay_state(&overlay);
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
                    let parsed_query = ParsedQuery::parse(trimmed, config.search_dsl_enabled);

                    match search_overlay_results(
                        &service,
                        &config,
                        &plugin_registry,
                        &parsed_query,
                        max_results,
                    ) {
                        Ok(mut results) => {
                            dedupe_overlay_results(&mut results);
                            current_results = results;
                            selected_index = 0;
                            if current_results.is_empty() {
                                set_status_row_overlay_state(&overlay, STATUS_ROW_NO_RESULTS);
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
                        if overlay.query_text().trim().is_empty() {
                            set_idle_overlay_state(&overlay);
                            overlay.show_placeholder_hint(STATUS_ROW_TYPE_TO_SEARCH);
                        } else {
                            set_status_row_overlay_state(&overlay, STATUS_ROW_NO_RESULTS);
                        }
                        return;
                    }

                    if let Some(list_selection) = overlay.selected_index() {
                        selected_index = list_selection.min(current_results.len() - 1);
                    }

                    match launch_overlay_selection(
                        &service,
                        &config,
                        &plugin_registry,
                        &current_results,
                        selected_index,
                    ) {
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

    if startup_index_line.is_none()
        && last_provider_line.is_none()
        && last_icon_cache_line.is_none()
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
        "search_mode_default": cfg.search_mode_default,
        "search_dsl_enabled": cfg.search_dsl_enabled,
        "clipboard_enabled": cfg.clipboard_enabled,
        "clipboard_retention_minutes": cfg.clipboard_retention_minutes,
        "clipboard_exclude_sensitive_patterns_count": cfg.clipboard_exclude_sensitive_patterns.len(),
        "plugins_enabled": cfg.plugins_enabled,
        "plugin_paths_count": cfg.plugin_paths.len(),
        "plugins_safe_mode": cfg.plugins_safe_mode,
        "idle_cache_trim_ms": cfg.idle_cache_trim_ms,
        "active_memory_target_mb": cfg.active_memory_target_mb,
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
    if results.is_empty() {
        return Vec::new();
    }

    let mut rows = Vec::new();
    rows.push(section_header_row("Top Hit"));
    rows.push(result_row(&results[0], 0, OverlayRowRole::TopHit));

    let mut app_indices = Vec::new();
    let mut file_indices = Vec::new();
    let mut action_indices = Vec::new();
    let mut clipboard_indices = Vec::new();
    let mut other_indices = Vec::new();

    for (index, item) in results.iter().enumerate().skip(1) {
        if item.kind.eq_ignore_ascii_case("app") {
            app_indices.push(index);
        } else if item.kind.eq_ignore_ascii_case("file") || item.kind.eq_ignore_ascii_case("folder")
        {
            file_indices.push(index);
        } else if item.kind.eq_ignore_ascii_case("action") {
            action_indices.push(index);
        } else if item.kind.eq_ignore_ascii_case("clipboard") {
            clipboard_indices.push(index);
        } else {
            other_indices.push(index);
        }
    }

    append_group_rows(&mut rows, "Applications", &app_indices, results);
    append_group_rows(&mut rows, "Files", &file_indices, results);
    append_group_rows(&mut rows, "Actions", &action_indices, results);
    append_group_rows(&mut rows, "Clipboard", &clipboard_indices, results);
    append_group_rows(&mut rows, "Other", &other_indices, results);
    rows
}

#[cfg(target_os = "windows")]
fn append_group_rows(
    rows: &mut Vec<OverlayRow>,
    header: &str,
    indices: &[usize],
    results: &[crate::model::SearchItem],
) {
    if indices.is_empty() {
        return;
    }
    rows.push(section_header_row(header));
    for index in indices {
        rows.push(result_row(&results[*index], *index, OverlayRowRole::Item));
    }
}

#[cfg(target_os = "windows")]
fn section_header_row(title: &str) -> OverlayRow {
    OverlayRow {
        role: OverlayRowRole::Header,
        result_index: -1,
        kind: "section".to_string(),
        title: title.to_string(),
        path: String::new(),
        icon_path: String::new(),
    }
}

#[cfg(target_os = "windows")]
fn result_row(
    item: &crate::model::SearchItem,
    result_index: usize,
    role: OverlayRowRole,
) -> OverlayRow {
    OverlayRow {
        role,
        result_index: result_index as i32,
        kind: item.kind.clone(),
        title: item.title.clone(),
        path: overlay_subtitle(item),
        icon_path: item.path.clone(),
    }
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
        if item.path.trim().is_empty() {
            return "SwiftFind action".to_string();
        }
        return item.path.trim().to_string();
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
    overlay.clear_placeholder_hint();
    overlay.set_results(&[], 0);
    overlay.set_status_text("");
}

#[cfg(target_os = "windows")]
fn set_status_row_overlay_state(overlay: &NativeOverlayShell, message: &str) {
    overlay.clear_placeholder_hint();
    let rows = [OverlayRow {
        role: OverlayRowRole::Status,
        result_index: -1,
        kind: "status".to_string(),
        title: message.to_string(),
        path: String::new(),
        icon_path: String::new(),
    }];
    overlay.set_results(&rows, 0);
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
    cfg: &Config,
    plugins: &PluginRegistry,
    parsed_query: &ParsedQuery,
    result_limit: usize,
) -> Result<Vec<crate::model::SearchItem>, String> {
    if result_limit == 0 {
        return Ok(Vec::new());
    }

    let filter = build_search_filter(cfg, parsed_query);
    let text_query = parsed_query.free_text.trim();
    let candidate_limit = result_limit.saturating_mul(6).max(60);

    let mut merged = Vec::new();
    merged.extend(
        service
            .search_with_filter(text_query, candidate_limit, &filter)
            .map_err(|error| format!("indexed search failed: {error}"))?,
    );
    merged.extend(crate::search::search_with_filter(
        &plugins.provider_items,
        text_query,
        candidate_limit,
        &filter,
    ));

    let mut action_items =
        search_actions_with_mode(text_query, candidate_limit, parsed_query.command_mode);
    if !plugins.action_items.is_empty() {
        action_items.extend(crate::search::search_with_filter(
            &plugins.action_items,
            text_query,
            candidate_limit,
            &SearchFilter {
                mode: crate::config::SearchMode::Actions,
                ..SearchFilter::default()
            },
        ));
    }
    merged.extend(crate::search::search_with_filter(
        &action_items,
        text_query,
        candidate_limit,
        &filter,
    ));

    merged.extend(clipboard_history::search_history(
        cfg,
        text_query,
        &filter,
        candidate_limit.min(120),
    ));

    Ok(crate::search::search_with_filter(
        &merged,
        text_query,
        result_limit,
        &filter,
    ))
}

fn build_search_filter(cfg: &Config, parsed_query: &ParsedQuery) -> SearchFilter {
    let mode = resolved_mode_for_query(cfg, parsed_query);
    SearchFilter {
        mode,
        kind_filter: parsed_query.kind_filter.clone(),
        include_groups: parsed_query.include_groups.clone(),
        exclude_terms: parsed_query.exclude_terms.clone(),
        modified_within: parsed_query.modified_within,
        created_within: parsed_query.created_within,
    }
}

fn resolved_mode_for_query(cfg: &Config, parsed_query: &ParsedQuery) -> crate::config::SearchMode {
    let mut mode = parsed_query
        .mode_override
        .unwrap_or(cfg.search_mode_default);
    if parsed_query.command_mode {
        mode = crate::config::SearchMode::Actions;
    }
    mode
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn launch_overlay_selection(
    service: &CoreService,
    cfg: &Config,
    plugins: &PluginRegistry,
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
    if selected.kind.eq_ignore_ascii_case("action") {
        return execute_action_selection(service, cfg, plugins, selected);
    }
    if selected.kind.eq_ignore_ascii_case("clipboard") {
        return clipboard_history::copy_result_to_clipboard(cfg, &selected.id);
    }

    service
        .launch(LaunchTarget::Id(&selected.id))
        .map_err(|error| format!("launch failed: {error}"))
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn execute_action_selection(
    service: &CoreService,
    cfg: &Config,
    plugins: &PluginRegistry,
    selected: &crate::model::SearchItem,
) -> Result<(), String> {
    if selected.id.starts_with(ACTION_WEB_SEARCH_PREFIX) {
        return crate::action_executor::launch_open_target(selected.path.trim())
            .map_err(|error| format!("web search launch failed: {error}"));
    }

    match selected.id.as_str() {
        ACTION_OPEN_LOGS_ID => crate::logging::open_logs_folder()
            .map_err(|error| format!("open logs folder failed: {error}")),
        ACTION_REBUILD_INDEX_ID => {
            let report = service
                .rebuild_index_incremental_with_report()
                .map_err(|error| format!("rebuild index failed: {error}"))?;
            log_info(&format!(
                "[swiftfind-core] action_rebuild_index indexed={} discovered={} upserted={} removed={}",
                report.indexed_total, report.discovered_total, report.upserted_total, report.removed_total
            ));
            Ok(())
        }
        ACTION_CLEAR_CLIPBOARD_ID => clipboard_history::clear_history(cfg),
        ACTION_OPEN_CONFIG_ID => {
            crate::action_executor::launch_path(cfg.config_path.to_string_lossy().as_ref())
                .map_err(|error| format!("open config failed: {error}"))
        }
        ACTION_DIAGNOSTICS_BUNDLE_ID => {
            let output_dir = write_diagnostics_bundle(cfg)
                .map_err(|error| format!("diagnostics bundle failed: {error}"))?;
            log_info(&format!(
                "[swiftfind-core] diagnostics bundle written to {}",
                output_dir.display()
            ));
            Ok(())
        }
        _ => execute_plugin_action(cfg, plugins, &selected.id),
    }
}

fn execute_plugin_action(
    cfg: &Config,
    plugins: &PluginRegistry,
    result_id: &str,
) -> Result<(), String> {
    let action = plugins
        .actions_by_result_id
        .get(result_id)
        .ok_or_else(|| "unknown action".to_string())?;

    match &action.kind {
        PluginActionKind::OpenPath { path } => crate::action_executor::launch_path(path)
            .map_err(|error| format!("plugin open path failed: {error}")),
        PluginActionKind::Command { command, args } => {
            if cfg.plugins_safe_mode {
                return Err(
                    "plugin command execution blocked: plugins_safe_mode is enabled in config"
                        .to_string(),
                );
            }
            if command.trim().is_empty() {
                return Err("plugin command action missing command".to_string());
            }
            std::process::Command::new(command)
                .args(args)
                .spawn()
                .map_err(|e| format!("plugin command spawn failed: {e}"))?;
            Ok(())
        }
    }
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
        parse_status_diagnostics_snapshot, parse_tasklist_pid_lines, search_overlay_results,
        RuntimeCommand, RuntimeOptions,
    };
    use crate::action_registry::{ACTION_DIAGNOSTICS_BUNDLE_ID, ACTION_WEB_SEARCH_PREFIX};
    use crate::config::Config;
    use crate::core_service::CoreService;
    use crate::index_store::open_memory;
    use crate::model::SearchItem;
    use crate::plugin_sdk::PluginRegistry;
    use crate::query_dsl::ParsedQuery;
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

        let parsed = ParsedQuery::parse("code", true);
        let cfg = Config::default();
        let plugins = PluginRegistry::default();
        let results = search_overlay_results(&service, &cfg, &plugins, &parsed, 20)
            .expect("search should succeed");

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

        let parsed = ParsedQuery::parse("code", true);
        let cfg = Config::default();
        let plugins = PluginRegistry::default();
        let results = search_overlay_results(&service, &cfg, &plugins, &parsed, 20)
            .expect("search should succeed");
        launch_overlay_selection(&service, &cfg, &plugins, &results, 0)
            .expect("launch should succeed");

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
        let cfg = Config::default();
        let plugins = PluginRegistry::default();
        let error = launch_overlay_selection(&service, &cfg, &plugins, &results, 0)
            .expect_err("launch should fail");

        assert!(error.contains("launch failed:"));
    }

    #[test]
    fn overlay_launch_selection_rejects_out_of_range_index() {
        let service = CoreService::with_connection(Config::default(), open_memory().unwrap())
            .expect("service should initialize");
        let results = vec![SearchItem::new("item-1", "app", "One", "C:\\One.exe")];

        let cfg = Config::default();
        let plugins = PluginRegistry::default();
        let error = launch_overlay_selection(&service, &cfg, &plugins, &results, 1)
            .expect_err("selection should fail");

        assert!(error.contains("selected index out of range"));
    }

    #[test]
    fn overlay_launch_selection_rejects_empty_results() {
        let service = CoreService::with_connection(Config::default(), open_memory().unwrap())
            .expect("service should initialize");

        let cfg = Config::default();
        let plugins = PluginRegistry::default();
        let error = launch_overlay_selection(&service, &cfg, &plugins, &[], 0)
            .expect_err("empty selection should fail");

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
    fn command_mode_returns_action_results() {
        let service = CoreService::with_connection(Config::default(), open_memory().unwrap())
            .expect("service should initialize");
        let cfg = Config::default();
        let plugins = PluginRegistry::default();
        let parsed = ParsedQuery::parse(">diag", true);
        let results = search_overlay_results(&service, &cfg, &plugins, &parsed, 10)
            .expect("search should succeed");
        assert!(results
            .iter()
            .any(|item| item.id == ACTION_DIAGNOSTICS_BUNDLE_ID));
    }

    #[test]
    fn command_mode_includes_web_search_action() {
        let service = CoreService::with_connection(Config::default(), open_memory().unwrap())
            .expect("service should initialize");
        let cfg = Config::default();
        let plugins = PluginRegistry::default();
        let parsed = ParsedQuery::parse(">swiftfind roadmap", true);
        let results = search_overlay_results(&service, &cfg, &plugins, &parsed, 10)
            .expect("search should succeed");
        assert!(results
            .iter()
            .any(|item| item.id.starts_with(ACTION_WEB_SEARCH_PREFIX)));
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
            SearchItem::new(
                "f1",
                "file",
                "Framer.lnk",
                "C:\\Users\\Admin\\Desktop\\Framer.lnk",
            ),
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
        assert!(snapshot
            .startup_index_line
            .as_deref()
            .unwrap_or_default()
            .contains("startup indexed_items=310"));
        assert!(snapshot
            .last_provider_line
            .as_deref()
            .unwrap_or_default()
            .contains("index_provider name=start-menu-apps"));
        assert!(snapshot
            .last_icon_cache_line
            .as_deref()
            .unwrap_or_default()
            .contains("overlay_icon_cache reason=cache_clear"));
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
