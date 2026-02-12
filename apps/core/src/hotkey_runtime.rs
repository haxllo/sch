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
            MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, VK_F1, VK_F10, VK_F11, VK_F12, VK_F2,
            VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_SPACE,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::RegisterHotKey;

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
        let vk = match key_upper.as_str() {
            "SPACE" => VK_SPACE,
            "F1" => VK_F1,
            "F2" => VK_F2,
            "F3" => VK_F3,
            "F4" => VK_F4,
            "F5" => VK_F5,
            "F6" => VK_F6,
            "F7" => VK_F7,
            "F8" => VK_F8,
            "F9" => VK_F9,
            "F10" => VK_F10,
            "F11" => VK_F11,
            "F12" => VK_F12,
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

        let ok = unsafe { RegisterHotKey(0, id, modifiers, vk) };
        if ok == 0 {
            return Err(HotkeyRuntimeError::RegistrationFailed(format!(
                "RegisterHotKey failed for '{hotkey}'"
            )));
        }

        self.registered_ids.push(id);
        Ok(HotkeyRegistration::Native(id))
    }

    fn unregister_all(&mut self) -> Result<(), HotkeyRuntimeError> {
        use windows_sys::Win32::UI::WindowsAndMessaging::UnregisterHotKey;

        for id in self.registered_ids.drain(..) {
            unsafe {
                UnregisterHotKey(0, id);
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
