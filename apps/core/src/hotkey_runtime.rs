use crate::hotkey::parse_hotkey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyRegistration {
    Native(i32),
    Noop(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyRuntimeError {
    InvalidHotkey(String),
    RegistrationFailed(String),
    EventLoopFailed(String),
    UnsupportedPlatform,
}

pub trait HotkeyRegistrar: Send {
    fn register_hotkey(&mut self, hotkey: &str) -> Result<HotkeyRegistration, HotkeyRuntimeError>;
    fn unregister_all(&mut self) -> Result<(), HotkeyRuntimeError>;
}

#[derive(Default)]
pub struct MockHotkeyRegistrar {
    registrations: Vec<String>,
}

impl MockHotkeyRegistrar {
    pub fn registrations(&self) -> &[String] {
        &self.registrations
    }
}

impl HotkeyRegistrar for MockHotkeyRegistrar {
    fn register_hotkey(&mut self, hotkey: &str) -> Result<HotkeyRegistration, HotkeyRuntimeError> {
        parse_hotkey(hotkey).map_err(HotkeyRuntimeError::InvalidHotkey)?;
        self.registrations.push(hotkey.to_string());
        Ok(HotkeyRegistration::Noop(hotkey.to_string()))
    }

    fn unregister_all(&mut self) -> Result<(), HotkeyRuntimeError> {
        self.registrations.clear();
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
#[derive(Default)]
pub struct NoopHotkeyRegistrar {
    registrations: Vec<String>,
}

#[cfg(not(target_os = "windows"))]
impl NoopHotkeyRegistrar {
    pub fn registrations(&self) -> &[String] {
        &self.registrations
    }
}

#[cfg(not(target_os = "windows"))]
impl HotkeyRegistrar for NoopHotkeyRegistrar {
    fn register_hotkey(&mut self, hotkey: &str) -> Result<HotkeyRegistration, HotkeyRuntimeError> {
        parse_hotkey(hotkey).map_err(HotkeyRuntimeError::InvalidHotkey)?;
        self.registrations.push(hotkey.to_string());
        Ok(HotkeyRegistration::Noop(hotkey.to_string()))
    }

    fn unregister_all(&mut self) -> Result<(), HotkeyRuntimeError> {
        self.registrations.clear();
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub struct WindowsHotkeyRegistrar {
    next_id: i32,
    registered_ids: Vec<i32>,
}

#[cfg(target_os = "windows")]
impl Default for WindowsHotkeyRegistrar {
    fn default() -> Self {
        Self {
            next_id: 1,
            registered_ids: Vec::new(),
        }
    }
}

#[cfg(target_os = "windows")]
impl HotkeyRegistrar for WindowsHotkeyRegistrar {
    fn register_hotkey(&mut self, hotkey: &str) -> Result<HotkeyRegistration, HotkeyRuntimeError> {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            RegisterHotKey, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, VK_F1, VK_F10, VK_F11,
            VK_F12, VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_SPACE,
        };

        let parsed = parse_hotkey(hotkey).map_err(HotkeyRuntimeError::InvalidHotkey)?;

        let mut modifiers = 0_u32;
        for modifier in &parsed.modifiers {
            match modifier.to_ascii_lowercase().as_str() {
                "alt" => modifiers |= MOD_ALT,
                "ctrl" | "control" => modifiers |= MOD_CONTROL,
                "shift" => modifiers |= MOD_SHIFT,
                "win" | "meta" | "super" => modifiers |= MOD_WIN,
                _ => {
                    return Err(HotkeyRuntimeError::InvalidHotkey(format!(
                        "unsupported modifier: {modifier}"
                    )))
                }
            }
        }

        let key_upper = parsed.key.to_ascii_uppercase();
        let vk: u32 = match key_upper.as_str() {
            "SPACE" => VK_SPACE as u32,
            "F1" => VK_F1 as u32,
            "F2" => VK_F2 as u32,
            "F3" => VK_F3 as u32,
            "F4" => VK_F4 as u32,
            "F5" => VK_F5 as u32,
            "F6" => VK_F6 as u32,
            "F7" => VK_F7 as u32,
            "F8" => VK_F8 as u32,
            "F9" => VK_F9 as u32,
            "F10" => VK_F10 as u32,
            "F11" => VK_F11 as u32,
            "F12" => VK_F12 as u32,
            _ if key_upper.len() == 1 => key_upper.as_bytes()[0] as u32,
            _ => {
                return Err(HotkeyRuntimeError::InvalidHotkey(format!(
                    "unsupported key: {}",
                    parsed.key
                )))
            }
        };

        let id = self.next_id;
        self.next_id += 1;

        let ok = unsafe { RegisterHotKey(std::ptr::null_mut(), id, modifiers, vk) };
        if ok == 0 {
            return Err(HotkeyRuntimeError::RegistrationFailed(format!(
                "RegisterHotKey failed for '{hotkey}'"
            )));
        }

        self.registered_ids.push(id);
        Ok(HotkeyRegistration::Native(id))
    }

    fn unregister_all(&mut self) -> Result<(), HotkeyRuntimeError> {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::UnregisterHotKey;

        for id in self.registered_ids.drain(..) {
            unsafe {
                UnregisterHotKey(std::ptr::null_mut(), id);
            }
        }
        Ok(())
    }
}

pub fn default_hotkey_registrar() -> Box<dyn HotkeyRegistrar> {
    #[cfg(target_os = "windows")]
    {
        Box::new(WindowsHotkeyRegistrar::default())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Box::new(NoopHotkeyRegistrar::default())
    }
}

#[cfg(target_os = "windows")]
pub fn run_message_loop<F>(mut on_hotkey: F) -> Result<(), HotkeyRuntimeError>
where
    F: FnMut(i32),
{
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, TranslateMessage, MSG, WM_HOTKEY,
    };

    let mut msg: MSG = unsafe { std::mem::zeroed() };
    loop {
        let status = unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) };
        if status == -1 {
            return Err(HotkeyRuntimeError::EventLoopFailed(
                "GetMessageW returned -1".to_string(),
            ));
        }

        if status == 0 {
            return Ok(());
        }

        if msg.message == WM_HOTKEY {
            on_hotkey(msg.wParam as i32);
        }

        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn run_message_loop<F>(_on_hotkey: F) -> Result<(), HotkeyRuntimeError>
where
    F: FnMut(i32),
{
    Err(HotkeyRuntimeError::UnsupportedPlatform)
}
