#[cfg(target_os = "windows")]
mod imp {
    use std::collections::{HashMap, HashSet};
    use std::ffi::c_void;
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;
    use std::time::Instant;

    use windows_sys::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL,
    };
    use windows_sys::Win32::Graphics::Gdi::{
        AddFontResourceExW, BeginPaint, CreateFontW, CreateRoundRectRgn, CreateSolidBrush, DeleteObject, DrawTextW,
        EndPaint, FillRect, FillRgn, FrameRgn, GetDC, GetTextExtentPoint32W, GetTextMetricsW, HDC, InvalidateRect, PAINTSTRUCT, ReleaseDC, ScreenToClient, SelectObject, SetBkColor,
        SetBkMode, SetTextColor, SetWindowRgn, DEFAULT_CHARSET, DEFAULT_QUALITY, DT_CENTER,
        DT_EDITCONTROL, DT_END_ELLIPSIS, DT_LEFT, DT_SINGLELINE, DT_VCENTER, FF_DONTCARE, FW_MEDIUM, FW_SEMIBOLD,
        FR_PRIVATE, OPAQUE, OUT_DEFAULT_PRECIS, TRANSPARENT, TextOutW,
        TEXTMETRICW,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Shell::{
        SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_SMALLICON, SHGFI_USEFILEATTRIBUTES,
    };
    use windows_sys::Win32::UI::Controls::{DRAWITEMSTRUCT, EM_SETSEL, MEASUREITEMSTRUCT, ODS_SELECTED};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SetFocus, VK_DOWN, VK_ESCAPE, VK_RETURN, VK_UP,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, CreateWindowExW, DefWindowProcW, DispatchMessageW,
        DestroyIcon, DI_NORMAL, DrawIconEx,
        GetClientRect, GetCursorPos, GetForegroundWindow, GetMessageW, GetParent, GetSystemMetrics,
        GetWindowLongPtrW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
        HideCaret,
        IsChild, LB_ADDSTRING, LB_GETCOUNT, LB_GETCURSEL, LB_GETITEMRECT, LB_GETTOPINDEX,
        LB_ITEMFROMPOINT, LB_RESETCONTENT, LB_SETCURSEL,
        LB_SETTOPINDEX, LoadCursorW, SetCursor,
        MoveWindow, PostMessageW, PostQuitMessage, RegisterClassW, SendMessageW,
        SetForegroundWindow, SetLayeredWindowAttributes, SetTimer, SetWindowLongPtrW, SetWindowPos,
        SetWindowTextW, ShowWindow, TranslateMessage, CREATESTRUCTW, CS_DROPSHADOW,
        CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, EN_CHANGE, ES_AUTOHSCROLL, ES_MULTILINE, GWLP_USERDATA,
        GWLP_WNDPROC, HMENU, HWND_TOP, IDC_ARROW, IDC_HAND, KillTimer, LBN_DBLCLK, LBS_HASSTRINGS,
        LBS_NOINTEGRALHEIGHT, LBS_NOTIFY, LBS_OWNERDRAWFIXED, LWA_ALPHA, MSG, SM_CXSCREEN,
        SM_CYSCREEN, SW_HIDE, SW_SHOW, SWP_NOACTIVATE, WM_APP, WM_CLOSE, WM_COMMAND, WM_CREATE,
        WM_CTLCOLORLISTBOX, WM_CTLCOLOREDIT, WM_CTLCOLORSTATIC, WM_DESTROY, WM_DRAWITEM,
        WM_HOTKEY, WM_KEYDOWN, WM_MEASUREITEM, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCCREATE,
        WM_NCDESTROY, WM_PAINT, WM_SETFONT, WM_SETFOCUS, WM_SIZE, WM_TIMER, WM_LBUTTONUP, WM_ACTIVATE,
        WNDCLASSW, WS_CHILD,
        WS_CLIPCHILDREN, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_POPUP, WS_TABSTOP,
        WS_VISIBLE,
    };

    const CLASS_NAME: &str = "SwiftFindOverlayWindowClass";
    const WINDOW_TITLE: &str = "SwiftFind Launcher";
    const INPUT_CLASS: &str = "EDIT";
    const LIST_CLASS: &str = "LISTBOX";
    const STATUS_CLASS: &str = "STATIC";

    // Overlay layout tokens.
    const WINDOW_WIDTH: i32 = 576;
    const COMPACT_HEIGHT: i32 = 90;
    const PANEL_RADIUS: i32 = 20;
    const PANEL_MARGIN_X: i32 = 14;
    const PANEL_MARGIN_BOTTOM: i32 = 8;
    const INPUT_HEIGHT: i32 = 36;
    const INPUT_TOP: i32 = (COMPACT_HEIGHT - INPUT_HEIGHT) / 2;
    const INPUT_TO_LIST_GAP: i32 = 2;
    const STATUS_HEIGHT: i32 = 18;
    const ROW_HEIGHT: i32 = 56;
    const LIST_RADIUS: i32 = 16;
    const MAX_VISIBLE_ROWS: usize = 5;
    const ROW_INSET_X: i32 = 10;
    const ROW_ICON_SIZE: i32 = 22;
    const ROW_ICON_DRAW_SIZE: i32 = 18;
    const ROW_ICON_GAP: i32 = 10;
    const ROW_TEXT_TOP_PAD: i32 = 7;
    const ROW_TEXT_BOTTOM_PAD: i32 = 6;
    const ROW_VERTICAL_INSET: i32 = 1;
    const ROW_ACTIVE_RADIUS: i32 = 10;

    const CONTROL_ID_INPUT: usize = 1001;
    const CONTROL_ID_LIST: usize = 1002;
    const CONTROL_ID_STATUS: usize = 1003;
    const CONTROL_ID_HELP: usize = 1004;
    const CONTROL_ID_HELP_TIP: usize = 1005;
    const STATIC_NOTIFY_STYLE: u32 = 0x0100; // SS_NOTIFY
    const STATIC_RIGHT_STYLE: u32 = 0x00000002; // SS_RIGHT

    const SWIFTFIND_WM_ESCAPE: u32 = WM_APP + 1;
    const SWIFTFIND_WM_QUERY_CHANGED: u32 = WM_APP + 2;
    const SWIFTFIND_WM_MOVE_UP: u32 = WM_APP + 3;
    const SWIFTFIND_WM_MOVE_DOWN: u32 = WM_APP + 4;
    const SWIFTFIND_WM_SUBMIT: u32 = WM_APP + 5;
    const EM_GETRECT: u32 = 0x00B2;
    const EM_SETRECTNP: u32 = 0x00B4;

    const TIMER_SCROLL_ANIM: usize = 0xBEF0;
    const TIMER_WINDOW_ANIM: usize = 0xBEF1;
    const TIMER_ROW_ANIM: usize = 0xBEF2;

    const OVERLAY_ANIM_MS: u32 = 150;
    const OVERLAY_HIDE_ANIM_MS: u32 = 115;
    const RESULTS_ANIM_MS: u32 = 150;
    const SCROLL_ANIM_MS: u64 = 120;
    const ANIM_FRAME_MS: u64 = 8;
    const WHEEL_LINES_PER_NOTCH: i32 = 3;
    const ROW_ANIM_MS: u64 = 130;
    const ROW_STAGGER_MS: u64 = 16;

    // Typography tokens.
    const FONT_INPUT_HEIGHT: i32 = -19;
    const FONT_TITLE_HEIGHT: i32 = -15;
    const FONT_META_HEIGHT: i32 = -13;
    const FONT_STATUS_HEIGHT: i32 = -13;
    const FONT_HELP_TIP_HEIGHT: i32 = -11;
    const INPUT_TEXT_SHIFT_X: i32 = 10;
    const INPUT_TEXT_SHIFT_Y: i32 = 0;
    const INPUT_TEXT_LINE_HEIGHT_FALLBACK: i32 = 20;
    const INPUT_TEXT_LEFT_INSET: i32 = 19;
    const INPUT_TEXT_RIGHT_INSET: i32 = 10;
    const HELP_ICON_SIZE: i32 = 14;
    const HELP_ICON_RIGHT_INSET: i32 = 12;
    const HELP_ICON_GAP_FROM_INPUT: i32 = 8;

    // Visual tokens.
    const COLOR_PANEL_BG: u32 = 0x00101010;
    const COLOR_PANEL_BORDER: u32 = 0x002A2A2A;
    const COLOR_INPUT_BG: u32 = COLOR_PANEL_BG;
    const COLOR_RESULTS_BG: u32 = 0x00101010;
    const COLOR_TEXT_PRIMARY: u32 = 0x00F4F4F4;
    const COLOR_TEXT_SECONDARY: u32 = 0x00B8B8B8;
    const COLOR_TEXT_ERROR: u32 = 0x00E8E8E8;
    const COLOR_TEXT_HIGHLIGHT: u32 = 0x00FFFFFF;
    const COLOR_TEXT_HINT: u32 = 0x009B9B9B;
    const COLOR_SELECTION: u32 = 0x00262626;
    const COLOR_SELECTION_BORDER: u32 = 0x00383838;
    const COLOR_ROW_HOVER: u32 = 0x001A1A1A;
    const COLOR_ROW_SEPARATOR: u32 = 0x00161616;
    const COLOR_SELECTION_ACCENT: u32 = 0x00343434;
    const COLOR_ICON_BG: u32 = 0x001D1D1D;
    const COLOR_ICON_TEXT: u32 = 0x00F0F0F0;
    const COLOR_HELP_ICON: u32 = COLOR_TEXT_SECONDARY;
    const COLOR_HELP_ICON_HOVER: u32 = COLOR_TEXT_PRIMARY;
    const COLOR_HELP_TIP_BG: u32 = COLOR_PANEL_BG;
    const COLOR_HELP_TIP_TEXT: u32 = COLOR_TEXT_SECONDARY;
    const HELP_TIP_WIDTH: i32 = 196;
    const HELP_TIP_HEIGHT: i32 = 26;
    const HELP_TIP_RADIUS: i32 = 10;
    const HELP_TIP_TEXT_PAD_X: i32 = 10;
    const DEFAULT_FONT_FAMILY: &str = "Segoe UI Variable Text";
    const GEIST_FONT_FAMILY: &str = "Geist";
    const HOTKEY_HELP_TEXT_FALLBACK: &str = "Click to change hotkey";
    const FOOTER_HINT_TEXT: &str = "Enter open | Up/Down move | Esc close";

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum OverlayEvent {
        Hotkey(i32),
        QueryChanged(String),
        MoveSelection(i32),
        Submit,
        Escape,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OverlayRow {
        pub kind: String,
        pub title: String,
        pub path: String,
        pub icon_path: String,
    }

    pub struct NativeOverlayShell {
        hwnd: HWND,
    }

    struct OverlayShellState {
        edit_hwnd: HWND,
        list_hwnd: HWND,
        status_hwnd: HWND,
        help_hwnd: HWND,
        help_tip_hwnd: HWND,

        edit_prev_proc: isize,
        list_prev_proc: isize,
        help_prev_proc: isize,
        help_tip_prev_proc: isize,

        input_font: isize,
        title_font: isize,
        meta_font: isize,
        status_font: isize,
        help_tip_font: isize,

        panel_brush: isize,
        border_brush: isize,
        input_brush: isize,
        results_brush: isize,
        selection_brush: isize,
        selection_border_brush: isize,
        row_hover_brush: isize,
        row_separator_brush: isize,
        selection_accent_brush: isize,
        icon_brush: isize,
        help_tip_brush: isize,
        help_tip_border_brush: isize,

        status_is_error: bool,
        help_hovered: bool,
        help_tip_visible: bool,
        results_visible: bool,
        help_config_path: String,
        active_query: String,

        hover_index: i32,

        scroll_from_top: i32,
        scroll_to_top: i32,
        scroll_anim_start: Option<Instant>,
        row_anim_start: Option<Instant>,
        row_anim_exiting: bool,

        window_anim: Option<WindowAnimation>,
        rows: Vec<OverlayRow>,
        icon_cache: HashMap<String, isize>,
    }

    impl Default for OverlayShellState {
        fn default() -> Self {
            Self {
                edit_hwnd: std::ptr::null_mut(),
                list_hwnd: std::ptr::null_mut(),
                status_hwnd: std::ptr::null_mut(),
                help_hwnd: std::ptr::null_mut(),
                help_tip_hwnd: std::ptr::null_mut(),
                edit_prev_proc: 0,
                list_prev_proc: 0,
                help_prev_proc: 0,
                help_tip_prev_proc: 0,
                input_font: 0,
                title_font: 0,
                meta_font: 0,
                status_font: 0,
                help_tip_font: 0,
                panel_brush: 0,
                border_brush: 0,
                input_brush: 0,
                results_brush: 0,
                selection_brush: 0,
                selection_border_brush: 0,
                row_hover_brush: 0,
                row_separator_brush: 0,
                selection_accent_brush: 0,
                icon_brush: 0,
                help_tip_brush: 0,
                help_tip_border_brush: 0,
                status_is_error: false,
                help_hovered: false,
                help_tip_visible: false,
                results_visible: false,
                help_config_path: String::new(),
                active_query: String::new(),
                hover_index: -1,
                scroll_from_top: 0,
                scroll_to_top: 0,
                scroll_anim_start: None,
                row_anim_start: None,
                row_anim_exiting: false,
                window_anim: None,
                rows: Vec::new(),
                icon_cache: HashMap::new(),
            }
        }
    }

    struct WindowAnimation {
        start: Instant,
        duration_ms: u32,
        from_left: i32,
        from_top: i32,
        from_width: i32,
        from_height: i32,
        to_left: i32,
        to_top: i32,
        to_width: i32,
        to_height: i32,
        from_alpha: u8,
        to_alpha: u8,
        hide_on_complete: bool,
    }

    impl NativeOverlayShell {
        pub fn create() -> Result<Self, String> {
            let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
            let class_name = class_name_wide();

            let mut class: WNDCLASSW = unsafe { std::mem::zeroed() };
            class.style = CS_HREDRAW | CS_VREDRAW | CS_DROPSHADOW;
            class.lpfnWndProc = Some(overlay_wnd_proc);
            class.hInstance = instance;
            class.hCursor = unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW) };
            class.hbrBackground = std::ptr::null_mut();
            class.lpszClassName = class_name.as_ptr();

            let atom = unsafe { RegisterClassW(&class) };
            if atom == 0 {
                let error = unsafe { GetLastError() };
                if error != 1410 {
                    return Err(format!("RegisterClassW failed with error {error}"));
                }
            }

            let state = Box::new(OverlayShellState::default());
            let state_ptr = Box::into_raw(state);

            let hwnd = unsafe {
                CreateWindowExW(
                    WS_EX_TOOLWINDOW | WS_EX_LAYERED,
                    class_name.as_ptr(),
                    to_wide(WINDOW_TITLE).as_ptr(),
                    WS_POPUP | WS_CLIPCHILDREN,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    WINDOW_WIDTH,
                    COMPACT_HEIGHT,
                    std::ptr::null_mut(),
                    0 as HMENU,
                    instance,
                    state_ptr as *mut c_void,
                )
            };

            if hwnd.is_null() {
                unsafe {
                    let _ = Box::from_raw(state_ptr);
                }
                let error = unsafe { GetLastError() };
                return Err(format!("CreateWindowExW failed with error {error}"));
            }

            let shell = Self { hwnd };
            shell.center_window();
            shell.apply_rounded_corners();
            shell.hide_immediate();
            Ok(shell)
        }

        pub fn is_visible(&self) -> bool {
            unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(self.hwnd) != 0 }
        }

        pub fn has_focus(&self) -> bool {
            let fg = unsafe { GetForegroundWindow() };
            if fg == self.hwnd {
                return true;
            }
            unsafe { IsChild(self.hwnd, fg) != 0 }
        }

        pub fn show_and_focus(&self) {
            self.center_window();
            self.ensure_compact_state();
            self.animate_show();
            unsafe {
                SetForegroundWindow(self.hwnd);
            }
            self.focus_input_and_select_all();
        }

        pub fn focus_input_and_select_all(&self) {
            if let Some(state) = state_for(self.hwnd) {
                unsafe {
                    SetFocus(state.edit_hwnd);
                    SendMessageW(state.edit_hwnd, EM_SETSEL, 0, -1);
                }
                hide_input_caret(state.edit_hwnd);
            }
        }

        pub fn hide(&self) {
            self.animate_hide();
        }

        pub fn hide_now(&self) {
            hide_overlay_immediate(self.hwnd);
        }

        pub fn query_text(&self) -> String {
            let Some(state) = state_for(self.hwnd) else {
                return String::new();
            };

            let length = unsafe { GetWindowTextLengthW(state.edit_hwnd) };
            if length <= 0 {
                return String::new();
            }

            let mut buffer = vec![0_u16; (length as usize) + 1];
            let copied = unsafe {
                GetWindowTextW(state.edit_hwnd, buffer.as_mut_ptr(), buffer.len() as i32)
            };
            String::from_utf16_lossy(&buffer[..(copied as usize)])
        }

        pub fn set_status_text(&self, message: &str) {
            if let Some(state) = state_for(self.hwnd) {
                let trimmed = message.trim();
                let status_text = if trimmed.is_empty() {
                    FOOTER_HINT_TEXT
                } else {
                    trimmed
                };
                state.status_is_error = !trimmed.is_empty() && trimmed.to_ascii_lowercase().contains("error");
                state.help_tip_visible = false;
                unsafe {
                    ShowWindow(state.help_tip_hwnd, SW_HIDE);
                }
                if trimmed.is_empty() {
                    state.help_hovered = false;
                }
                let wide = to_wide(status_text);
                unsafe {
                    SetWindowTextW(state.status_hwnd, wide.as_ptr());
                    InvalidateRect(state.status_hwnd, std::ptr::null(), 1);
                }
                layout_children(self.hwnd, state);
                unsafe {
                    InvalidateRect(self.hwnd, std::ptr::null(), 1);
                }
            }
        }

        pub fn set_hotkey_hint(&self, _hotkey: &str) {
            self.set_status_text("");
        }

        pub fn set_help_config_path(&self, path: &str) {
            if let Some(state) = state_for(self.hwnd) {
                state.help_config_path = path.to_string();
            }
        }

        pub fn clear_query_text(&self) {
            if let Some(state) = state_for(self.hwnd) {
                unsafe {
                    SetWindowTextW(state.edit_hwnd, to_wide("").as_ptr());
                }
            }
        }

        pub fn set_results(&self, rows: &[OverlayRow], selected_index: usize) {
            if let Some(state) = state_for(self.hwnd) {
                state.active_query = self.query_text().trim().to_string();
                state.hover_index = -1;

                if rows.is_empty() {
                    if state.results_visible && !state.rows.is_empty() {
                        state.row_anim_exiting = true;
                        state.row_anim_start = Some(Instant::now());
                        unsafe {
                            SetTimer(self.hwnd, TIMER_ROW_ANIM, ANIM_FRAME_MS as u32, None);
                        }
                        self.collapse_results();
                        return;
                    }

                    state.rows.clear();
                    unsafe {
                        SendMessageW(state.list_hwnd, LB_RESETCONTENT, 0, 0);
                        SendMessageW(state.list_hwnd, LB_SETTOPINDEX, 0, 0);
                    }

                    self.collapse_results();
                    state.scroll_anim_start = None;
                    state.scroll_from_top = 0;
                    state.scroll_to_top = 0;
                    state.hover_index = -1;
                    state.row_anim_start = None;
                    state.row_anim_exiting = false;
                    unsafe {
                        KillTimer(self.hwnd, TIMER_ROW_ANIM);
                    }
                    return;
                }

                state.row_anim_exiting = false;
                state.row_anim_start = Some(Instant::now());
                unsafe {
                    SetTimer(self.hwnd, TIMER_ROW_ANIM, ANIM_FRAME_MS as u32, None);
                }

                state.rows.clear();
                state.rows.extend_from_slice(rows);
                unsafe {
                    SendMessageW(state.list_hwnd, LB_RESETCONTENT, 0, 0);
                    SendMessageW(state.list_hwnd, LB_SETTOPINDEX, 0, 0);
                }

                for row in rows {
                    // Keep listbox item text lightweight; owner-draw uses state.rows.
                    let wide = to_wide(&row.title);
                    unsafe {
                        SendMessageW(state.list_hwnd, LB_ADDSTRING, 0, wide.as_ptr() as LPARAM);
                    }
                }

                self.expand_results(rows.len());
                self.set_selected_index_internal(selected_index, false);
            }
        }

        pub fn set_selected_index(&self, selected_index: usize) {
            self.set_selected_index_internal(selected_index, true);
        }

        fn set_selected_index_internal(&self, selected_index: usize, animate_scroll: bool) {
            let Some(state) = state_for(self.hwnd) else {
                return;
            };

            let count = unsafe { SendMessageW(state.list_hwnd, LB_GETCOUNT, 0, 0) };
            if count <= 0 {
                return;
            }

            let clamped = selected_index.min((count as usize).saturating_sub(1));
            unsafe {
                SendMessageW(state.list_hwnd, LB_SETCURSEL, clamped, 0);
            }
            if animate_scroll {
                begin_scroll_animation(self.hwnd, state, clamped as i32, count as i32);
            } else {
                let current_top =
                    unsafe { SendMessageW(state.list_hwnd, LB_GETTOPINDEX, 0, 0) as i32 };
                let target_top = target_top_index_for_selection(
                    state.list_hwnd,
                    clamped as i32,
                    count as i32,
                    current_top,
                );
                state.scroll_anim_start = None;
                state.scroll_from_top = target_top;
                state.scroll_to_top = target_top;
                unsafe {
                    KillTimer(self.hwnd, TIMER_SCROLL_ANIM);
                    SendMessageW(state.list_hwnd, LB_SETTOPINDEX, target_top as usize, 0);
                }
            }
            unsafe {
                InvalidateRect(state.list_hwnd, std::ptr::null(), 0);
            }
        }

        pub fn selected_index(&self) -> Option<usize> {
            let state = state_for(self.hwnd)?;
            let count = unsafe { SendMessageW(state.list_hwnd, LB_GETCOUNT, 0, 0) as i32 };
            if state.hover_index >= 0 && state.hover_index < count {
                return Some(state.hover_index as usize);
            }
            let index = unsafe { SendMessageW(state.list_hwnd, LB_GETCURSEL, 0, 0) };
            if index < 0 {
                None
            } else {
                Some(index as usize)
            }
        }

        pub fn run_message_loop_with_events<F>(&self, mut on_event: F) -> Result<(), String>
        where
            F: FnMut(OverlayEvent),
        {
            let mut msg: MSG = unsafe { std::mem::zeroed() };
            loop {
                let status = unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) };
                if status == -1 {
                    let err = unsafe { GetLastError() };
                    return Err(format!("GetMessageW failed with error {err}"));
                }
                if status == 0 {
                    return Ok(());
                }

                match msg.message {
                    WM_HOTKEY => on_event(OverlayEvent::Hotkey(msg.wParam as i32)),
                    SWIFTFIND_WM_QUERY_CHANGED => on_event(OverlayEvent::QueryChanged(self.query_text())),
                    SWIFTFIND_WM_MOVE_UP => on_event(OverlayEvent::MoveSelection(-1)),
                    SWIFTFIND_WM_MOVE_DOWN => on_event(OverlayEvent::MoveSelection(1)),
                    SWIFTFIND_WM_SUBMIT => on_event(OverlayEvent::Submit),
                    SWIFTFIND_WM_ESCAPE => on_event(OverlayEvent::Escape),
                    _ => {}
                }

                unsafe {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }

        fn center_window(&self) {
            let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
            let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
            let x = (screen_width - WINDOW_WIDTH).max(0) / 2;
            let y = (screen_height - COMPACT_HEIGHT).max(0) / 5;

            unsafe {
                SetWindowPos(
                    self.hwnd,
                    HWND_TOP,
                    x,
                    y,
                    WINDOW_WIDTH,
                    COMPACT_HEIGHT,
                    SWP_NOACTIVATE,
                );
            }
        }

        fn apply_rounded_corners(&self) {
            apply_rounded_corners_hwnd(self.hwnd);
        }

        fn hide_immediate(&self) {
            unsafe {
                SetLayeredWindowAttributes(self.hwnd, 0, 255, LWA_ALPHA);
                ShowWindow(self.hwnd, SW_HIDE);
            }
        }

        fn ensure_compact_state(&self) {
            self.animate_results_height(COMPACT_HEIGHT, 0);
            if let Some(state) = state_for(self.hwnd) {
                state.results_visible = false;
                state.scroll_anim_start = None;
                state.scroll_from_top = 0;
                state.scroll_to_top = 0;
                state.hover_index = -1;
                unsafe {
                    ShowWindow(state.list_hwnd, SW_HIDE);
                    KillTimer(self.hwnd, TIMER_SCROLL_ANIM);
                    KillTimer(self.hwnd, TIMER_ROW_ANIM);
                    SendMessageW(state.list_hwnd, LB_SETTOPINDEX, 0, 0);
                    SendMessageW(state.list_hwnd, LB_RESETCONTENT, 0, 0);
                }
                state.row_anim_start = None;
                state.row_anim_exiting = false;
                state.rows.clear();
            }
        }

        fn expand_results(&self, result_count: usize) {
            let rows = result_count.min(MAX_VISIBLE_ROWS) as i32;
            let list_top = COMPACT_HEIGHT + INPUT_TO_LIST_GAP;
            let target_height = list_top + rows * ROW_HEIGHT + PANEL_MARGIN_X;

            if let Some(state) = state_for(self.hwnd) {
                state.results_visible = true;
                unsafe {
                    ShowWindow(state.list_hwnd, SW_SHOW);
                }
            }

            self.animate_results_height(target_height, RESULTS_ANIM_MS);
        }

        fn collapse_results(&self) {
            self.animate_results_height(COMPACT_HEIGHT, RESULTS_ANIM_MS);
            if let Some(state) = state_for(self.hwnd) {
                state.results_visible = false;
            }
        }

        fn animate_results_height(&self, target_height: i32, duration_ms: u32) {
            let mut rect: RECT = unsafe { std::mem::zeroed() };
            unsafe {
                GetWindowRect(self.hwnd, &mut rect);
            }
            let current_height = rect.bottom - rect.top;

            if current_height == target_height {
                return;
            }

            if duration_ms == 0 {
                apply_window_state(self.hwnd, rect.left, rect.top, rect.right - rect.left, target_height, 255);
                return;
            }

            start_window_animation(
                self.hwnd,
                rect.left,
                rect.top,
                rect.right - rect.left,
                current_height,
                rect.left,
                rect.top,
                rect.right - rect.left,
                target_height,
                255,
                255,
                duration_ms,
                false,
            );
        }

        fn animate_show(&self) {
            if self.is_visible() {
                unsafe {
                    ShowWindow(self.hwnd, SW_SHOW);
                }
                return;
            }

            let mut rect: RECT = unsafe { std::mem::zeroed() };
            unsafe {
                GetWindowRect(self.hwnd, &mut rect);
            }
            let final_left = rect.left;
            let final_top = rect.top;
            let final_width = rect.right - rect.left;
            let final_height = rect.bottom - rect.top;

            let start_width = ((final_width as f32) * 0.96_f32) as i32;
            let start_height = ((final_height as f32) * 0.96_f32) as i32;
            let start_left = final_left + (final_width - start_width) / 2;
            let start_top = final_top + (final_height - start_height) / 2;

            apply_window_state(self.hwnd, start_left, start_top, start_width, start_height, 0);
            unsafe {
                ShowWindow(self.hwnd, SW_SHOW);
            }
            start_window_animation(
                self.hwnd,
                start_left,
                start_top,
                start_width,
                start_height,
                final_left,
                final_top,
                final_width,
                final_height,
                0,
                255,
                OVERLAY_ANIM_MS,
                false,
            );
        }

        fn animate_hide(&self) {
            if !self.is_visible() {
                return;
            }

            let mut rect: RECT = unsafe { std::mem::zeroed() };
            unsafe {
                GetWindowRect(self.hwnd, &mut rect);
            }
            let final_left = rect.left;
            let final_top = rect.top;
            let final_width = rect.right - rect.left;
            let final_height = rect.bottom - rect.top;

            let end_width = ((final_width as f32) * 0.96_f32) as i32;
            let end_height = ((final_height as f32) * 0.96_f32) as i32;
            let end_left = final_left + (final_width - end_width) / 2;
            let end_top = final_top + (final_height - end_height) / 2;
            start_window_animation(
                self.hwnd,
                final_left,
                final_top,
                final_width,
                final_height,
                end_left,
                end_top,
                end_width,
                end_height,
                255,
                0,
                OVERLAY_HIDE_ANIM_MS,
                true,
            );
        }
    }

    extern "system" fn overlay_wnd_proc(
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
                let state_ptr = unsafe { (*create).lpCreateParams as *mut OverlayShellState };
                unsafe {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
                }
                1
            }
            WM_CREATE => {
                if let Some(state) = state_for(hwnd) {
                    state.panel_brush = unsafe { CreateSolidBrush(COLOR_PANEL_BG) } as isize;
                    state.border_brush = unsafe { CreateSolidBrush(COLOR_PANEL_BORDER) } as isize;
                    state.input_brush = unsafe { CreateSolidBrush(COLOR_INPUT_BG) } as isize;
                    state.results_brush = unsafe { CreateSolidBrush(COLOR_RESULTS_BG) } as isize;
                    state.selection_brush = unsafe { CreateSolidBrush(COLOR_SELECTION) } as isize;
                    state.selection_border_brush =
                        unsafe { CreateSolidBrush(COLOR_SELECTION_BORDER) } as isize;
                    state.row_hover_brush = unsafe { CreateSolidBrush(COLOR_ROW_HOVER) } as isize;
                    state.row_separator_brush =
                        unsafe { CreateSolidBrush(COLOR_ROW_SEPARATOR) } as isize;
                    state.selection_accent_brush =
                        unsafe { CreateSolidBrush(COLOR_SELECTION_ACCENT) } as isize;
                    state.icon_brush = unsafe { CreateSolidBrush(COLOR_ICON_BG) } as isize;
                    state.help_tip_brush = unsafe { CreateSolidBrush(COLOR_HELP_TIP_BG) } as isize;
                    state.help_tip_border_brush =
                        unsafe { CreateSolidBrush(COLOR_PANEL_BORDER) } as isize;

                    state.input_font = create_font(FONT_INPUT_HEIGHT, FW_MEDIUM as i32);
                    state.title_font = create_font(FONT_TITLE_HEIGHT, FW_SEMIBOLD as i32);
                    state.meta_font = create_font(FONT_META_HEIGHT, FW_MEDIUM as i32);
                    state.status_font = create_font(FONT_STATUS_HEIGHT, FW_MEDIUM as i32);
                    state.help_tip_font = create_font(FONT_HELP_TIP_HEIGHT, FW_MEDIUM as i32);

                    state.edit_hwnd = unsafe {
                        CreateWindowExW(
                            0,
                            to_wide(INPUT_CLASS).as_ptr(),
                            to_wide("").as_ptr(),
                            WS_CHILD
                                | WS_VISIBLE
                                | WS_TABSTOP
                                | ES_AUTOHSCROLL as u32
                                | ES_MULTILINE as u32,
                            0,
                            0,
                            0,
                            0,
                            hwnd,
                            CONTROL_ID_INPUT as HMENU,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                        )
                    };

                    state.list_hwnd = unsafe {
                        CreateWindowExW(
                            0,
                            to_wide(LIST_CLASS).as_ptr(),
                            std::ptr::null(),
                            WS_CHILD
                                | WS_TABSTOP
                                | LBS_NOTIFY as u32
                                | LBS_OWNERDRAWFIXED as u32
                                | LBS_HASSTRINGS as u32
                                | LBS_NOINTEGRALHEIGHT as u32,
                            0,
                            0,
                            0,
                            0,
                            hwnd,
                            CONTROL_ID_LIST as HMENU,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                        )
                    };

                    state.status_hwnd = unsafe {
                        CreateWindowExW(
                            0,
                            to_wide(STATUS_CLASS).as_ptr(),
                            to_wide(FOOTER_HINT_TEXT).as_ptr(),
                            WS_CHILD | WS_VISIBLE | STATIC_RIGHT_STYLE,
                            0,
                            0,
                            0,
                            0,
                            hwnd,
                            CONTROL_ID_STATUS as HMENU,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                        )
                    };
                    state.help_hwnd = unsafe {
                        CreateWindowExW(
                            0,
                            to_wide(STATUS_CLASS).as_ptr(),
                            to_wide("?").as_ptr(),
                            WS_CHILD | WS_VISIBLE | STATIC_NOTIFY_STYLE,
                            0,
                            0,
                            0,
                            0,
                            hwnd,
                            CONTROL_ID_HELP as HMENU,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                        )
                    };
                    state.help_tip_hwnd = unsafe {
                        CreateWindowExW(
                            0,
                            to_wide(STATUS_CLASS).as_ptr(),
                            to_wide(HOTKEY_HELP_TEXT_FALLBACK).as_ptr(),
                            WS_CHILD | STATIC_NOTIFY_STYLE,
                            0,
                            0,
                            0,
                            0,
                            hwnd,
                            CONTROL_ID_HELP_TIP as HMENU,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                        )
                    };

                    unsafe {
                        SendMessageW(state.edit_hwnd, WM_SETFONT, state.input_font as usize, 1);
                        SendMessageW(state.list_hwnd, WM_SETFONT, state.meta_font as usize, 1);
                        SendMessageW(state.status_hwnd, WM_SETFONT, state.status_font as usize, 1);
                        SendMessageW(state.help_hwnd, WM_SETFONT, state.status_font as usize, 1);
                        SendMessageW(state.help_tip_hwnd, WM_SETFONT, state.help_tip_font as usize, 1);
                        state.edit_prev_proc = SetWindowLongPtrW(
                            state.edit_hwnd,
                            GWLP_WNDPROC,
                            control_subclass_proc as *const () as isize,
                        );
                        state.list_prev_proc = SetWindowLongPtrW(
                            state.list_hwnd,
                            GWLP_WNDPROC,
                            control_subclass_proc as *const () as isize,
                        );
                        state.help_prev_proc = SetWindowLongPtrW(
                            state.help_hwnd,
                            GWLP_WNDPROC,
                            control_subclass_proc as *const () as isize,
                        );
                        state.help_tip_prev_proc = SetWindowLongPtrW(
                            state.help_tip_hwnd,
                            GWLP_WNDPROC,
                            control_subclass_proc as *const () as isize,
                        );

                        ShowWindow(state.list_hwnd, SW_HIDE);
                        ShowWindow(state.help_tip_hwnd, SW_HIDE);
                    }

                    state.results_visible = false;
                    state.hover_index = -1;
                    layout_children(hwnd, state);
                }
                0
            }
            WM_MEASUREITEM => {
                let measure = lparam as *mut MEASUREITEMSTRUCT;
                if !measure.is_null() {
                    unsafe {
                        if (*measure).CtlID as usize == CONTROL_ID_LIST {
                            (*measure).itemHeight = ROW_HEIGHT as u32;
                            return 1;
                        }
                    }
                }
                unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
            }
            WM_DRAWITEM => {
                let draw = lparam as *mut DRAWITEMSTRUCT;
                if draw.is_null() {
                    return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
                }

                let dis = unsafe { &mut *draw };
                if dis.CtlID as usize != CONTROL_ID_LIST {
                    return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
                }

                draw_list_row(hwnd, dis);
                1
            }
            WM_COMMAND => {
                let control_id = wparam & 0xffff;
                let notification = (wparam >> 16) & 0xffff;
                if control_id == CONTROL_ID_INPUT && notification as u32 == EN_CHANGE as u32 {
                    unsafe {
                        PostMessageW(hwnd, SWIFTFIND_WM_QUERY_CHANGED, 0, 0);
                    }
                    return 0;
                }
                if control_id == CONTROL_ID_LIST && notification as u32 == LBN_DBLCLK as u32 {
                    unsafe {
                        PostMessageW(hwnd, SWIFTFIND_WM_SUBMIT, 0, 0);
                    }
                    return 0;
                }
                if (control_id == CONTROL_ID_HELP || control_id == CONTROL_ID_HELP_TIP)
                    && notification == 0
                {
                    if let Some(state) = state_for(hwnd) {
                        if let Err(error) = open_help_config_file(state) {
                            state.status_is_error = true;
                            state.help_tip_visible = false;
                            let wide = to_wide(&format!("Help open error: {error}"));
                            unsafe {
                                SetWindowTextW(state.status_hwnd, wide.as_ptr());
                                InvalidateRect(state.status_hwnd, std::ptr::null(), 1);
                            }
                            layout_children(hwnd, state);
                        }
                    }
                    return 0;
                }
                unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
            }
            WM_CTLCOLORSTATIC => {
                if let Some(state) = state_for(hwnd) {
                    let target = lparam as HWND;
                    if target == state.help_tip_hwnd {
                        unsafe {
                            SetTextColor(wparam as _, COLOR_HELP_TIP_TEXT);
                            SetBkColor(wparam as _, COLOR_HELP_TIP_BG);
                            SetBkMode(wparam as _, OPAQUE as i32);
                        }
                        return state.help_tip_brush;
                    }
                    if target == state.help_hwnd {
                        unsafe {
                            SetTextColor(
                                wparam as _,
                                if state.help_hovered {
                                    COLOR_HELP_ICON_HOVER
                                } else {
                                    COLOR_HELP_ICON
                                },
                            );
                            SetBkMode(wparam as _, TRANSPARENT as i32);
                        }
                        return state.panel_brush;
                    }
                    if target == state.status_hwnd {
                        let color = if state.status_is_error {
                            COLOR_TEXT_ERROR
                        } else {
                            COLOR_TEXT_HINT
                        };
                        unsafe {
                            SetTextColor(wparam as _, color);
                            SetBkMode(wparam as _, TRANSPARENT as i32);
                        }
                        return state.panel_brush;
                    }
                }
                unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
            }
            WM_CTLCOLOREDIT => {
                if let Some(state) = state_for(hwnd) {
                    let target = lparam as HWND;
                    if target == state.edit_hwnd {
                        unsafe {
                            SetTextColor(wparam as _, COLOR_TEXT_PRIMARY);
                            SetBkColor(wparam as _, COLOR_INPUT_BG);
                            SetBkMode(wparam as _, OPAQUE as i32);
                        }
                        return state.input_brush;
                    }
                }
                unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
            }
            WM_CTLCOLORLISTBOX => {
                if let Some(state) = state_for(hwnd) {
                    let target = lparam as HWND;
                    if target == state.list_hwnd {
                        unsafe {
                            SetTextColor(wparam as _, COLOR_TEXT_PRIMARY);
                            SetBkColor(wparam as _, COLOR_RESULTS_BG);
                        }
                        return state.results_brush;
                    }
                }
                unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
            }
            WM_SIZE => {
                if let Some(state) = state_for(hwnd) {
                    layout_children(hwnd, state);
                }
                apply_rounded_corners_hwnd(hwnd);
                0
            }
            WM_ACTIVATE => {
                let activation = (wparam & 0xFFFF) as u32;
                if activation == 0 {
                    unsafe {
                        PostMessageW(hwnd, SWIFTFIND_WM_ESCAPE, 0, 0);
                    }
                    hide_overlay_immediate(hwnd);
                }
                0
            }
            WM_PAINT => {
                draw_panel_background(hwnd);
                0
            }
            WM_TIMER => {
                if wparam == TIMER_SCROLL_ANIM {
                    if let Some(state) = state_for(hwnd) {
                        let running = scroll_animation_tick(state);
                        if !running {
                            unsafe {
                                KillTimer(hwnd, TIMER_SCROLL_ANIM);
                            }
                        }
                    }
                }
                if wparam == TIMER_WINDOW_ANIM {
                    if let Some(state) = state_for(hwnd) {
                        let running = window_animation_tick(hwnd, state);
                        if !running {
                            unsafe {
                                KillTimer(hwnd, TIMER_WINDOW_ANIM);
                            }
                        }
                    }
                }
                if wparam == TIMER_ROW_ANIM {
                    if let Some(state) = state_for(hwnd) {
                        let running = row_animation_tick(state);
                        unsafe {
                            InvalidateRect(state.list_hwnd, std::ptr::null(), 0);
                        }
                        if !running {
                            unsafe {
                                KillTimer(hwnd, TIMER_ROW_ANIM);
                            }
                        }
                    }
                }
                0
            }
            WM_CLOSE => {
                unsafe {
                    ShowWindow(hwnd, SW_HIDE);
                }
                0
            }
            WM_DESTROY => {
                unsafe {
                    PostQuitMessage(0);
                }
                0
            }
            WM_NCDESTROY => {
                let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OverlayShellState };
                if !state_ptr.is_null() {
                    unsafe {
                        cleanup_state_resources(&mut *state_ptr);
                        let _ = Box::from_raw(state_ptr);
                        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    }
                }
                0
            }
            SWIFTFIND_WM_ESCAPE
            | SWIFTFIND_WM_QUERY_CHANGED
            | SWIFTFIND_WM_MOVE_UP
            | SWIFTFIND_WM_MOVE_DOWN
            | SWIFTFIND_WM_SUBMIT => 0,
            _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
        }
    }

    extern "system" fn control_subclass_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let parent = unsafe { GetParent(hwnd) };
        if parent.is_null() {
            return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
        }

        if let Some(state) = state_for(parent) {
            if hwnd == state.help_tip_hwnd && message == WM_PAINT {
                paint_help_tip(hwnd, state);
                return 0;
            }
            if hwnd == state.edit_hwnd
                && (message == WM_SETFOCUS
                    || message == WM_KEYDOWN
                    || message == windows_sys::Win32::UI::WindowsAndMessaging::WM_LBUTTONDOWN
                    || message == WM_LBUTTONUP)
            {
                hide_input_caret(hwnd);
            }
            if message == WM_MOUSEMOVE {
                if hwnd == state.help_hwnd || hwnd == state.help_tip_hwnd {
                    set_help_hover_state(parent, state, true);
                } else if state.help_hovered {
                    set_help_hover_state(parent, state, false);
                }
            }
            if message == windows_sys::Win32::UI::WindowsAndMessaging::WM_SETCURSOR
                && (hwnd == state.help_hwnd || hwnd == state.help_tip_hwnd)
            {
                unsafe {
                    SetCursor(LoadCursorW(std::ptr::null_mut(), IDC_HAND));
                }
                return 1;
            }
            if message == WM_MOUSEMOVE && hwnd == state.list_hwnd {
                let mut cursor = POINT { x: 0, y: 0 };
                unsafe {
                    GetCursorPos(&mut cursor);
                    ScreenToClient(hwnd, &mut cursor);
                }
                let packed = ((cursor.y as u32) << 16) | (cursor.x as u32 & 0xFFFF);
                let hit = unsafe { SendMessageW(hwnd, LB_ITEMFROMPOINT, 0, packed as isize) };
                let row = (hit & 0xFFFF) as i32;
                let outside = ((hit >> 16) & 0xFFFF) != 0;
                let count = unsafe { SendMessageW(hwnd, LB_GETCOUNT, 0, 0) as i32 };
                let next_hover = if outside || count <= 0 || row < 0 || row >= count {
                    -1
                } else {
                    row
                };

                if next_hover != state.hover_index {
                    let previous_hover = state.hover_index;
                    state.hover_index = next_hover;
                    invalidate_list_row(hwnd, previous_hover);
                    invalidate_list_row(hwnd, next_hover);
                }
            }
            if message == WM_MOUSEWHEEL && hwnd == state.list_hwnd {
                let count = unsafe { SendMessageW(hwnd, LB_GETCOUNT, 0, 0) };
                if count > 0 {
                    let current_top = unsafe { SendMessageW(hwnd, LB_GETTOPINDEX, 0, 0) as i32 };
                    let visible_rows = visible_row_capacity(hwnd);
                    let max_top = (count as i32 - visible_rows).max(0);
                    let wheel = ((wparam >> 16) & 0xFFFF) as u16 as i16;
                    let mut notches = (wheel as i32) / 120;
                    if notches == 0 && wheel != 0 {
                        notches = if wheel > 0 { 1 } else { -1 };
                    }
                    if notches != 0 {
                        let target_top =
                            (current_top - notches * WHEEL_LINES_PER_NOTCH).clamp(0, max_top);
                        state.scroll_anim_start = None;
                        unsafe {
                            KillTimer(parent, TIMER_SCROLL_ANIM);
                            SendMessageW(hwnd, LB_SETTOPINDEX, target_top as usize, 0);
                        }
                    }
                }
                return 0;
            }
            if message == WM_LBUTTONUP && hwnd == state.list_hwnd {
                let count = unsafe { SendMessageW(hwnd, LB_GETCOUNT, 0, 0) as i32 };
                if count > 0 {
                    let x = (lparam as u32 & 0xFFFF) as i16 as i32;
                    let y = ((lparam as u32 >> 16) & 0xFFFF) as i16 as i32;
                    let packed = ((y as u32) << 16) | (x as u32 & 0xFFFF);
                    let hit = unsafe { SendMessageW(hwnd, LB_ITEMFROMPOINT, 0, packed as isize) };
                    let row = (hit & 0xFFFF) as i32;
                    let outside = ((hit >> 16) & 0xFFFF) != 0;
                    if !outside && row >= 0 && row < count {
                        unsafe {
                            SendMessageW(hwnd, LB_SETCURSEL, row as usize, 0);
                        PostMessageW(parent, SWIFTFIND_WM_SUBMIT, 0, 0);
                        }
                    }
                }
                return 0;
            }
            if (message == WM_LBUTTONUP || message == windows_sys::Win32::UI::WindowsAndMessaging::WM_LBUTTONDOWN)
                && (hwnd == state.help_hwnd || hwnd == state.help_tip_hwnd)
            {
                if let Err(error) = open_help_config_file(state) {
                    state.status_is_error = true;
                    state.help_tip_visible = false;
                    let wide = to_wide(&format!("Help open error: {error}"));
                    unsafe {
                        SetWindowTextW(state.status_hwnd, wide.as_ptr());
                        InvalidateRect(state.status_hwnd, std::ptr::null(), 1);
                    }
                    layout_children(parent, state);
                }
                return 0;
            }
        }

        if message == WM_KEYDOWN {
            match wparam as u16 {
                VK_ESCAPE => {
                    unsafe {
                        PostMessageW(parent, SWIFTFIND_WM_ESCAPE, 0, 0);
                    }
                    return 0;
                }
                VK_UP => {
                    unsafe {
                        PostMessageW(parent, SWIFTFIND_WM_MOVE_UP, 0, 0);
                    }
                    return 0;
                }
                VK_DOWN => {
                    unsafe {
                        PostMessageW(parent, SWIFTFIND_WM_MOVE_DOWN, 0, 0);
                    }
                    return 0;
                }
                VK_RETURN => {
                    unsafe {
                        PostMessageW(parent, SWIFTFIND_WM_SUBMIT, 0, 0);
                    }
                    return 0;
                }
                _ => {}
            }
        }

        let Some(state) = state_for(parent) else {
            return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
        };

        let prev_ptr = if hwnd == state.edit_hwnd {
            state.edit_prev_proc
        } else if hwnd == state.list_hwnd {
            state.list_prev_proc
        } else if hwnd == state.help_hwnd {
            state.help_prev_proc
        } else if hwnd == state.help_tip_hwnd {
            state.help_tip_prev_proc
        } else {
            0
        };

        if prev_ptr == 0 {
            return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
        }

        let prev_proc = unsafe {
            std::mem::transmute::<isize, windows_sys::Win32::UI::WindowsAndMessaging::WNDPROC>(prev_ptr)
        };
        let result = unsafe { CallWindowProcW(prev_proc, hwnd, message, wparam, lparam) };
        if hwnd == state.edit_hwnd && message == WM_PAINT {
            paint_edit_placeholder(hwnd, state);
        }
        result
    }

    fn paint_edit_placeholder(edit_hwnd: HWND, state: &OverlayShellState) {
        let text_len = unsafe { GetWindowTextLengthW(edit_hwnd) };
        if text_len > 0 {
            return;
        }

        let mut text_rect: RECT = unsafe { std::mem::zeroed() };
        unsafe {
            SendMessageW(edit_hwnd, EM_GETRECT, 0, &mut text_rect as *mut RECT as LPARAM);
        }
        if text_rect.right <= text_rect.left || text_rect.bottom <= text_rect.top {
            let mut client: RECT = unsafe { std::mem::zeroed() };
            unsafe {
                GetClientRect(edit_hwnd, &mut client);
            }
            let line_height = input_line_height_for_edit(edit_hwnd, state.input_font);
            text_rect = compute_input_text_rect(
                client.right - client.left,
                client.bottom - client.top,
                line_height,
            );
        }
        if text_rect.right <= text_rect.left {
            return;
        }

        let hdc = unsafe { GetDC(edit_hwnd) };
        if hdc.is_null() {
            return;
        }

        unsafe {
            let old_font = SelectObject(hdc, state.input_font as _);
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, COLOR_TEXT_SECONDARY);
            let placeholder = to_wide("Type to search");
            DrawTextW(
                hdc,
                placeholder.as_ptr(),
                -1,
                &mut text_rect,
                DT_LEFT | DT_SINGLELINE | DT_EDITCONTROL | DT_VCENTER | DT_END_ELLIPSIS,
            );
            SelectObject(hdc, old_font);
            ReleaseDC(edit_hwnd, hdc);
        }
    }

    fn hide_input_caret(edit_hwnd: HWND) {
        unsafe {
            let _ = HideCaret(edit_hwnd);
        }
    }

    fn draw_list_row(hwnd: HWND, dis: &mut DRAWITEMSTRUCT) {
        if dis.itemID == u32::MAX {
            return;
        }

        let Some(state) = state_for(hwnd) else {
            return;
        };

        let item_index = dis.itemID as i32;
        let row = state
            .rows
            .get(item_index as usize)
            .cloned()
            .unwrap_or_else(|| OverlayRow {
                kind: "file".to_string(),
                title: String::new(),
                path: String::new(),
                icon_path: String::new(),
            });

        let visibility = row_animation_visibility(state, item_index);
        let offset_y = if state.row_anim_exiting {
            -(((1.0 - visibility) * 4.0).round() as i32)
        } else {
            ((1.0 - visibility) * 6.0).round() as i32
        };

        let selected_flag = (dis.itemState & ODS_SELECTED as u32) != 0;
        let hovered = state.hover_index == item_index;
        unsafe {
            FillRect(dis.hDC, &dis.rcItem, state.results_brush as _);
            if selected_flag || hovered {
                let row_rect = RECT {
                    left: dis.rcItem.left + 2,
                    top: dis.rcItem.top + ROW_VERTICAL_INSET + offset_y,
                    right: dis.rcItem.right - 2,
                    bottom: dis.rcItem.bottom - ROW_VERTICAL_INSET + offset_y,
                };
                let region = CreateRoundRectRgn(
                    row_rect.left,
                    row_rect.top,
                    row_rect.right,
                    row_rect.bottom,
                    ROW_ACTIVE_RADIUS,
                    ROW_ACTIVE_RADIUS,
                );
                let base_fill = if selected_flag { COLOR_SELECTION } else { COLOR_ROW_HOVER };
                let fill_color = blend_color(COLOR_RESULTS_BG, base_fill, visibility);
                let fill_brush = CreateSolidBrush(fill_color);
                FillRgn(dis.hDC, region, fill_brush);
                DeleteObject(fill_brush as _);
                DeleteObject(region as _);
            }

            let icon_rect = RECT {
                left: dis.rcItem.left + ROW_INSET_X,
                top: dis.rcItem.top + (ROW_HEIGHT - ROW_ICON_SIZE) / 2 + offset_y,
                right: dis.rcItem.left + ROW_INSET_X + ROW_ICON_SIZE,
                bottom: dis.rcItem.top + (ROW_HEIGHT - ROW_ICON_SIZE) / 2 + ROW_ICON_SIZE + offset_y,
            };
            let icon_backplate = CreateRoundRectRgn(
                icon_rect.left,
                icon_rect.top,
                icon_rect.right + 1,
                icon_rect.bottom + 1,
                8,
                8,
            );
            FillRgn(dis.hDC, icon_backplate, state.icon_brush as _);
            DeleteObject(icon_backplate as _);

            let old_font = SelectObject(dis.hDC, state.title_font as _);
            SetBkMode(dis.hDC, TRANSPARENT as i32);
            let primary_text = blend_color(COLOR_RESULTS_BG, COLOR_TEXT_PRIMARY, visibility);
            let secondary_text = blend_color(COLOR_RESULTS_BG, COLOR_TEXT_SECONDARY, visibility);
            let highlight_text = blend_color(COLOR_RESULTS_BG, COLOR_TEXT_HIGHLIGHT, visibility);
            SetTextColor(dis.hDC, primary_text);

            let icon_drawn = draw_row_icon(dis.hDC, &icon_rect, &row, state);
            if !icon_drawn {
                FillRect(dis.hDC, &icon_rect, state.icon_brush as _);
                let mut icon_text_rect = icon_rect;
                SetTextColor(dis.hDC, COLOR_ICON_TEXT);
                DrawTextW(
                    dis.hDC,
                    to_wide(icon_glyph_for_kind(&row.kind)).as_ptr(),
                    -1,
                    &mut icon_text_rect,
                    DT_CENTER | DT_SINGLELINE | DT_VCENTER,
                );
            }

            SetTextColor(dis.hDC, primary_text);
            let text_left = icon_rect.right + ROW_ICON_GAP;
            let has_meta = !row.path.trim().is_empty();
            let title_rect = RECT {
                left: text_left,
                top: if has_meta {
                    dis.rcItem.top + ROW_TEXT_TOP_PAD + offset_y
                } else {
                    dis.rcItem.top + offset_y
                },
                right: dis.rcItem.right - ROW_INSET_X,
                bottom: if has_meta {
                    dis.rcItem.top + ROW_HEIGHT / 2 + offset_y
                } else {
                    dis.rcItem.bottom + offset_y
                },
            };
            draw_highlighted_title(
                dis.hDC,
                &title_rect,
                &row.title,
                &state.active_query,
                primary_text,
                highlight_text,
            );

            if has_meta {
                SelectObject(dis.hDC, state.meta_font as _);
                SetTextColor(dis.hDC, secondary_text);
                let mut path_rect = RECT {
                    left: text_left,
                    top: dis.rcItem.top + ROW_HEIGHT / 2 - 1 + offset_y,
                    right: dis.rcItem.right - ROW_INSET_X,
                    bottom: dis.rcItem.bottom - ROW_TEXT_BOTTOM_PAD + offset_y,
                };
                DrawTextW(
                    dis.hDC,
                    to_wide(&row.path).as_ptr(),
                    -1,
                    &mut path_rect,
                    DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
                );
            }

            SelectObject(dis.hDC, old_font);
        }
    }

    fn row_animation_visibility(state: &OverlayShellState, item_index: i32) -> f32 {
        let Some(start) = state.row_anim_start else {
            return 1.0;
        };

        let stagger = (item_index.max(0) as u64) * ROW_STAGGER_MS;
        let elapsed = start.elapsed().as_millis() as u64;
        if elapsed <= stagger {
            return if state.row_anim_exiting { 1.0 } else { 0.0 };
        }

        let t = ((elapsed - stagger) as f32 / ROW_ANIM_MS as f32).clamp(0.0, 1.0);
        let eased = ease_out(t);
        if state.row_anim_exiting {
            1.0 - eased
        } else {
            eased
        }
    }

    fn draw_highlighted_title(
        hdc: HDC,
        rect: &RECT,
        title: &str,
        query: &str,
        base_color: u32,
        highlight_color: u32,
    ) {
        if rect.right <= rect.left || title.trim().is_empty() {
            return;
        }

        let max_width = rect.right - rect.left;
        if max_width <= 0 {
            return;
        }

        let display = fit_text_with_ellipsis(hdc, title, max_width);
        if display.is_empty() {
            return;
        }

        let highlighted = fuzzy_match_positions(&display, query);
        let text_height = current_text_height(hdc).max(1);
        let y = rect.top + ((rect.bottom - rect.top - text_height).max(0) / 2);
        let mut x = rect.left;

        for (index, ch) in display.chars().enumerate() {
            let s = ch.to_string();
            let width = measure_text_width(hdc, &s).max(1);
            if x + width > rect.right {
                break;
            }

            let wide = to_wide_no_nul(&s);
            unsafe {
                SetTextColor(
                    hdc,
                    if highlighted.contains(&index) {
                        highlight_color
                    } else {
                        base_color
                    },
                );
                TextOutW(hdc, x, y, wide.as_ptr(), wide.len() as i32);
            }
            x += width;
        }
    }

    fn fit_text_with_ellipsis(hdc: HDC, text: &str, max_width: i32) -> String {
        if text.trim().is_empty() {
            return String::new();
        }
        if measure_text_width(hdc, text) <= max_width {
            return text.to_string();
        }

        let ellipsis = "...";
        let ellipsis_width = measure_text_width(hdc, ellipsis);
        if ellipsis_width >= max_width {
            return String::new();
        }

        let mut output = String::new();
        for ch in text.chars() {
            let mut candidate = output.clone();
            candidate.push(ch);
            if measure_text_width(hdc, &candidate) + ellipsis_width > max_width {
                break;
            }
            output.push(ch);
        }
        output.push_str(ellipsis);
        output
    }

    fn fuzzy_match_positions(text: &str, query: &str) -> HashSet<usize> {
        let query = query.trim();
        if query.is_empty() {
            return HashSet::new();
        }

        let mut matched = HashSet::new();
        let mut text_iter = text.chars().enumerate();

        for q in query.chars() {
            let q_lower = q.to_ascii_lowercase();
            let mut found = None;
            for (index, t) in text_iter.by_ref() {
                if t.to_ascii_lowercase() == q_lower {
                    found = Some(index);
                    break;
                }
            }
            if let Some(index) = found {
                matched.insert(index);
            } else {
                return HashSet::new();
            }
        }

        matched
    }

    fn measure_text_width(hdc: HDC, text: &str) -> i32 {
        if text.is_empty() {
            return 0;
        }
        let wide = to_wide_no_nul(text);
        let mut size: SIZE = unsafe { std::mem::zeroed() };
        let ok = unsafe { GetTextExtentPoint32W(hdc, wide.as_ptr(), wide.len() as i32, &mut size) };
        if ok == 0 {
            0
        } else {
            size.cx
        }
    }

    fn current_text_height(hdc: HDC) -> i32 {
        let mut tm: TEXTMETRICW = unsafe { std::mem::zeroed() };
        let ok = unsafe { GetTextMetricsW(hdc, &mut tm) };
        if ok == 0 {
            14
        } else {
            tm.tmHeight as i32
        }
    }

    fn blend_color(from: u32, to: u32, t: f32) -> u32 {
        let t = t.clamp(0.0, 1.0);
        let lerp = |a: u32, b: u32| -> u32 { (a as f32 + (b as f32 - a as f32) * t).round() as u32 };
        let r = lerp(from & 0xFF, to & 0xFF);
        let g = lerp((from >> 8) & 0xFF, (to >> 8) & 0xFF);
        let b = lerp((from >> 16) & 0xFF, (to >> 16) & 0xFF);
        r | (g << 8) | (b << 16)
    }

    fn icon_glyph_for_kind(kind: &str) -> &'static str {
        if kind.eq_ignore_ascii_case("app") {
            "A"
        } else if kind.eq_ignore_ascii_case("folder") {
            "D"
        } else {
            "F"
        }
    }

    fn draw_row_icon(hdc: HDC, icon_rect: &RECT, row: &OverlayRow, state: &mut OverlayShellState) -> bool {
        let Some(icon_handle) = icon_handle_for_row(state, row) else {
            return false;
        };
        let icon_size = ROW_ICON_DRAW_SIZE;
        let x = icon_rect.left + (ROW_ICON_SIZE - icon_size) / 2;
        let y = icon_rect.top + (ROW_ICON_SIZE - icon_size) / 2;
        unsafe {
            DrawIconEx(
                hdc,
                x,
                y,
                icon_handle as _,
                icon_size,
                icon_size,
                0,
                std::ptr::null_mut(),
                DI_NORMAL,
            ) != 0
        }
    }

    fn icon_handle_for_row(state: &mut OverlayShellState, row: &OverlayRow) -> Option<isize> {
        let key = icon_cache_key(row);
        if let Some(cached) = state.icon_cache.get(&key) {
            return if *cached == 0 { None } else { Some(*cached) };
        }

        let loaded = load_shell_icon_for_row(row).unwrap_or(0);
        state.icon_cache.insert(key, loaded);
        if loaded == 0 {
            None
        } else {
            Some(loaded)
        }
    }

    fn icon_cache_key(row: &OverlayRow) -> String {
        let kind = row.kind.to_ascii_lowercase();
        let source = row.icon_path.trim().to_ascii_lowercase();
        if source.is_empty() {
            format!("kind:{kind}")
        } else {
            format!("kind:{kind}|{source}")
        }
    }

    fn load_shell_icon_for_row(row: &OverlayRow) -> Option<isize> {
        let kind = row.kind.to_ascii_lowercase();
        let source = row.icon_path.trim();

        if kind == "folder" {
            return shell_icon_with_attrs("folder", FILE_ATTRIBUTE_DIRECTORY);
        }

        if !source.is_empty() {
            if let Some(icon) = shell_icon_for_existing_path(source) {
                return Some(icon);
            }
            if let Some(icon) = shell_icon_with_attrs(source, FILE_ATTRIBUTE_NORMAL) {
                return Some(icon);
            }
        }

        if kind == "app" {
            if let Some(icon) = shell_icon_with_attrs("swiftfind.exe", FILE_ATTRIBUTE_NORMAL) {
                return Some(icon);
            }
        }

        shell_icon_with_attrs("swiftfind.file", FILE_ATTRIBUTE_NORMAL)
    }

    fn shell_icon_for_existing_path(path: &str) -> Option<isize> {
        let mut sfi: SHFILEINFOW = unsafe { std::mem::zeroed() };
        let wide = to_wide(path);
        let flags = SHGFI_ICON | SHGFI_SMALLICON;
        let result = unsafe {
            SHGetFileInfoW(
                wide.as_ptr(),
                0,
                &mut sfi,
                std::mem::size_of::<SHFILEINFOW>() as u32,
                flags,
            )
        };
        if result == 0 || sfi.hIcon.is_null() {
            None
        } else {
            Some(sfi.hIcon as isize)
        }
    }

    fn shell_icon_with_attrs(path_hint: &str, attrs: u32) -> Option<isize> {
        let mut sfi: SHFILEINFOW = unsafe { std::mem::zeroed() };
        let wide = to_wide(path_hint);
        let flags = SHGFI_ICON | SHGFI_SMALLICON | SHGFI_USEFILEATTRIBUTES;
        let result = unsafe {
            SHGetFileInfoW(
                wide.as_ptr(),
                attrs,
                &mut sfi,
                std::mem::size_of::<SHFILEINFOW>() as u32,
                flags,
            )
        };
        if result == 0 || sfi.hIcon.is_null() {
            None
        } else {
            Some(sfi.hIcon as isize)
        }
    }

    fn clear_icon_cache(state: &mut OverlayShellState) {
        for handle in state.icon_cache.values() {
            if *handle != 0 {
                unsafe {
                    DestroyIcon(*handle as _);
                }
            }
        }
        state.icon_cache.clear();
    }


    fn begin_scroll_animation(
        overlay_hwnd: HWND,
        state: &mut OverlayShellState,
        selected_index: i32,
        count: i32,
    ) {
        if count <= 0 {
            return;
        }

        let current_top = unsafe { SendMessageW(state.list_hwnd, LB_GETTOPINDEX, 0, 0) as i32 };
        let target_top = target_top_index_for_selection(state.list_hwnd, selected_index, count, current_top);
        begin_scroll_animation_to_top(overlay_hwnd, state, target_top);
    }

    fn begin_scroll_animation_to_top(
        overlay_hwnd: HWND,
        state: &mut OverlayShellState,
        target_top: i32,
    ) {
        let current_top = unsafe { SendMessageW(state.list_hwnd, LB_GETTOPINDEX, 0, 0) as i32 };
        state.scroll_anim_start = None;
        state.scroll_from_top = target_top;
        state.scroll_to_top = target_top;
        unsafe {
            KillTimer(overlay_hwnd, TIMER_SCROLL_ANIM);
            if target_top != current_top {
                SendMessageW(state.list_hwnd, LB_SETTOPINDEX, target_top as usize, 0);
            }
        }
    }

    fn target_top_index_for_selection(
        list_hwnd: HWND,
        selected_index: i32,
        count: i32,
        current_top: i32,
    ) -> i32 {
        let visible_rows = visible_row_capacity(list_hwnd);
        let mut target_top = current_top;

        if selected_index < current_top {
            target_top = selected_index;
        } else if selected_index >= current_top + visible_rows {
            target_top = selected_index - visible_rows + 1;
        }

        let max_top = (count - visible_rows).max(0);
        target_top.clamp(0, max_top)
    }

    fn visible_row_capacity(list_hwnd: HWND) -> i32 {
        let mut rect: RECT = unsafe { std::mem::zeroed() };
        unsafe {
            GetClientRect(list_hwnd, &mut rect);
        }
        let height = (rect.bottom - rect.top).max(0);
        let rows = height / ROW_HEIGHT;
        rows.max(1)
    }

    fn scroll_animation_tick(state: &mut OverlayShellState) -> bool {
        let Some(start) = state.scroll_anim_start else {
            return false;
        };

        let elapsed = start.elapsed().as_millis() as u64;
        let t = (elapsed as f32 / SCROLL_ANIM_MS as f32).clamp(0.0, 1.0);
        let eased = ease_out(t);
        let next_top = lerp_i32(state.scroll_from_top, state.scroll_to_top, eased);
        unsafe {
            SendMessageW(state.list_hwnd, LB_SETTOPINDEX, next_top as usize, 0);
        }

        if t >= 1.0 {
            state.scroll_anim_start = None;
            return false;
        }
        true
    }

    fn row_animation_tick(state: &mut OverlayShellState) -> bool {
        let Some(start) = state.row_anim_start else {
            return false;
        };

        let row_count = state.rows.len() as u64;
        let total_ms = ROW_ANIM_MS + row_count.min(MAX_VISIBLE_ROWS as u64) * ROW_STAGGER_MS;
        let elapsed = start.elapsed().as_millis() as u64;
        if elapsed >= total_ms {
            state.row_anim_start = None;
            state.row_anim_exiting = false;
            return false;
        }
        true
    }

    fn start_window_animation(
        hwnd: HWND,
        from_left: i32,
        from_top: i32,
        from_width: i32,
        from_height: i32,
        to_left: i32,
        to_top: i32,
        to_width: i32,
        to_height: i32,
        from_alpha: u8,
        to_alpha: u8,
        duration_ms: u32,
        hide_on_complete: bool,
    ) {
        let Some(state) = state_for(hwnd) else {
            return;
        };

        state.window_anim = Some(WindowAnimation {
            start: Instant::now(),
            duration_ms: duration_ms.max(1),
            from_left,
            from_top,
            from_width,
            from_height,
            to_left,
            to_top,
            to_width,
            to_height,
            from_alpha,
            to_alpha,
            hide_on_complete,
        });

        unsafe {
            SetTimer(hwnd, TIMER_WINDOW_ANIM, ANIM_FRAME_MS as u32, None);
        }
    }

    fn window_animation_tick(hwnd: HWND, state: &mut OverlayShellState) -> bool {
        let Some(anim) = state.window_anim.as_ref() else {
            return false;
        };

        let elapsed_ms = anim.start.elapsed().as_millis() as u32;
        let t = (elapsed_ms as f32 / anim.duration_ms as f32).clamp(0.0, 1.0);
        let eased = ease_out(t);

        let left = lerp_i32(anim.from_left, anim.to_left, eased);
        let top = lerp_i32(anim.from_top, anim.to_top, eased);
        let width = lerp_i32(anim.from_width, anim.to_width, eased);
        let height = lerp_i32(anim.from_height, anim.to_height, eased);
        let alpha = lerp_i32(anim.from_alpha as i32, anim.to_alpha as i32, eased) as u8;
        apply_window_state(hwnd, left, top, width, height, alpha);

        if t >= 1.0 {
            let hide_on_complete = anim.hide_on_complete;
            state.window_anim = None;
            if hide_on_complete {
                unsafe {
                    ShowWindow(hwnd, SW_HIDE);
                    SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
                }
            } else if !state.results_visible {
                unsafe {
                    ShowWindow(state.list_hwnd, SW_HIDE);
                    SendMessageW(state.list_hwnd, LB_RESETCONTENT, 0, 0);
                }
                state.rows.clear();
                state.hover_index = -1;
                state.row_anim_start = None;
                state.row_anim_exiting = false;
            }
            return false;
        }

        true
    }

    fn apply_window_state(hwnd: HWND, left: i32, top: i32, width: i32, height: i32, alpha: u8) {
        unsafe {
            SetWindowPos(
                hwnd,
                HWND_TOP,
                left,
                top,
                width.max(1),
                height.max(1),
                SWP_NOACTIVATE,
            );
            SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA);
        }
    }

    fn hide_overlay_immediate(hwnd: HWND) {
        unsafe {
            KillTimer(hwnd, TIMER_WINDOW_ANIM);
            KillTimer(hwnd, TIMER_ROW_ANIM);
            SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
            ShowWindow(hwnd, SW_HIDE);
        }
    }


    fn layout_children(hwnd: HWND, state: &mut OverlayShellState) {
        let mut rect: RECT = unsafe { std::mem::zeroed() };
        unsafe {
            GetClientRect(hwnd, &mut rect);
        }
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width <= 0 || height <= 0 {
            return;
        }

        let input_width = width - PANEL_MARGIN_X * 2;
        let help_reserved = HELP_ICON_SIZE + HELP_ICON_RIGHT_INSET + HELP_ICON_GAP_FROM_INPUT;
        let edit_width = (input_width - help_reserved).max(120);
        let status_len = unsafe { GetWindowTextLengthW(state.status_hwnd) };
        let status_visible = status_len > 0;
        // Keep input exactly centered in compact mode and stable across states.
        let input_top = INPUT_TOP.max(0);
        let status_top = COMPACT_HEIGHT - PANEL_MARGIN_BOTTOM - STATUS_HEIGHT;

        let list_top = COMPACT_HEIGHT + INPUT_TO_LIST_GAP;
        let list_left = PANEL_MARGIN_X + 1;
        let list_width = (input_width - 2).max(0);
        let list_height = (height - list_top - PANEL_MARGIN_X - 1).max(0);
        let help_left = PANEL_MARGIN_X + edit_width + HELP_ICON_GAP_FROM_INPUT;
        let help_top = input_top + (INPUT_HEIGHT - HELP_ICON_SIZE) / 2;
        let tip_left = (help_left + HELP_ICON_SIZE - HELP_TIP_WIDTH).max(PANEL_MARGIN_X);
        let tip_top = (help_top - HELP_TIP_HEIGHT - 6).max(6);

        unsafe {
            MoveWindow(
                state.edit_hwnd,
                PANEL_MARGIN_X,
                input_top,
                edit_width,
                INPUT_HEIGHT,
                1,
            );
            apply_edit_text_rect(state.edit_hwnd);
            if status_visible {
                ShowWindow(state.status_hwnd, SW_SHOW);
                MoveWindow(
                    state.status_hwnd,
                    PANEL_MARGIN_X,
                    status_top,
                    input_width,
                    STATUS_HEIGHT,
                    1,
                );
            } else {
                ShowWindow(state.status_hwnd, SW_HIDE);
            }
            MoveWindow(state.help_hwnd, help_left, help_top, HELP_ICON_SIZE, HELP_ICON_SIZE, 1);
            MoveWindow(
                state.help_tip_hwnd,
                tip_left,
                tip_top,
                HELP_TIP_WIDTH,
                HELP_TIP_HEIGHT,
                1,
            );
            apply_help_tip_rounded_corners(state.help_tip_hwnd, HELP_TIP_WIDTH, HELP_TIP_HEIGHT);
            if state.help_tip_visible {
                ShowWindow(state.help_tip_hwnd, SW_SHOW);
            } else {
                ShowWindow(state.help_tip_hwnd, SW_HIDE);
            }
            MoveWindow(
                state.list_hwnd,
                list_left,
                list_top,
                list_width,
                list_height,
                1,
            );
            apply_list_rounded_corners(state.list_hwnd, list_width, list_height);
        }
    }

    fn apply_edit_text_rect(edit_hwnd: HWND) {
        let mut client: RECT = unsafe { std::mem::zeroed() };
        unsafe {
            GetClientRect(edit_hwnd, &mut client);
        }
        let width = (client.right - client.left).max(0);
        let height = (client.bottom - client.top).max(0);
        if width <= 0 || height <= 0 {
            return;
        }

        let line_height = input_line_height_for_edit(edit_hwnd, 0);
        let text_rect = compute_input_text_rect(width, height, line_height);

        unsafe {
            SendMessageW(
                edit_hwnd,
                EM_SETRECTNP,
                0,
                (&text_rect as *const RECT) as LPARAM,
            );
            InvalidateRect(edit_hwnd, std::ptr::null(), 1);
        }
    }

    fn compute_input_text_rect(width: i32, height: i32, line_height: i32) -> RECT {
        let line_height = line_height.clamp(14, (height - 2).max(14));
        let centered_top = ((height - line_height) / 2).max(0) + INPUT_TEXT_SHIFT_Y;
        let max_top = (height - line_height).max(0);
        let top = centered_top.clamp(0, max_top);
        let mut text_rect = RECT {
            left: INPUT_TEXT_LEFT_INSET + INPUT_TEXT_SHIFT_X,
            top,
            right: width - INPUT_TEXT_RIGHT_INSET + INPUT_TEXT_SHIFT_X,
            bottom: top + line_height,
        };
        if text_rect.right <= text_rect.left {
            text_rect.right = width;
        }
        if text_rect.bottom <= text_rect.top {
            text_rect.top = 0;
            text_rect.bottom = height;
        }
        text_rect
    }

    fn input_line_height_for_edit(edit_hwnd: HWND, fallback_font: isize) -> i32 {
        let hdc = unsafe { GetDC(edit_hwnd) };
        if hdc.is_null() {
            return INPUT_TEXT_LINE_HEIGHT_FALLBACK;
        }

        let font_to_use = if fallback_font != 0 {
            fallback_font
        } else if let Some(state) = state_for(unsafe { GetParent(edit_hwnd) }) {
            state.input_font
        } else {
            0
        };

        let old_font = if font_to_use != 0 {
            unsafe { SelectObject(hdc, font_to_use as _) }
        } else {
            std::ptr::null_mut()
        };

        let mut tm: TEXTMETRICW = unsafe { std::mem::zeroed() };
        let ok = unsafe { GetTextMetricsW(hdc, &mut tm) };

        if !old_font.is_null() {
            unsafe {
                SelectObject(hdc, old_font);
            }
        }
        unsafe {
            ReleaseDC(edit_hwnd, hdc);
        }

        if ok == 0 {
            INPUT_TEXT_LINE_HEIGHT_FALLBACK
        } else {
            tm.tmHeight as i32
        }
    }

    fn apply_list_rounded_corners(list_hwnd: HWND, width: i32, height: i32) {
        if width <= 0 || height <= 0 {
            return;
        }
        unsafe {
            let region =
                CreateRoundRectRgn(0, 0, width + 1, height + 1, LIST_RADIUS, LIST_RADIUS);
            SetWindowRgn(list_hwnd, region, 1);
        }
    }

    fn apply_help_tip_rounded_corners(help_tip_hwnd: HWND, width: i32, height: i32) {
        if width <= 0 || height <= 0 {
            return;
        }
        unsafe {
            let region = CreateRoundRectRgn(
                0,
                0,
                width + 1,
                height + 1,
                HELP_TIP_RADIUS,
                HELP_TIP_RADIUS,
            );
            SetWindowRgn(help_tip_hwnd, region, 1);
        }
    }

    fn paint_help_tip(hwnd: HWND, state: &OverlayShellState) {
        if state.help_tip_brush == 0 || state.help_tip_border_brush == 0 {
            return;
        }

        unsafe {
            let mut paint: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut paint);
            if hdc.is_null() {
                return;
            }

            let width = paint.rcPaint.right - paint.rcPaint.left;
            let height = paint.rcPaint.bottom - paint.rcPaint.top;
            if width <= 0 || height <= 0 {
                EndPaint(hwnd, &paint);
                return;
            }

            let bg_region =
                CreateRoundRectRgn(0, 0, width + 1, height + 1, HELP_TIP_RADIUS, HELP_TIP_RADIUS);
            FillRgn(hdc, bg_region, state.help_tip_brush as _);
            DeleteObject(bg_region as _);

            let border_region =
                CreateRoundRectRgn(0, 0, width + 1, height + 1, HELP_TIP_RADIUS, HELP_TIP_RADIUS);
            FrameRgn(hdc, border_region, state.help_tip_border_brush as _, 1, 1);
            DeleteObject(border_region as _);

            let old_font = if state.help_tip_font != 0 {
                SelectObject(hdc, state.help_tip_font as _)
            } else {
                std::ptr::null_mut()
            };
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, COLOR_HELP_TIP_TEXT);

            let mut text_rect = RECT {
                left: HELP_TIP_TEXT_PAD_X,
                top: 0,
                right: width - HELP_TIP_TEXT_PAD_X,
                bottom: height,
            };
            let text = to_wide(&help_hint_text(state));
            DrawTextW(
                hdc,
                text.as_ptr(),
                -1,
                &mut text_rect,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
            );

            if !old_font.is_null() {
                SelectObject(hdc, old_font);
            }
            EndPaint(hwnd, &paint);
        }
    }

    fn invalidate_list_row(list_hwnd: HWND, row: i32) {
        if row < 0 {
            return;
        }
        let mut rect: RECT = unsafe { std::mem::zeroed() };
        unsafe {
            let ok = SendMessageW(
                list_hwnd,
                LB_GETITEMRECT,
                row as usize,
                (&mut rect as *mut RECT) as LPARAM,
            );
            if ok != 0 {
                InvalidateRect(list_hwnd, &rect, 0);
            }
        }
    }

    fn set_help_hover_state(hwnd: HWND, state: &mut OverlayShellState, hovered: bool) {
        if state.help_hovered == hovered {
            return;
        }
        state.help_hovered = hovered;

        unsafe {
            InvalidateRect(state.help_hwnd, std::ptr::null(), 0);
        }

        if hovered {
            state.help_tip_visible = true;
            let wide = to_wide(&help_hint_text(state));
            unsafe {
                SetWindowTextW(state.help_tip_hwnd, wide.as_ptr());
            }
            layout_children(hwnd, state);
            unsafe {
                InvalidateRect(state.help_tip_hwnd, std::ptr::null(), 1);
            }
            return;
        }

        if state.help_tip_visible {
            state.help_tip_visible = false;
            let wide = to_wide(&help_hint_text(state));
            unsafe {
                SetWindowTextW(state.help_tip_hwnd, wide.as_ptr());
            }
            layout_children(hwnd, state);
        }
    }

    fn help_hint_text(state: &OverlayShellState) -> String {
        if state.help_config_path.trim().is_empty() {
            HOTKEY_HELP_TEXT_FALLBACK.to_string()
        } else {
            "Click to edit hotkey".to_string()
        }
    }

    fn open_help_config_file(state: &mut OverlayShellState) -> Result<(), String> {
        let cfg_path = state.help_config_path.trim().to_string();
        let target = if cfg_path.is_empty() {
            if let Ok(appdata) = std::env::var("APPDATA") {
                format!("{appdata}\\SwiftFind\\config.json")
            } else {
                return Err("APPDATA is not set; cannot locate config path.".to_string());
            }
        } else {
            cfg_path
        };

        let path = std::path::Path::new(&target);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create config directory: {e}"))?;
        }
        if !path.exists() {
            let cfg = crate::config::load(Some(path))
                .map_err(|e| format!("failed to load default config template: {e}"))?;
            crate::config::write_user_template(&cfg, path)
                .map_err(|e| format!("failed to create config file: {e}"))?;
        }

        std::process::Command::new("notepad")
            .arg(&target)
            .spawn()
            .map_err(|e| format!("failed to open config file: {e}"))?;

        state.status_is_error = false;
        state.help_tip_visible = false;
        let wide = to_wide("Opened hotkey config.");
        unsafe {
            SetWindowTextW(state.status_hwnd, wide.as_ptr());
            InvalidateRect(state.status_hwnd, std::ptr::null(), 1);
        }
        Ok(())
    }

    fn draw_panel_background(hwnd: HWND) {
        let Some(state) = state_for(hwnd) else {
            return;
        };

        let mut paint: PAINTSTRUCT = unsafe { std::mem::zeroed() };
        unsafe {
            let hdc = BeginPaint(hwnd, &mut paint);
            FillRect(hdc, &paint.rcPaint, state.panel_brush as _);

            let mut client_rect: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut client_rect);
            let width = client_rect.right - client_rect.left;
            let height = client_rect.bottom - client_rect.top;
            if width > 0 && height > 0 {
                let border_region =
                    CreateRoundRectRgn(0, 0, width + 1, height + 1, PANEL_RADIUS, PANEL_RADIUS);
                FrameRgn(hdc, border_region, state.border_brush as _, 1, 1);
                DeleteObject(border_region as _);
            }
            EndPaint(hwnd, &paint);
        }
    }

    fn apply_rounded_corners_hwnd(hwnd: HWND) {
        let mut rect: RECT = unsafe { std::mem::zeroed() };
        unsafe {
            GetClientRect(hwnd, &mut rect);
        }
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width <= 0 || height <= 0 {
            return;
        }

        unsafe {
            let region = CreateRoundRectRgn(
                0,
                0,
                width + 1,
                height + 1,
                PANEL_RADIUS,
                PANEL_RADIUS,
            );
            SetWindowRgn(hwnd, region, 1);
        }
    }

    fn cleanup_state_resources(state: &mut OverlayShellState) {
        unsafe {
            if state.input_font != 0 {
                DeleteObject(state.input_font as _);
            }
            if state.title_font != 0 {
                DeleteObject(state.title_font as _);
            }
            if state.meta_font != 0 {
                DeleteObject(state.meta_font as _);
            }
            if state.status_font != 0 {
                DeleteObject(state.status_font as _);
            }
            if state.help_tip_font != 0 {
                DeleteObject(state.help_tip_font as _);
            }
            if state.panel_brush != 0 {
                DeleteObject(state.panel_brush as _);
            }
            if state.border_brush != 0 {
                DeleteObject(state.border_brush as _);
            }
            if state.input_brush != 0 {
                DeleteObject(state.input_brush as _);
            }
            if state.results_brush != 0 {
                DeleteObject(state.results_brush as _);
            }
            if state.selection_brush != 0 {
                DeleteObject(state.selection_brush as _);
            }
            if state.selection_border_brush != 0 {
                DeleteObject(state.selection_border_brush as _);
            }
            if state.row_hover_brush != 0 {
                DeleteObject(state.row_hover_brush as _);
            }
            if state.row_separator_brush != 0 {
                DeleteObject(state.row_separator_brush as _);
            }
            if state.selection_accent_brush != 0 {
                DeleteObject(state.selection_accent_brush as _);
            }
            if state.icon_brush != 0 {
                DeleteObject(state.icon_brush as _);
            }
            if state.help_tip_brush != 0 {
                DeleteObject(state.help_tip_brush as _);
            }
            if state.help_tip_border_brush != 0 {
                DeleteObject(state.help_tip_border_brush as _);
            }
        }
        clear_icon_cache(state);
    }

    fn state_for(hwnd: HWND) -> Option<&'static mut OverlayShellState> {
        let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OverlayShellState };
        if state_ptr.is_null() {
            None
        } else {
            Some(unsafe { &mut *state_ptr })
        }
    }

    fn class_name_wide() -> &'static [u16] {
        static CLASS_NAME_WIDE: OnceLock<Vec<u16>> = OnceLock::new();
        CLASS_NAME_WIDE.get_or_init(|| to_wide(CLASS_NAME)).as_slice()
    }

    fn font_family_wide() -> &'static [u16] {
        static FONT_FAMILY_WIDE: OnceLock<Vec<u16>> = OnceLock::new();
        FONT_FAMILY_WIDE
            .get_or_init(|| {
                let family = std::env::var("SWIFTFIND_FONT_FAMILY")
                    .ok()
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty())
                    .unwrap_or_else(|| {
                        if register_private_geist_fonts() {
                            GEIST_FONT_FAMILY.to_string()
                        } else {
                            DEFAULT_FONT_FAMILY.to_string()
                        }
                    });
                to_wide(&family)
            })
            .as_slice()
    }

    fn register_private_geist_fonts() -> bool {
        static REGISTERED: OnceLock<bool> = OnceLock::new();
        *REGISTERED.get_or_init(|| {
            let mut candidates = Vec::new();
            if let Ok(dir) = std::env::var("SWIFTFIND_FONT_DIR") {
                let trimmed = dir.trim();
                if !trimmed.is_empty() {
                    candidates.push(PathBuf::from(trimmed));
                }
            }
            if let Ok(cwd) = std::env::current_dir() {
                candidates.push(cwd.join("apps/ui/src/fonts/Geist/otf"));
                candidates.push(cwd.join("fonts/Geist/otf"));
            }

            let files = [
                "Geist-Regular.otf",
                "Geist-Medium.otf",
                "Geist-SemiBold.otf",
                "Geist-Bold.otf",
            ];

            for base_dir in candidates {
                if !base_dir.is_dir() {
                    continue;
                }
                let mut loaded_any = false;
                for file_name in files {
                    let font_path = base_dir.join(file_name);
                    if !font_path.is_file() {
                        continue;
                    }
                    let font_wide = path_to_wide(&font_path);
                    let added = unsafe {
                        AddFontResourceExW(font_wide.as_ptr(), FR_PRIVATE, std::ptr::null())
                    };
                    if added > 0 {
                        loaded_any = true;
                    }
                }
                if loaded_any {
                    return true;
                }
            }
            false
        })
    }

    fn path_to_wide(path: &Path) -> Vec<u16> {
        path.to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect()
    }

    fn create_font(height: i32, weight: i32) -> isize {
        (unsafe {
            CreateFontW(
                height,
                0,
                0,
                0,
                weight,
                0,
                0,
                0,
                DEFAULT_CHARSET as u32,
                OUT_DEFAULT_PRECIS as u32,
                0,
                DEFAULT_QUALITY as u32,
                FF_DONTCARE as u32,
                font_family_wide().as_ptr(),
            )
        }) as isize
    }

    fn lerp_i32(from: i32, to: i32, t: f32) -> i32 {
        (from as f32 + (to - from) as f32 * t).round() as i32
    }

    fn ease_out(t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        1.0 - (1.0 - t) * (1.0 - t)
    }

    fn to_wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn to_wide_no_nul(value: &str) -> Vec<u16> {
        value.encode_utf16().collect()
    }
}

#[cfg(target_os = "windows")]
pub use imp::{NativeOverlayShell, OverlayEvent, OverlayRow};
