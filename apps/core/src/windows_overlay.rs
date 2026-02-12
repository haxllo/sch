#[cfg(target_os = "windows")]
mod imp {
    use std::ffi::c_void;
    use std::sync::OnceLock;
    use std::time::{Duration, Instant};

    use windows_sys::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::{
        BeginPaint, CreateFontW, CreateRoundRectRgn, CreateSolidBrush, DeleteObject, DrawTextW,
        EndPaint, FillRect, FrameRect, InvalidateRect, PAINTSTRUCT, ScreenToClient, SelectObject, SetBkColor,
        SetBkMode, SetTextColor, SetWindowRgn, DEFAULT_CHARSET, DEFAULT_QUALITY, DT_CENTER,
        DT_END_ELLIPSIS, DT_LEFT, DT_SINGLELINE, DT_VCENTER, FF_DONTCARE, FW_MEDIUM, FW_SEMIBOLD,
        OUT_DEFAULT_PRECIS, TRANSPARENT,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Controls::{DRAWITEMSTRUCT, MEASUREITEMSTRUCT, ODS_SELECTED, EM_SETSEL};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SetFocus, VK_DOWN, VK_ESCAPE, VK_RETURN, VK_UP,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, CreateWindowExW, DefWindowProcW, DispatchMessageW,
        GetClientRect, GetCursorPos, GetForegroundWindow, GetMessageW, GetParent, GetSystemMetrics,
        GetWindowLongPtrW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
        IsChild, LB_ADDSTRING, LB_GETCOUNT, LB_GETCURSEL, LB_GETTEXT, LB_GETTEXTLEN,
        LB_GETTOPINDEX, LB_ITEMFROMPOINT, LB_RESETCONTENT, LB_SETCURSEL, LB_SETTABSTOPS,
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
        WM_NCDESTROY, WM_PAINT, WM_SETFONT, WM_SIZE, WM_TIMER, WNDCLASSW, WS_BORDER, WS_CHILD,
        WS_CLIPCHILDREN, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_TABSTOP,
        WS_VISIBLE, WS_VSCROLL,
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
    const PANEL_MARGIN_TOP: i32 = 12;
    const PANEL_MARGIN_BOTTOM: i32 = 8;
    const INPUT_HEIGHT: i32 = 36;
    const STATUS_HEIGHT: i32 = 18;
    const ROW_GAP: i32 = 8;
    const ROW_HEIGHT: i32 = 44;
    const MAX_VISIBLE_ROWS: usize = 6;
    const ROW_INSET_X: i32 = 10;
    const ROW_ICON_SIZE: i32 = 22;
    const ROW_ICON_GAP: i32 = 10;
    const ROW_TEXT_TOP_PAD: i32 = 6;
    const ROW_TEXT_BOTTOM_PAD: i32 = 5;

    const CONTROL_ID_INPUT: usize = 1001;
    const CONTROL_ID_LIST: usize = 1002;
    const CONTROL_ID_STATUS: usize = 1003;

    const SWIFTFIND_WM_ESCAPE: u32 = WM_APP + 1;
    const SWIFTFIND_WM_QUERY_CHANGED: u32 = WM_APP + 2;
    const SWIFTFIND_WM_MOVE_UP: u32 = WM_APP + 3;
    const SWIFTFIND_WM_MOVE_DOWN: u32 = WM_APP + 4;
    const SWIFTFIND_WM_SUBMIT: u32 = WM_APP + 5;

    const TIMER_SELECTION_ANIM: usize = 0xBEEF;
    const TIMER_SCROLL_ANIM: usize = 0xBEF0;

    const OVERLAY_ANIM_MS: u32 = 150;
    const RESULTS_ANIM_MS: u32 = 150;
    const SELECTION_ANIM_MS: u64 = 90;
    const SCROLL_ANIM_MS: u64 = 120;
    const ANIM_FRAME_MS: u64 = 8;
    const WHEEL_LINES_PER_NOTCH: i32 = 3;

    // Typography tokens.
    const FONT_INPUT_HEIGHT: i32 = -22;
    const FONT_TITLE_HEIGHT: i32 = -15;
    const FONT_META_HEIGHT: i32 = -13;
    const FONT_STATUS_HEIGHT: i32 = -13;

    // Visual tokens.
    const COLOR_PANEL_BG: u32 = 0x001C1C1C; // #1C1C1C (BGR)
    const COLOR_PANEL_BORDER: u32 = 0x003E3E3E; // #3E3E3E (BGR)
    const COLOR_INPUT_BG: u32 = 0x00191919;
    const COLOR_RESULTS_BG: u32 = 0x00151515;
    const COLOR_TEXT_PRIMARY: u32 = 0x00F2EEE9;
    const COLOR_TEXT_SECONDARY: u32 = 0x00B4AEA8;
    const COLOR_TEXT_ERROR: u32 = 0x007779E8;
    const COLOR_SELECTION: u32 = 0x00503E31;
    const COLOR_SELECTION_BORDER: u32 = 0x00614B39;
    const COLOR_ROW_HOVER: u32 = 0x00382D24;
    const COLOR_ROW_SEPARATOR: u32 = 0x002B2B2B;
    const COLOR_SELECTION_ACCENT: u32 = 0x007A5D45;
    const COLOR_ICON_BG: u32 = 0x00312A24;
    const COLOR_ICON_TEXT: u32 = 0x00D8D0C8;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum OverlayEvent {
        Hotkey(i32),
        QueryChanged(String),
        MoveSelection(i32),
        Submit,
        Escape,
    }

    pub struct NativeOverlayShell {
        hwnd: HWND,
    }

    #[derive(Default)]
    struct OverlayShellState {
        edit_hwnd: HWND,
        list_hwnd: HWND,
        status_hwnd: HWND,

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

        status_is_error: bool,
        results_visible: bool,

        selection_prev: i32,
        selection_current: i32,
        selection_anim_start: Option<Instant>,
        hover_index: i32,

        scroll_from_top: i32,
        scroll_to_top: i32,
        scroll_anim_start: Option<Instant>,
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
            }
        }

        pub fn clear_query_text(&self) {
            if let Some(state) = state_for(self.hwnd) {
                unsafe {
                    SetWindowTextW(state.edit_hwnd, to_wide("").as_ptr());
                }
            }
        }

        pub fn set_results(&self, rows: &[String], selected_index: usize) {
            if let Some(state) = state_for(self.hwnd) {
                unsafe {
                    SendMessageW(state.list_hwnd, LB_RESETCONTENT, 0, 0);
                }

                for row in rows {
                    let wide = to_wide(row);
                    unsafe {
                        SendMessageW(state.list_hwnd, LB_ADDSTRING, 0, wide.as_ptr() as LPARAM);
                    }
                }

                if rows.is_empty() {
                    self.collapse_results();
                } else {
                    self.expand_results(rows.len());
                    self.set_selected_index(selected_index);
                }

                if rows.is_empty() {
                    state.selection_prev = -1;
                    state.selection_current = -1;
                    state.selection_anim_start = None;
                    state.scroll_anim_start = None;
                    state.scroll_from_top = 0;
                    state.scroll_to_top = 0;
                }
            }
        }

        pub fn set_selected_index(&self, selected_index: usize) {
            let Some(state) = state_for(self.hwnd) else {
                return;
            };

            let count = unsafe { SendMessageW(state.list_hwnd, LB_GETCOUNT, 0, 0) };
            if count <= 0 {
                return;
            }

            let clamped = selected_index.min((count as usize).saturating_sub(1));
            let previous = unsafe { SendMessageW(state.list_hwnd, LB_GETCURSEL, 0, 0) };
            unsafe {
                SendMessageW(state.list_hwnd, LB_SETCURSEL, clamped, 0);
            }

            if previous != clamped as isize {
                state.selection_prev = previous as i32;
                state.selection_current = clamped as i32;
                state.selection_anim_start = Some(Instant::now());
                begin_scroll_animation(self.hwnd, state, clamped as i32, count as i32);
                unsafe {
                    SetTimer(self.hwnd, TIMER_SELECTION_ANIM, 16, None);
                    InvalidateRect(state.list_hwnd, std::ptr::null(), 1);
                }
            }
        }

        pub fn selected_index(&self) -> Option<usize> {
            let state = state_for(self.hwnd)?;
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
                unsafe {
                    ShowWindow(state.list_hwnd, SW_HIDE);
                }
            }
        }

        fn expand_results(&self, result_count: usize) {
            let rows = result_count.min(MAX_VISIBLE_ROWS) as i32;
            let target_height = COMPACT_HEIGHT + ROW_GAP + rows * ROW_HEIGHT;

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
            let width = rect.right - rect.left;
            let current_height = rect.bottom - rect.top;
            let top = rect.top;
            let left = rect.left;

            if current_height == target_height {
                return;
            }

            if duration_ms == 0 {
                unsafe {
                    SetWindowPos(
                        self.hwnd,
                        HWND_TOPMOST,
                        left,
                        top,
                        width,
                        target_height,
                        SWP_NOACTIVATE,
                    );
                    SetLayeredWindowAttributes(self.hwnd, 0, 255, LWA_ALPHA);
                }
                return;
            }

            let steps = (duration_ms / ANIM_FRAME_MS as u32).max(1);
            let expanding = target_height > current_height;
            let start_alpha = if expanding { 236 } else { 255 };
            let end_alpha = if expanding { 255 } else { 236 };
            for step in 1..=steps {
                let t = step as f32 / steps as f32;
                let eased = ease_out(t);
                let h = lerp_i32(current_height, target_height, eased);
                let alpha = lerp_i32(start_alpha, end_alpha, eased) as u8;
                unsafe {
                    SetWindowPos(
                        self.hwnd,
                        HWND_TOPMOST,
                        left,
                        top,
                        width,
                        h,
                        SWP_NOACTIVATE,
                    );
                    SetLayeredWindowAttributes(self.hwnd, 0, alpha, LWA_ALPHA);
                }
                std::thread::sleep(Duration::from_millis(ANIM_FRAME_MS));
            }

            unsafe {
                SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    left,
                    top,
                    width,
                    target_height,
                    SWP_NOACTIVATE,
                );
                SetLayeredWindowAttributes(self.hwnd, 0, 255, LWA_ALPHA);
            }
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

            unsafe {
                SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    start_left,
                    final_top,
                    start_width,
                    start_height,
                    SWP_NOACTIVATE,
                );
                SetLayeredWindowAttributes(self.hwnd, 0, 0, LWA_ALPHA);
                ShowWindow(self.hwnd, SW_SHOW);
            }

            let steps = (OVERLAY_ANIM_MS / ANIM_FRAME_MS as u32).max(1);
            for step in 1..=steps {
                let t = step as f32 / steps as f32;
                let eased = ease_out(t);
                let w = lerp_i32(start_width, final_width, eased);
                let h = lerp_i32(start_height, final_height, eased);
                let x = final_left + (final_width - w) / 2;
                let alpha = lerp_i32(0, 255, eased) as u8;
                unsafe {
                    SetWindowPos(self.hwnd, HWND_TOPMOST, x, final_top, w, h, SWP_NOACTIVATE);
                    SetLayeredWindowAttributes(self.hwnd, 0, alpha, LWA_ALPHA);
                }
                std::thread::sleep(Duration::from_millis(ANIM_FRAME_MS));
            }

            unsafe {
                SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    final_left,
                    final_top,
                    final_width,
                    final_height,
                    SWP_NOACTIVATE,
                );
                SetLayeredWindowAttributes(self.hwnd, 0, 255, LWA_ALPHA);
            }
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

            let steps = (OVERLAY_ANIM_MS / ANIM_FRAME_MS as u32).max(1);
            for step in 1..=steps {
                let t = step as f32 / steps as f32;
                let eased = ease_out(t);
                let w = lerp_i32(final_width, end_width, eased);
                let h = lerp_i32(final_height, end_height, eased);
                let x = final_left + (final_width - w) / 2;
                let alpha = lerp_i32(255, 0, eased) as u8;
                unsafe {
                    SetWindowPos(self.hwnd, HWND_TOPMOST, x, final_top, w, h, SWP_NOACTIVATE);
                    SetLayeredWindowAttributes(self.hwnd, 0, alpha, LWA_ALPHA);
                }
                std::thread::sleep(Duration::from_millis(ANIM_FRAME_MS));
            }

            unsafe {
                ShowWindow(self.hwnd, SW_HIDE);
                SetLayeredWindowAttributes(self.hwnd, 0, 255, LWA_ALPHA);
                SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    final_left,
                    final_top,
                    final_width,
                    final_height,
                    SWP_NOACTIVATE,
                );
            }
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

                    state.input_font = create_font(FONT_INPUT_HEIGHT, FW_SEMIBOLD as i32);
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
                                | WS_VSCROLL
                                | WS_BORDER
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
                            to_wide("Type to search apps and files.").as_ptr(),
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

                        let tab_stop = [268_i32];
                        SendMessageW(state.list_hwnd, LB_SETTABSTOPS, 1, tab_stop.as_ptr() as LPARAM);

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
                    state.selection_prev = -1;
                    state.selection_current = -1;
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
                            SetBkColor(wparam as _, COLOR_PANEL_BG);
                            SetBkMode(wparam as _, TRANSPARENT as i32);
                        }
                        return state.panel_brush;
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
            WM_PAINT => {
                draw_panel_background(hwnd);
                0
            }
            WM_TIMER => {
                if wparam == TIMER_SELECTION_ANIM {
                    if let Some(state) = state_for(hwnd) {
                        let running = selection_animation_running(state);
                        unsafe {
                            InvalidateRect(state.list_hwnd, std::ptr::null(), 1);
                        }
                        if !running {
                            unsafe {
                                KillTimer(hwnd, TIMER_SELECTION_ANIM);
                            }
                        }
                    }
                }
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
                    state.hover_index = next_hover;
                    unsafe {
                        InvalidateRect(hwnd, std::ptr::null(), 1);
                    }
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
                        begin_scroll_animation_to_top(parent, state, target_top);
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
        unsafe { CallWindowProcW(prev_proc, hwnd, message, wparam, lparam) }
    }

    fn draw_list_row(hwnd: HWND, dis: &mut DRAWITEMSTRUCT) {
        if dis.itemID == u32::MAX {
            return;
        }

        let Some(state) = state_for(hwnd) else {
            return;
        };

        let item_index = dis.itemID as i32;
        let row_text = listbox_row_text(state.list_hwnd, item_index);
        let row = parse_result_row(&row_text);

        let progress = selection_progress(state);
        let selected_flag = (dis.itemState & ODS_SELECTED as u32) != 0;
        let hovered = state.hover_index == item_index;
        let mut row_bg = COLOR_RESULTS_BG;

        if selected_flag {
            row_bg = blend_color(COLOR_RESULTS_BG, COLOR_SELECTION, progress);
        } else if state.selection_prev == item_index && progress < 1.0 {
            row_bg = blend_color(COLOR_SELECTION, COLOR_RESULTS_BG, progress);
        } else if hovered {
            row_bg = COLOR_ROW_HOVER;
        }

        unsafe {
            let row_brush = CreateSolidBrush(row_bg);
            FillRect(dis.hDC, &dis.rcItem, row_brush);
            DeleteObject(row_brush as _);

            let icon_rect = RECT {
                left: dis.rcItem.left + ROW_INSET_X,
                top: dis.rcItem.top + (ROW_HEIGHT - ROW_ICON_SIZE) / 2,
                right: dis.rcItem.left + ROW_INSET_X + ROW_ICON_SIZE,
                bottom: dis.rcItem.top + (ROW_HEIGHT - ROW_ICON_SIZE) / 2 + ROW_ICON_SIZE,
            };
            let icon_brush = CreateSolidBrush(COLOR_ICON_BG);
            FillRect(dis.hDC, &icon_rect, icon_brush);
            DeleteObject(icon_brush as _);
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
            let mut title_rect = RECT {
                left: text_left,
                top: dis.rcItem.top + ROW_TEXT_TOP_PAD,
                right: dis.rcItem.right - ROW_INSET_X,
                bottom: dis.rcItem.top + ROW_HEIGHT / 2,
            };
            DrawTextW(
                dis.hDC,
                to_wide(&row.title).as_ptr(),
                -1,
                &mut title_rect,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
            );

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

            if selected_flag {
                let border_brush = CreateSolidBrush(COLOR_SELECTION_BORDER);
                FrameRect(dis.hDC, &dis.rcItem, border_brush);
                DeleteObject(border_brush as _);

                let accent_rect = RECT {
                    left: dis.rcItem.left + 1,
                    top: dis.rcItem.top + 1,
                    right: dis.rcItem.left + 4,
                    bottom: dis.rcItem.bottom - 1,
                };
                let accent_brush = CreateSolidBrush(COLOR_SELECTION_ACCENT);
                FillRect(dis.hDC, &accent_rect, accent_brush);
                DeleteObject(accent_brush as _);
            }

            let separator_rect = RECT {
                left: dis.rcItem.left + ROW_INSET_X,
                top: dis.rcItem.bottom - 1,
                right: dis.rcItem.right - ROW_INSET_X,
                bottom: dis.rcItem.bottom,
            };
            let separator_brush = CreateSolidBrush(COLOR_ROW_SEPARATOR);
            FillRect(dis.hDC, &separator_rect, separator_brush);
            DeleteObject(separator_brush as _);

            SelectObject(dis.hDC, old_font);
        }
    }

    struct ResultRow {
        kind: String,
        title: String,
        path: String,
    }

    fn parse_result_row(row: &str) -> ResultRow {
        if let Some((kind, rest)) = row.split_once('\u{1f}') {
            if let Some((title, path)) = rest.split_once('\u{1f}') {
                return ResultRow {
                    kind: kind.to_string(),
                    title: title.to_string(),
                    path: path.to_string(),
                };
            }
        }
        if let Some((title, path)) = row.split_once('\t') {
            return ResultRow {
                kind: "file".to_string(),
                title: title.to_string(),
                path: path.to_string(),
            };
        }
        ResultRow {
            kind: "file".to_string(),
            title: row.to_string(),
            path: String::new(),
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

    fn listbox_row_text(list_hwnd: HWND, index: i32) -> String {
        if index < 0 {
            return String::new();
        }

        let text_len = unsafe { SendMessageW(list_hwnd, LB_GETTEXTLEN, index as usize, 0) };
        if text_len <= 0 {
            return String::new();
        }

        let mut buf = vec![0_u16; (text_len as usize) + 2];
        unsafe {
            SendMessageW(
                list_hwnd,
                LB_GETTEXT,
                index as usize,
                buf.as_mut_ptr() as LPARAM,
            );
        }

        let end = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..end])
    }

    fn selection_animation_running(state: &mut OverlayShellState) -> bool {
        let Some(start) = state.selection_anim_start else {
            return false;
        };

        if start.elapsed().as_millis() as u64 >= SELECTION_ANIM_MS {
            state.selection_anim_start = None;
            state.selection_prev = -1;
            return false;
        }
        true
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
        if target_top == current_top {
            state.scroll_anim_start = None;
            state.scroll_from_top = current_top;
            state.scroll_to_top = target_top;
            return;
        }

        state.scroll_from_top = current_top;
        state.scroll_to_top = target_top;
        state.scroll_anim_start = Some(Instant::now());
        unsafe {
            SetTimer(overlay_hwnd, TIMER_SCROLL_ANIM, ANIM_FRAME_MS as u32, None);
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

    fn selection_progress(state: &OverlayShellState) -> f32 {
        let Some(start) = state.selection_anim_start else {
            return 1.0;
        };
        let elapsed = start.elapsed().as_millis() as u64;
        let t = (elapsed as f32 / SELECTION_ANIM_MS as f32).clamp(0.0, 1.0);
        ease_out(t)
    }

    fn blend_color(from: u32, to: u32, t: f32) -> u32 {
        let t = t.clamp(0.0, 1.0);
        let fb = ((from >> 16) & 0xFF) as f32;
        let fg = ((from >> 8) & 0xFF) as f32;
        let fr = (from & 0xFF) as f32;

        let tb = ((to >> 16) & 0xFF) as f32;
        let tg = ((to >> 8) & 0xFF) as f32;
        let tr = (to & 0xFF) as f32;

        let b = fb + (tb - fb) * t;
        let g = fg + (tg - fg) * t;
        let r = fr + (tr - fr) * t;

        ((b as u32) << 16) | ((g as u32) << 8) | (r as u32)
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
        let input_top = PANEL_MARGIN_TOP;
        let status_top = COMPACT_HEIGHT - PANEL_MARGIN_BOTTOM - STATUS_HEIGHT;

        let list_top = COMPACT_HEIGHT + ROW_GAP / 2;
        let list_height = (height - list_top - PANEL_MARGIN_BOTTOM).max(0);

        unsafe {
            MoveWindow(
                state.edit_hwnd,
                PANEL_MARGIN_X,
                input_top,
                input_width,
                INPUT_HEIGHT,
                1,
            );
            MoveWindow(
                state.status_hwnd,
                PANEL_MARGIN_X,
                status_top,
                input_width,
                STATUS_HEIGHT,
                1,
            );
            MoveWindow(
                state.list_hwnd,
                PANEL_MARGIN_X,
                list_top,
                input_width,
                list_height,
                1,
            );
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
            FrameRect(hdc, &paint.rcPaint, state.border_brush as _);
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
                to_wide("Segoe UI").as_ptr(),
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
pub use imp::{NativeOverlayShell, OverlayEvent};
