use crate::config::{self, ConfigError};
use crate::core_service::{CoreService, ServiceError};
use crate::hotkey_runtime::HotkeyRuntimeError;
#[cfg(target_os = "windows")]
use crate::hotkey_runtime::{default_hotkey_registrar, run_message_loop, HotkeyRegistration};

#[derive(Debug)]
pub enum RuntimeError {
    Config(ConfigError),
    Service(ServiceError),
    Hotkey(HotkeyRuntimeError),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(error) => write!(f, "config error: {error}"),
            Self::Service(error) => write!(f, "service error: {error}"),
            Self::Hotkey(error) => write!(f, "hotkey runtime error: {error:?}"),
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
        let mut registrar = default_hotkey_registrar();
        let registration = registrar.register_hotkey(&config.hotkey)?;
        log_registration(&registration);
        println!("[swiftfind-core] event loop running (WM_HOTKEY)");
        run_message_loop(|_| {
            println!("[swiftfind-core] hotkey_event received");
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
