#[cfg(target_os = "windows")]
mod imp {
    use std::ffi::c_void;
    use std::sync::OnceLock;

    use windows_sys::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::{
        CreateFontW, CreateRoundRectRgn, CreateSolidBrush, DeleteObject, SetBkColor, SetBkMode,
        SetTextColor, SetWindowRgn, DEFAULT_CHARSET, DEFAULT_QUALITY, FF_DONTCARE, FW_MEDIUM,
        FW_SEMIBOLD, OUT_DEFAULT_PRECIS, TRANSPARENT,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Controls::EM_SETSEL;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SetFocus, VK_DOWN, VK_ESCAPE, VK_RETURN, VK_UP,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, CreateWindowExW, DefWindowProcW, DispatchMessageW, FillRect,
        GetClientRect, GetForegroundWindow, GetMessageW, GetParent, GetSystemMetrics,
        GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, InvalidateRect, IsChild,
        LB_ADDSTRING, LB_GETCOUNT, LB_GETCURSEL, LB_ITEMFROMPOINT, LB_RESETCONTENT, LB_SETCURSEL,
        LB_SETTABSTOPS, LoadCursorW, MoveWindow, PostMessageW, PostQuitMessage, RegisterClassW,
        SendMessageW, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, SetWindowTextW,
        ShowWindow, TranslateMessage, CREATESTRUCTW, CS_DROPSHADOW, CS_HREDRAW, CS_VREDRAW,
        CW_USEDEFAULT, EN_CHANGE, ES_AUTOHSCROLL, GWLP_USERDATA, GWLP_WNDPROC, HMENU,
        HWND_TOPMOST, IDC_ARROW, LBN_DBLCLK, LBS_NOTIFY, MSG, SM_CXSCREEN, SM_CYSCREEN,
        SW_HIDE, SW_SHOW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, WM_APP,
        WM_CLOSE, WM_COMMAND, WM_CREATE, WM_CTLCOLORLISTBOX, WM_CTLCOLOREDIT,
        WM_CTLCOLORSTATIC, WM_DESTROY, WM_ERASEBKGND, WM_HOTKEY, WM_KEYDOWN, WM_MOUSEMOVE,
        WM_NCCREATE, WM_NCDESTROY, WM_SETFONT, WM_SIZE, WNDCLASSW, WS_BORDER, WS_CHILD,
        WS_CLIPCHILDREN, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_TABSTOP, WS_VISIBLE,
        WS_VSCROLL,
    };

    const CLASS_NAME: &str = "SwiftFindOverlayWindowClass";
    const WINDOW_TITLE: &str = "SwiftFind Launcher";
    const INPUT_CLASS: &str = "EDIT";
    const LIST_CLASS: &str = "LISTBOX";
    const STATUS_CLASS: &str = "STATIC";

    const WINDOW_WIDTH: i32 = 640;
    const WINDOW_HEIGHT: i32 = 340;
    const PANEL_MARGIN_X: i32 = 16;
    const PANEL_MARGIN_TOP: i32 = 14;
    const PANEL_MARGIN_BOTTOM: i32 = 12;
    const INPUT_HEIGHT: i32 = 38;
    const ROW_GAP: i32 = 10;
    const STATUS_HEIGHT: i32 = 22;
    const CONTROL_ID_INPUT: usize = 1001;
    const CONTROL_ID_LIST: usize = 1002;
    const CONTROL_ID_STATUS: usize = 1003;

    const SWIFTFIND_WM_ESCAPE: u32 = WM_APP + 1;
    const SWIFTFIND_WM_QUERY_CHANGED: u32 = WM_APP + 2;
    const SWIFTFIND_WM_MOVE_UP: u32 = WM_APP + 3;
    const SWIFTFIND_WM_MOVE_DOWN: u32 = WM_APP + 4;
    const SWIFTFIND_WM_SUBMIT: u32 = WM_APP + 5;

    const COLOR_PANEL_BG: u32 = 0x001A1511;
    const COLOR_EDIT_BG: u32 = 0x00201814;
    const COLOR_LIST_BG: u32 = 0x001E1712;
    const COLOR_TEXT_PRIMARY: u32 = 0x00F0EAE3;
    const COLOR_TEXT_SECONDARY: u32 = 0x00B8ADA1;
    const COLOR_TEXT_ERROR: u32 = 0x007268FF;

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
        list_font: isize,
        status_font: isize,
        panel_brush: isize,
        edit_brush: isize,
        list_brush: isize,
        status_is_error: bool,
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
                    WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                    class_name.as_ptr(),
                    to_wide(WINDOW_TITLE).as_ptr(),
                    WS_POPUP | WS_CLIPCHILDREN,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    WINDOW_WIDTH,
                    WINDOW_HEIGHT,
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
            shell.hide();
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
            unsafe {
                SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_SHOWWINDOW | SWP_NOMOVE | SWP_NOSIZE,
                );
                ShowWindow(self.hwnd, SW_SHOW);
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
            unsafe {
                ShowWindow(self.hwnd, SW_HIDE);
            }
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
            let copied =
                unsafe { GetWindowTextW(state.edit_hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
            String::from_utf16_lossy(&buffer[..(copied as usize)])
        }

        pub fn set_status_text(&self, message: &str) {
            if let Some(state) = state_for(self.hwnd) {
                state.status_is_error = status_is_error(message);
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

                let count = unsafe { SendMessageW(state.list_hwnd, LB_GETCOUNT, 0, 0) };
                if count > 0 {
                    let clamped = selected_index.min((count as usize).saturating_sub(1));
                    unsafe {
                        SendMessageW(state.list_hwnd, LB_SETCURSEL, clamped, 0);
                    }
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

        fn center_window(&self) {
            let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
            let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
            let x = (screen_width - WINDOW_WIDTH).max(0) / 2;
            let y = (screen_height - WINDOW_HEIGHT).max(0) / 5;

            unsafe {
                SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    x,
                    y,
                    WINDOW_WIDTH,
                    WINDOW_HEIGHT,
                    SWP_NOACTIVATE,
                );
            }
        }

        fn apply_rounded_corners(&self) {
            let mut rect: RECT = unsafe { std::mem::zeroed() };
            unsafe {
                GetClientRect(self.hwnd, &mut rect);
            }
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            if width <= 0 || height <= 0 {
                return;
            }
            unsafe {
                let region = CreateRoundRectRgn(0, 0, width + 1, height + 1, 24, 24);
                SetWindowRgn(self.hwnd, region, 1);
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
                    SWIFTFIND_WM_QUERY_CHANGED => {
                        on_event(OverlayEvent::QueryChanged(self.query_text()));
                    }
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
                    state.edit_brush = unsafe { CreateSolidBrush(COLOR_EDIT_BG) } as isize;
                    state.list_brush = unsafe { CreateSolidBrush(COLOR_LIST_BG) } as isize;
                    state.input_font = create_font(-22, FW_SEMIBOLD as i32);
                    state.list_font = create_font(-17, FW_MEDIUM as i32);
                    state.status_font = create_font(-15, FW_MEDIUM as i32);

                    state.edit_hwnd = unsafe {
                        CreateWindowExW(
                            0,
                            to_wide(INPUT_CLASS).as_ptr(),
                            to_wide("").as_ptr(),
                            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL as u32,
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
                            WS_CHILD | WS_VISIBLE | WS_VSCROLL | LBS_NOTIFY as u32 | WS_BORDER,
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
                            to_wide("Ready").as_ptr(),
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
                        SendMessageW(state.list_hwnd, WM_SETFONT, state.list_font as usize, 1);
                        SendMessageW(state.status_hwnd, WM_SETFONT, state.status_font as usize, 1);
                        let tab_stop = [252_i32];
                        SendMessageW(
                            state.list_hwnd,
                            LB_SETTABSTOPS,
                            1,
                            tab_stop.as_ptr() as LPARAM,
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
                    }
                    layout_children(hwnd, state);
                }
                0
            }
            WM_ERASEBKGND => {
                if let Some(state) = state_for(hwnd) {
                    if state.panel_brush != 0 {
                        let mut rect: RECT = unsafe { std::mem::zeroed() };
                        unsafe {
                            GetClientRect(hwnd, &mut rect);
                            FillRect(wparam as _, &rect, state.panel_brush as _);
                        }
                        return 1;
                    }
                }
                unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
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
            WM_CTLCOLOREDIT => {
                if let Some(state) = state_for(hwnd) {
                    unsafe {
                        SetTextColor(wparam as _, COLOR_TEXT_PRIMARY);
                        SetBkColor(wparam as _, COLOR_EDIT_BG);
                    }
                    if state.edit_brush != 0 {
                        return state.edit_brush;
                    }
                }
                unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
            }
            WM_CTLCOLORLISTBOX => {
                if let Some(state) = state_for(hwnd) {
                    unsafe {
                        SetTextColor(wparam as _, COLOR_TEXT_PRIMARY);
                        SetBkColor(wparam as _, COLOR_LIST_BG);
                    }
                    if state.list_brush != 0 {
                        return state.list_brush;
                    }
                }
                unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
            }
            WM_CTLCOLORSTATIC => {
                if let Some(state) = state_for(hwnd) {
                    let control = lparam as HWND;
                    if control == state.status_hwnd {
                        let text_color = if state.status_is_error {
                            COLOR_TEXT_ERROR
                        } else {
                            COLOR_TEXT_SECONDARY
                        };
                        unsafe {
                            SetTextColor(wparam as _, text_color);
                            SetBkColor(wparam as _, COLOR_PANEL_BG);
                            SetBkMode(wparam as _, TRANSPARENT as i32);
                        }
                        if state.panel_brush != 0 {
                            return state.panel_brush;
                        }
                    }
                }
                unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
            }
            WM_SIZE => {
                if let Some(state) = state_for(hwnd) {
                    layout_children(hwnd, state);
                }
                0
            }
            WM_CLOSE => {
                unsafe {
                    ShowWindow(hwnd, SW_HIDE);
                }
                0
            }
            SWIFTFIND_WM_ESCAPE => 0,
            SWIFTFIND_WM_QUERY_CHANGED => 0,
            SWIFTFIND_WM_MOVE_UP => 0,
            SWIFTFIND_WM_MOVE_DOWN => 0,
            SWIFTFIND_WM_SUBMIT => 0,
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
                        if (*state_ptr).input_font != 0 {
                            DeleteObject((*state_ptr).input_font as _);
                        }
                        if (*state_ptr).list_font != 0 {
                            DeleteObject((*state_ptr).list_font as _);
                        }
                        if (*state_ptr).status_font != 0 {
                            DeleteObject((*state_ptr).status_font as _);
                        }
                        if (*state_ptr).panel_brush != 0 {
                            DeleteObject((*state_ptr).panel_brush as _);
                        }
                        if (*state_ptr).edit_brush != 0 {
                            DeleteObject((*state_ptr).edit_brush as _);
                        }
                        if (*state_ptr).list_brush != 0 {
                            DeleteObject((*state_ptr).list_brush as _);
                        }
                        let _ = Box::from_raw(state_ptr);
                        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    }
                }
                0
            }
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
                let idx = unsafe { SendMessageW(hwnd, LB_ITEMFROMPOINT, 0, lparam) };
                let item_index = (idx & 0xFFFF) as usize;
                let outside = ((idx >> 16) & 0xFFFF) != 0;
                let count = unsafe { SendMessageW(hwnd, LB_GETCOUNT, 0, 0) };
                if !outside && count > 0 && item_index < count as usize {
                    unsafe {
                        SendMessageW(hwnd, LB_SETCURSEL, item_index, 0);
                    }
                }
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
            std::mem::transmute::<isize, windows_sys::Win32::UI::WindowsAndMessaging::WNDPROC>(
                prev_ptr,
            )
        };
        unsafe { CallWindowProcW(prev_proc, hwnd, message, wparam, lparam) }
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
        let list_top = PANEL_MARGIN_TOP + INPUT_HEIGHT + ROW_GAP;
        let status_top = height - PANEL_MARGIN_BOTTOM - STATUS_HEIGHT;
        let list_height = (status_top - list_top - ROW_GAP).max(60);

        unsafe {
            MoveWindow(
                state.edit_hwnd,
                PANEL_MARGIN_X,
                PANEL_MARGIN_TOP,
                input_width,
                INPUT_HEIGHT,
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
            MoveWindow(
                state.status_hwnd,
                PANEL_MARGIN_X,
                status_top,
                input_width,
                STATUS_HEIGHT,
                1,
            );
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
        unsafe {
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
                OUT_DEFAULT_PRECIS,
                0,
                DEFAULT_QUALITY,
                FF_DONTCARE as u32,
                to_wide("Segoe UI").as_ptr(),
            )
        } as isize
    }

    fn status_is_error(message: &str) -> bool {
        message.to_ascii_lowercase().contains("error")
    }

    fn to_wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(target_os = "windows")]
pub use imp::{NativeOverlayShell, OverlayEvent};
