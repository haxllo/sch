#[cfg(target_os = "windows")]
mod imp {
    use std::ffi::c_void;
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;
    use std::time::Instant;

    use windows_sys::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::{
        AddFontResourceExW, BeginPaint, CreateFontW, CreateRoundRectRgn, CreateSolidBrush, DeleteObject, DrawTextW,
        EndPaint, FillRect, FillRgn, FrameRgn, FrameRect, GetDC, InvalidateRect, PAINTSTRUCT, ReleaseDC, ScreenToClient, SelectObject, SetBkColor,
        SetBkMode, SetTextColor, SetWindowRgn, DEFAULT_CHARSET, DEFAULT_QUALITY, DT_CENTER,
        DT_END_ELLIPSIS, DT_LEFT, DT_SINGLELINE, DT_VCENTER, FF_DONTCARE, FW_MEDIUM, FW_SEMIBOLD,
        FR_PRIVATE, OPAQUE, OUT_DEFAULT_PRECIS, TRANSPARENT,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Controls::{DRAWITEMSTRUCT, EM_SETSEL, MEASUREITEMSTRUCT, ODS_SELECTED};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SetFocus, VK_DOWN, VK_ESCAPE, VK_RETURN, VK_UP,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, CreateWindowExW, DefWindowProcW, DispatchMessageW,
        GetClientRect, GetCursorPos, GetForegroundWindow, GetMessageW, GetParent, GetSystemMetrics,
        GetWindowLongPtrW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
        IsChild, LB_ADDSTRING, LB_GETCOUNT, LB_GETCURSEL, LB_GETITEMRECT, LB_GETTOPINDEX,
        LB_ITEMFROMPOINT, LB_RESETCONTENT, LB_SETCURSEL,
        LB_SETTOPINDEX, LoadCursorW,
        MoveWindow, PostMessageW, PostQuitMessage, RegisterClassW, SendMessageW,
        SetForegroundWindow, SetLayeredWindowAttributes, SetTimer, SetWindowLongPtrW, SetWindowPos,
        SetWindowTextW, ShowWindow, TranslateMessage, CREATESTRUCTW, CS_DROPSHADOW,
        CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, EN_CHANGE, ES_AUTOHSCROLL, GWLP_USERDATA,
        GWLP_WNDPROC, HMENU, HWND_TOPMOST, IDC_ARROW, KillTimer, LBN_DBLCLK, LBS_HASSTRINGS,
        LBS_NOINTEGRALHEIGHT, LBS_NOTIFY, LBS_OWNERDRAWFIXED, LWA_ALPHA, MSG, SM_CXSCREEN,
        SM_CYSCREEN, SW_HIDE, SW_SHOW, SWP_NOACTIVATE, WM_APP, WM_CLOSE, WM_COMMAND, WM_CREATE,
        WM_CTLCOLORLISTBOX, WM_CTLCOLOREDIT, WM_CTLCOLORSTATIC, WM_DESTROY, WM_DRAWITEM,
        WM_HOTKEY, WM_KEYDOWN, WM_MEASUREITEM, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCCREATE,
        WM_NCDESTROY, WM_PAINT, WM_SETFONT, WM_SIZE, WM_TIMER, WM_LBUTTONUP, WM_ACTIVATE,
        WNDCLASSW, WS_CHILD,
        WS_CLIPCHILDREN, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_TABSTOP,
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
    const ROW_HEIGHT: i32 = 50;
    const LIST_RADIUS: i32 = 16;
    const MAX_VISIBLE_ROWS: usize = 6;
    const ROW_INSET_X: i32 = 10;
    const ROW_ICON_SIZE: i32 = 22;
    const ROW_ICON_GAP: i32 = 10;
    const ROW_TEXT_TOP_PAD: i32 = 7;
    const ROW_TEXT_BOTTOM_PAD: i32 = 6;
    const ROW_VERTICAL_INSET: i32 = 1;
    const ROW_ACTIVE_RADIUS: i32 = 10;

    const CONTROL_ID_INPUT: usize = 1001;
    const CONTROL_ID_LIST: usize = 1002;
    const CONTROL_ID_STATUS: usize = 1003;

    const SWIFTFIND_WM_ESCAPE: u32 = WM_APP + 1;
    const SWIFTFIND_WM_QUERY_CHANGED: u32 = WM_APP + 2;
    const SWIFTFIND_WM_MOVE_UP: u32 = WM_APP + 3;
    const SWIFTFIND_WM_MOVE_DOWN: u32 = WM_APP + 4;
    const SWIFTFIND_WM_SUBMIT: u32 = WM_APP + 5;
    const EM_SETCUEBANNER: u32 = 0x1501;
    const EM_GETRECT: u32 = 0x00B2;

    const TIMER_SCROLL_ANIM: usize = 0xBEF0;
    const TIMER_WINDOW_ANIM: usize = 0xBEF1;

    const OVERLAY_ANIM_MS: u32 = 150;
    const OVERLAY_HIDE_ANIM_MS: u32 = 115;
    const RESULTS_ANIM_MS: u32 = 150;
    const SCROLL_ANIM_MS: u64 = 120;
    const ANIM_FRAME_MS: u64 = 8;
    const WHEEL_LINES_PER_NOTCH: i32 = 3;

    // Typography tokens.
    const FONT_INPUT_HEIGHT: i32 = -19;
    const FONT_TITLE_HEIGHT: i32 = -15;
    const FONT_META_HEIGHT: i32 = -13;
    const FONT_STATUS_HEIGHT: i32 = -13;

    // Visual tokens.
    const COLOR_PANEL_BG: u32 = 0x00101010;
    const COLOR_PANEL_BORDER: u32 = 0x002A2A2A;
    const COLOR_INPUT_BG: u32 = 0x00141414;
    const COLOR_RESULTS_BG: u32 = 0x00101010;
    const COLOR_TEXT_PRIMARY: u32 = 0x00F4F4F4;
    const COLOR_TEXT_SECONDARY: u32 = 0x00B8B8B8;
    const COLOR_TEXT_ERROR: u32 = 0x00E8E8E8;
    const COLOR_SELECTION: u32 = 0x00222222;
    const COLOR_SELECTION_BORDER: u32 = 0x00383838;
    const COLOR_ROW_HOVER: u32 = 0x00080808;
    const COLOR_ROW_SEPARATOR: u32 = 0x00161616;
    const COLOR_SELECTION_ACCENT: u32 = 0x004A4A4A;
    const COLOR_ICON_BG: u32 = 0x001D1D1D;
    const COLOR_ICON_TEXT: u32 = 0x00F0F0F0;
    const DEFAULT_FONT_FAMILY: &str = "Segoe UI Variable Text";
    const GEIST_FONT_FAMILY: &str = "Geist";

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
    }

    pub struct NativeOverlayShell {
        hwnd: HWND,
    }

    #[derive(Default)]
    struct OverlayShellState {
        edit_hwnd: HWND,
        list_hwnd: HWND,
        status_hwnd: HWND,
        cue_text_wide: Vec<u16>,

        edit_prev_proc: isize,
        list_prev_proc: isize,

        input_font: isize,
        title_font: isize,
        meta_font: isize,
        status_font: isize,

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

        status_is_error: bool,
        results_visible: bool,

        hover_index: i32,

        scroll_from_top: i32,
        scroll_to_top: i32,
        scroll_anim_start: Option<Instant>,

        window_anim: Option<WindowAnimation>,
        rows: Vec<OverlayRow>,
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
                    WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
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
                state.status_is_error = message.to_ascii_lowercase().contains("error");
                let wide = to_wide(message);
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

        pub fn clear_query_text(&self) {
            if let Some(state) = state_for(self.hwnd) {
                unsafe {
                    SetWindowTextW(state.edit_hwnd, to_wide("").as_ptr());
                }
            }
        }

        pub fn set_results(&self, rows: &[OverlayRow], selected_index: usize) {
            if let Some(state) = state_for(self.hwnd) {
                state.rows.clear();
                state.rows.extend_from_slice(rows);
                state.hover_index = -1;
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

                if rows.is_empty() {
                    self.collapse_results();
                } else {
                    self.expand_results(rows.len());
                    self.set_selected_index_internal(selected_index, false);
                }

                if rows.is_empty() {
                    state.scroll_anim_start = None;
                    state.scroll_from_top = 0;
                    state.scroll_to_top = 0;
                    state.hover_index = -1;
                }
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
                    HWND_TOPMOST,
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
                    SendMessageW(state.list_hwnd, LB_SETTOPINDEX, 0, 0);
                }
            }
        }

        fn expand_results(&self, result_count: usize) {
            let rows = result_count.min(MAX_VISIBLE_ROWS) as i32;
            let list_top = INPUT_TOP + INPUT_HEIGHT + INPUT_TO_LIST_GAP;
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
                unsafe {
                    ShowWindow(state.list_hwnd, SW_HIDE);
                }
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

                    state.input_font = create_font(FONT_INPUT_HEIGHT, FW_MEDIUM as i32);
                    state.title_font = create_font(FONT_TITLE_HEIGHT, FW_SEMIBOLD as i32);
                    state.meta_font = create_font(FONT_META_HEIGHT, FW_MEDIUM as i32);
                    state.status_font = create_font(FONT_STATUS_HEIGHT, FW_MEDIUM as i32);

                    state.edit_hwnd = unsafe {
                        CreateWindowExW(
                            0,
                            to_wide(INPUT_CLASS).as_ptr(),
                            to_wide("").as_ptr(),
                            WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL as u32,
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
                            to_wide("").as_ptr(),
                            WS_CHILD | WS_VISIBLE,
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

                    unsafe {
                        SendMessageW(state.edit_hwnd, WM_SETFONT, state.input_font as usize, 1);
                        SendMessageW(state.list_hwnd, WM_SETFONT, state.meta_font as usize, 1);
                        SendMessageW(state.status_hwnd, WM_SETFONT, state.status_font as usize, 1);
                        state.cue_text_wide = to_wide("Type to search");
                        SendMessageW(
                            state.edit_hwnd,
                            EM_SETCUEBANNER,
                            1,
                            state.cue_text_wide.as_ptr() as LPARAM,
                        );

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

                        ShowWindow(state.list_hwnd, SW_HIDE);
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
                unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
            }
            WM_CTLCOLORSTATIC => {
                if let Some(state) = state_for(hwnd) {
                    let target = lparam as HWND;
                    if target == state.status_hwnd {
                        let color = if state.status_is_error {
                            COLOR_TEXT_ERROR
                        } else {
                            COLOR_TEXT_SECONDARY
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
                        cleanup_state_resources(&*state_ptr);
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
            unsafe {
                GetClientRect(edit_hwnd, &mut text_rect);
            }
            text_rect.left += 10;
            text_rect.right -= 10;
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
                DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
            );
            SelectObject(hdc, old_font);
            ReleaseDC(edit_hwnd, hdc);
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
            });

        let selected_flag = (dis.itemState & ODS_SELECTED as u32) != 0;
        let hovered = state.hover_index == item_index;
        let row_active = selected_flag || hovered;

        unsafe {
            FillRect(dis.hDC, &dis.rcItem, state.results_brush as _);
            if row_active {
                let row_rect = RECT {
                    left: dis.rcItem.left + 2,
                    top: dis.rcItem.top + ROW_VERTICAL_INSET,
                    right: dis.rcItem.right - 2,
                    bottom: dis.rcItem.bottom - ROW_VERTICAL_INSET,
                };
                let region = CreateRoundRectRgn(
                    row_rect.left,
                    row_rect.top,
                    row_rect.right,
                    row_rect.bottom,
                    ROW_ACTIVE_RADIUS,
                    ROW_ACTIVE_RADIUS,
                );
                FillRgn(dis.hDC, region, state.row_hover_brush as _);
                DeleteObject(region as _);
            }

            let icon_rect = RECT {
                left: dis.rcItem.left + ROW_INSET_X,
                top: dis.rcItem.top + (ROW_HEIGHT - ROW_ICON_SIZE) / 2,
                right: dis.rcItem.left + ROW_INSET_X + ROW_ICON_SIZE,
                bottom: dis.rcItem.top + (ROW_HEIGHT - ROW_ICON_SIZE) / 2 + ROW_ICON_SIZE,
            };
            FillRect(dis.hDC, &icon_rect, state.icon_brush as _);
            FrameRect(dis.hDC, &icon_rect, state.border_brush as _);

            let old_font = SelectObject(dis.hDC, state.title_font as _);
            SetBkMode(dis.hDC, TRANSPARENT as i32);
            SetTextColor(dis.hDC, COLOR_TEXT_PRIMARY);

            let mut icon_text_rect = icon_rect;
            SetTextColor(dis.hDC, COLOR_ICON_TEXT);
            DrawTextW(
                dis.hDC,
                to_wide(icon_glyph_for_kind(&row.kind)).as_ptr(),
                -1,
                &mut icon_text_rect,
                DT_CENTER | DT_SINGLELINE | DT_VCENTER,
            );

            SetTextColor(dis.hDC, COLOR_TEXT_PRIMARY);
            let text_left = icon_rect.right + ROW_ICON_GAP;
            let has_meta = !row.path.trim().is_empty();
            let mut title_rect = RECT {
                left: text_left,
                top: if has_meta {
                    dis.rcItem.top + ROW_TEXT_TOP_PAD
                } else {
                    dis.rcItem.top
                },
                right: dis.rcItem.right - ROW_INSET_X,
                bottom: if has_meta {
                    dis.rcItem.top + ROW_HEIGHT / 2
                } else {
                    dis.rcItem.bottom
                },
            };
            DrawTextW(
                dis.hDC,
                to_wide(&row.title).as_ptr(),
                -1,
                &mut title_rect,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
            );

            if has_meta {
                SelectObject(dis.hDC, state.meta_font as _);
                SetTextColor(dis.hDC, COLOR_TEXT_SECONDARY);
                let mut path_rect = RECT {
                    left: text_left,
                    top: dis.rcItem.top + ROW_HEIGHT / 2 - 1,
                    right: dis.rcItem.right - ROW_INSET_X,
                    bottom: dis.rcItem.bottom - ROW_TEXT_BOTTOM_PAD,
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

    fn icon_glyph_for_kind(kind: &str) -> &'static str {
        if kind.eq_ignore_ascii_case("app") {
            "A"
        } else if kind.eq_ignore_ascii_case("folder") {
            "D"
        } else {
            "F"
        }
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
            }
            return false;
        }

        true
    }

    fn apply_window_state(hwnd: HWND, left: i32, top: i32, width: i32, height: i32, alpha: u8) {
        unsafe {
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
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
            SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
            ShowWindow(hwnd, SW_HIDE);
        }
    }


    fn layout_children(hwnd: HWND, state: &OverlayShellState) {
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
        let status_len = unsafe { GetWindowTextLengthW(state.status_hwnd) };
        let status_visible = status_len > 0;
        // Keep input exactly centered in compact mode and stable across states.
        let input_top = INPUT_TOP.max(0);
        let status_top = COMPACT_HEIGHT - PANEL_MARGIN_BOTTOM - STATUS_HEIGHT;

        let list_top = input_top + INPUT_HEIGHT + INPUT_TO_LIST_GAP;
        let list_left = PANEL_MARGIN_X + 1;
        let list_width = (input_width - 2).max(0);
        let list_height = (height - list_top - PANEL_MARGIN_X - 1).max(0);

        unsafe {
            MoveWindow(
                state.edit_hwnd,
                PANEL_MARGIN_X,
                input_top,
                input_width,
                INPUT_HEIGHT,
                1,
            );
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

    fn cleanup_state_resources(state: &OverlayShellState) {
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
        }
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
}

#[cfg(target_os = "windows")]
pub use imp::{NativeOverlayShell, OverlayEvent, OverlayRow};
