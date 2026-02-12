#[cfg(target_os = "windows")]
mod imp {
    use std::ffi::c_void;
    use std::sync::OnceLock;

    use windows_sys::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetWindowLongPtrW,
        GetWindowTextLengthW, GetWindowTextW, IsWindow, LoadCursorW, MessageBoxW, RegisterClassW,
        SendMessageW, SetWindowLongPtrW, SetWindowTextW, ShowWindow, TranslateMessage,
        CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, HMENU, IDC_ARROW,
        MB_ICONERROR, MB_OK, MSG, SW_SHOW, WM_CLOSE, WM_COMMAND, WM_CREATE, WM_NCCREATE,
        WM_NCDESTROY, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_EX_TOOLWINDOW, WS_OVERLAPPED, WS_SYSMENU,
        WS_TABSTOP, WS_VISIBLE,
    };

    const CLASS_NAME: &str = "SwiftFindSettingsWindowClass";
    const WINDOW_TITLE: &str = "SwiftFind Settings";

    const ID_EDIT_HOTKEY: usize = 2001;
    const ID_CHECK_STARTUP: usize = 2002;
    const ID_EDIT_MAX_RESULTS: usize = 2003;
    const ID_BUTTON_SAVE: usize = 2004;
    const ID_BUTTON_CANCEL: usize = 2005;

    const BUTTON_AUTOCHECKBOX: u32 = 0x00000003;
    const BUTTON_DEFPUSH: u32 = 0x00000001;
    const BUTTON_PUSH: u32 = 0x00000000;
    const WM_SETFONT: u32 = 0x0030;
    const BN_CLICKED: usize = 0;
    const BM_GETCHECK: u32 = 0x00F0;
    const BM_SETCHECK: u32 = 0x00F1;
    const BST_CHECKED: usize = 1;
    const MB_TASKMODAL: u32 = 0x00002000;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SettingsDialogResult {
        pub hotkey: String,
        pub max_results: u16,
        pub launch_at_startup: bool,
    }

    struct SettingsDialogState {
        hotkey_edit: HWND,
        startup_check: HWND,
        max_results_edit: HWND,
        result: Option<SettingsDialogResult>,
        closed: bool,
    }

    impl SettingsDialogState {
        fn new() -> Self {
            Self {
                hotkey_edit: std::ptr::null_mut(),
                startup_check: std::ptr::null_mut(),
                max_results_edit: std::ptr::null_mut(),
                result: None,
                closed: false,
            }
        }
    }

    pub fn open_settings_dialog(
        cfg: &crate::config::Config,
    ) -> Result<Option<SettingsDialogResult>, String> {
        let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
        let class_name = class_name_wide();

        let mut class: WNDCLASSW = unsafe { std::mem::zeroed() };
        class.style = CS_HREDRAW | CS_VREDRAW;
        class.lpfnWndProc = Some(settings_wnd_proc);
        class.hInstance = instance;
        class.hCursor = unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW) };
        class.hbrBackground = std::ptr::null_mut();
        class.lpszClassName = class_name.as_ptr();

        let atom = unsafe { RegisterClassW(&class) };
        if atom == 0 {
            let error = unsafe { GetLastError() };
            if error != 1410 {
                return Err(format!("RegisterClassW settings failed with error {error}"));
            }
        }

        let mut state = Box::new(SettingsDialogState::new());
        let state_ptr: *mut SettingsDialogState = &mut *state;

        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                class_name.as_ptr(),
                to_wide(WINDOW_TITLE).as_ptr(),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                460,
                280,
                std::ptr::null_mut(),
                0 as HMENU,
                instance,
                state_ptr as *mut c_void,
            )
        };
        if hwnd.is_null() {
            let error = unsafe { GetLastError() };
            return Err(format!(
                "CreateWindowExW settings failed with error {error}"
            ));
        }

        if let Some(s) = state_for(hwnd) {
            s.hotkey_edit = create_control(
                hwnd,
                "EDIT",
                &cfg.hotkey,
                WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                ID_EDIT_HOTKEY,
                20,
                52,
                420,
                28,
            );
            s.startup_check = create_control(
                hwnd,
                "BUTTON",
                "Launch at Windows startup",
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | BUTTON_AUTOCHECKBOX,
                ID_CHECK_STARTUP,
                20,
                95,
                300,
                24,
            );
            if cfg.launch_at_startup {
                unsafe {
                    SendMessageW(s.startup_check, BM_SETCHECK, BST_CHECKED, 0);
                }
            }
            s.max_results_edit = create_control(
                hwnd,
                "EDIT",
                &cfg.max_results.to_string(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                ID_EDIT_MAX_RESULTS,
                20,
                145,
                100,
                28,
            );
            create_control(
                hwnd,
                "BUTTON",
                "Save",
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | BUTTON_DEFPUSH,
                ID_BUTTON_SAVE,
                250,
                205,
                90,
                30,
            );
            create_control(
                hwnd,
                "BUTTON",
                "Cancel",
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | BUTTON_PUSH,
                ID_BUTTON_CANCEL,
                350,
                205,
                90,
                30,
            );
            create_control(
                hwnd,
                "STATIC",
                "Hotkey",
                WS_CHILD | WS_VISIBLE,
                0,
                20,
                32,
                420,
                18,
            );
            create_control(
                hwnd,
                "STATIC",
                "Recommended: Ctrl+Shift+Space, Ctrl+Alt+Space, Alt+Shift+Space, Ctrl+Shift+P",
                WS_CHILD | WS_VISIBLE,
                0,
                20,
                176,
                420,
                18,
            );
            create_control(
                hwnd,
                "STATIC",
                "Max results (5-100)",
                WS_CHILD | WS_VISIBLE,
                0,
                20,
                126,
                200,
                18,
            );
        }

        unsafe {
            ShowWindow(hwnd, SW_SHOW);
        }

        let mut msg: MSG = unsafe { std::mem::zeroed() };
        loop {
            if unsafe { IsWindow(hwnd) } == 0 {
                break;
            }
            let status = unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) };
            if status == -1 {
                return Err("Settings message loop failed.".to_string());
            }
            if status == 0 {
                break;
            }
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if state.closed {
                break;
            }
        }

        Ok(state.result.take())
    }

    extern "system" fn settings_wnd_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_NCCREATE => {
                let create = lparam as *const CREATESTRUCTW;
                if create.is_null() {
                    return 0;
                }
                let state_ptr = unsafe { (*create).lpCreateParams as *mut SettingsDialogState };
                unsafe {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
                }
                1
            }
            WM_CREATE => 0,
            WM_COMMAND => {
                let control_id = wparam & 0xffff;
                let notification = (wparam >> 16) & 0xffff;
                if notification == BN_CLICKED {
                    if control_id == ID_BUTTON_SAVE {
                        if let Some(state) = state_for(hwnd) {
                            match collect_settings_from_controls(state) {
                                Ok(result) => {
                                    state.result = Some(result);
                                    state.closed = true;
                                    unsafe {
                                        windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(
                                            hwnd,
                                        );
                                    }
                                }
                                Err(error) => {
                                    show_validation_error(hwnd, &error);
                                }
                            }
                        }
                        return 0;
                    }
                    if control_id == ID_BUTTON_CANCEL {
                        if let Some(state) = state_for(hwnd) {
                            state.closed = true;
                        }
                        unsafe {
                            windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
                        }
                        return 0;
                    }
                }
                unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
            }
            WM_CLOSE => {
                if let Some(state) = state_for(hwnd) {
                    state.closed = true;
                }
                unsafe {
                    windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
                }
                0
            }
            WM_NCDESTROY => {
                unsafe {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                }
                0
            }
            _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
        }
    }

    fn collect_settings_from_controls(
        state: &SettingsDialogState,
    ) -> Result<SettingsDialogResult, String> {
        let hotkey_raw = get_window_text(state.hotkey_edit);
        let hotkey = crate::settings::validate_hotkey(&hotkey_raw)?;

        let max_raw = get_window_text(state.max_results_edit);
        let max_results = max_raw
            .trim()
            .parse::<u16>()
            .map_err(|_| "Max results must be a number.".to_string())?;
        crate::settings::validate_max_results(max_results)?;

        let launch_at_startup =
            unsafe { SendMessageW(state.startup_check, BM_GETCHECK, 0, 0) as usize } == BST_CHECKED;

        Ok(SettingsDialogResult {
            hotkey,
            max_results,
            launch_at_startup,
        })
    }

    fn create_control(
        parent: HWND,
        class_name: &str,
        text: &str,
        style: u32,
        id: usize,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> HWND {
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide(class_name).as_ptr(),
                to_wide(text).as_ptr(),
                style,
                x,
                y,
                width,
                height,
                parent,
                id as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if !hwnd.is_null() {
            unsafe {
                SendMessageW(hwnd, WM_SETFONT, 0, 1);
                SetWindowTextW(hwnd, to_wide(text).as_ptr());
            }
        }
        hwnd
    }

    fn get_window_text(hwnd: HWND) -> String {
        let length = unsafe { GetWindowTextLengthW(hwnd) };
        if length <= 0 {
            return String::new();
        }
        let mut buffer = vec![0_u16; length as usize + 1];
        let copied = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
        String::from_utf16_lossy(&buffer[..copied as usize])
    }

    fn show_validation_error(hwnd: HWND, error: &str) {
        unsafe {
            MessageBoxW(
                hwnd,
                to_wide(error).as_ptr(),
                to_wide("Invalid Settings").as_ptr(),
                MB_OK | MB_ICONERROR | MB_TASKMODAL,
            );
        }
    }

    fn state_for(hwnd: HWND) -> Option<&'static mut SettingsDialogState> {
        let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsDialogState };
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { &mut *ptr })
        }
    }

    fn class_name_wide() -> &'static [u16] {
        static CLASS_NAME_WIDE: OnceLock<Vec<u16>> = OnceLock::new();
        CLASS_NAME_WIDE
            .get_or_init(|| to_wide(CLASS_NAME))
            .as_slice()
    }

    fn to_wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(target_os = "windows")]
pub use imp::{open_settings_dialog, SettingsDialogResult};

#[cfg(not(target_os = "windows"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsDialogResult {
    pub hotkey: String,
    pub max_results: u16,
    pub launch_at_startup: bool,
}

#[cfg(not(target_os = "windows"))]
pub fn open_settings_dialog(
    _cfg: &crate::config::Config,
) -> Result<Option<SettingsDialogResult>, String> {
    Err("settings UI is only supported on Windows".to_string())
}
