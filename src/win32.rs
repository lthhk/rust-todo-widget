#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals
)]

use std::ffi::{c_int, c_void, OsStr};
use std::os::windows::ffi::OsStrExt;

pub type BOOL = i32;
pub type BYTE = u8;
pub type DWORD = u32;
pub type UINT = u32;
pub type WORD = u16;
pub type LONG = i32;
pub type LONG_PTR = isize;
pub type WPARAM = usize;
pub type LPARAM = isize;
pub type LRESULT = isize;
pub type HWND = isize;
pub type HINSTANCE = isize;
pub type HICON = isize;
pub type HCURSOR = isize;
pub type HBRUSH = isize;
pub type HDC = isize;
pub type HMENU = isize;
pub type HFONT = isize;
pub type HPEN = isize;
pub type HGDIOBJ = isize;
pub type HKEY = isize;
pub type LPCWSTR = *const u16;
pub type LPWSTR = *mut u16;
pub type COLORREF = u32;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct POINT {
    pub x: LONG,
    pub y: LONG,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct RECT {
    pub left: LONG,
    pub top: LONG,
    pub right: LONG,
    pub bottom: LONG,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: UINT,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: DWORD,
    pub pt: POINT,
}

pub type WNDPROC = Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT>;

#[repr(C)]
pub struct WNDCLASSW {
    pub style: UINT,
    pub lpfnWndProc: WNDPROC,
    pub cbClsExtra: c_int,
    pub cbWndExtra: c_int,
    pub hInstance: HINSTANCE,
    pub hIcon: HICON,
    pub hCursor: HCURSOR,
    pub hbrBackground: HBRUSH,
    pub lpszMenuName: LPCWSTR,
    pub lpszClassName: LPCWSTR,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PAINTSTRUCT {
    pub hdc: HDC,
    pub fErase: BOOL,
    pub rcPaint: RECT,
    pub fRestore: BOOL,
    pub fIncUpdate: BOOL,
    pub rgbReserved: [BYTE; 32],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SYSTEMTIME {
    pub wYear: WORD,
    pub wMonth: WORD,
    pub wDayOfWeek: WORD,
    pub wDay: WORD,
    pub wHour: WORD,
    pub wMinute: WORD,
    pub wSecond: WORD,
    pub wMilliseconds: WORD,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct MINMAXINFO {
    pub ptReserved: POINT,
    pub ptMaxSize: POINT,
    pub ptMaxPosition: POINT,
    pub ptMinTrackSize: POINT,
    pub ptMaxTrackSize: POINT,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct INITCOMMONCONTROLSEX {
    pub dwSize: DWORD,
    pub dwICC: DWORD,
}

#[repr(C)]
pub struct CHOOSECOLORW {
    pub lStructSize: DWORD,
    pub hwndOwner: HWND,
    pub hInstance: HWND,
    pub rgbResult: COLORREF,
    pub lpCustColors: *mut COLORREF,
    pub Flags: DWORD,
    pub lCustData: LPARAM,
    pub lpfnHook: isize,
    pub lpTemplateName: LPCWSTR,
}

pub const CS_DBLCLKS: UINT = 0x0008;
pub const CS_HREDRAW: UINT = 0x0002;
pub const CS_VREDRAW: UINT = 0x0001;

pub const WS_OVERLAPPED: DWORD = 0x00000000;
pub const WS_POPUP: DWORD = 0x80000000;
pub const WS_CHILD: DWORD = 0x40000000;
pub const WS_VISIBLE: DWORD = 0x10000000;
pub const WS_DISABLED: DWORD = 0x08000000;
pub const WS_CLIPSIBLINGS: DWORD = 0x04000000;
pub const WS_BORDER: DWORD = 0x00800000;
pub const WS_DLGFRAME: DWORD = 0x00400000;
pub const WS_VSCROLL: DWORD = 0x00200000;
pub const WS_CAPTION: DWORD = 0x00C00000;
pub const WS_SYSMENU: DWORD = 0x00080000;
pub const WS_THICKFRAME: DWORD = 0x00040000;
pub const WS_MINIMIZEBOX: DWORD = 0x00020000;
pub const WS_MAXIMIZEBOX: DWORD = 0x00010000;
pub const WS_TABSTOP: DWORD = 0x00010000;
pub const WS_OVERLAPPEDWINDOW: DWORD =
    WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX;

pub const WS_EX_DLGMODALFRAME: DWORD = 0x00000001;
pub const WS_EX_TOPMOST: DWORD = 0x00000008;
pub const WS_EX_CLIENTEDGE: DWORD = 0x00000200;
pub const WS_EX_TOOLWINDOW: DWORD = 0x00000080;
pub const WS_EX_LAYERED: DWORD = 0x00080000;

pub const ES_LEFT: DWORD = 0x0000;
pub const ES_MULTILINE: DWORD = 0x0004;
pub const ES_PASSWORD: DWORD = 0x0020;
pub const ES_AUTOVSCROLL: DWORD = 0x0040;
pub const ES_AUTOHSCROLL: DWORD = 0x0080;
pub const ES_READONLY: DWORD = 0x0800;
pub const ES_WANTRETURN: DWORD = 0x1000;

pub const BS_PUSHBUTTON: DWORD = 0x00000000;
pub const BS_AUTOCHECKBOX: DWORD = 0x00000003;

pub const CBS_DROPDOWNLIST: DWORD = 0x0003;
pub const CBS_HASSTRINGS: DWORD = 0x0200;

pub const DTS_SHOWNONE: DWORD = 0x0002;
pub const ICC_DATE_CLASSES: DWORD = 0x00000100;

pub const CC_RGBINIT: DWORD = 0x00000001;
pub const CC_FULLOPEN: DWORD = 0x00000002;

pub const CW_USEDEFAULT: c_int = 0x80000000u32 as c_int;
pub const SW_SHOW: c_int = 5;
pub const ERROR_FILE_NOT_FOUND: LONG = 2;

pub const WM_CREATE: UINT = 0x0001;
pub const WM_DESTROY: UINT = 0x0002;
pub const WM_SIZE: UINT = 0x0005;
pub const WM_CLOSE: UINT = 0x0010;
pub const WM_GETMINMAXINFO: UINT = 0x0024;
pub const WM_NCHITTEST: UINT = 0x0084;
pub const WM_PAINT: UINT = 0x000F;
pub const WM_COMMAND: UINT = 0x0111;
pub const WM_TIMER: UINT = 0x0113;
pub const WM_SETFONT: UINT = 0x0030;
pub const WM_MOUSEMOVE: UINT = 0x0200;
pub const WM_LBUTTONDOWN: UINT = 0x0201;
pub const WM_LBUTTONUP: UINT = 0x0202;
pub const WM_LBUTTONDBLCLK: UINT = 0x0203;
pub const WM_RBUTTONUP: UINT = 0x0205;
pub const WM_MOUSEWHEEL: UINT = 0x020A;
pub const WM_NCLBUTTONDOWN: UINT = 0x00A1;
pub const WM_APP: UINT = 0x8000;

pub const HTCAPTION: WPARAM = 2;
pub const HTCLIENT: WPARAM = 1;
pub const HTLEFT: WPARAM = 10;
pub const HTRIGHT: WPARAM = 11;
pub const HTTOP: WPARAM = 12;
pub const HTTOPLEFT: WPARAM = 13;
pub const HTTOPRIGHT: WPARAM = 14;
pub const HTBOTTOM: WPARAM = 15;
pub const HTBOTTOMLEFT: WPARAM = 16;
pub const HTBOTTOMRIGHT: WPARAM = 17;

pub const HWND_BOTTOM: HWND = 1;

pub const SWP_NOSIZE: UINT = 0x0001;
pub const SWP_NOMOVE: UINT = 0x0002;
pub const SWP_NOACTIVATE: UINT = 0x0010;

pub const IDC_ARROW: LPCWSTR = 32512usize as LPCWSTR;
pub const IDI_APPLICATION: LPCWSTR = 32512usize as LPCWSTR;

pub const LWA_ALPHA: DWORD = 0x00000002;

pub const DT_LEFT: UINT = 0x00000000;
pub const DT_CENTER: UINT = 0x00000001;
pub const DT_RIGHT: UINT = 0x00000002;
pub const DT_VCENTER: UINT = 0x00000004;
pub const DT_WORDBREAK: UINT = 0x00000010;
pub const DT_SINGLELINE: UINT = 0x00000020;
pub const DT_NOPREFIX: UINT = 0x00000800;
pub const DT_END_ELLIPSIS: UINT = 0x00008000;

pub const TRANSPARENT: c_int = 1;

pub const FW_NORMAL: c_int = 400;
pub const FW_SEMIBOLD: c_int = 600;
pub const FW_BOLD: c_int = 700;
pub const DEFAULT_CHARSET: DWORD = 1;
pub const OUT_DEFAULT_PRECIS: DWORD = 0;
pub const CLIP_DEFAULT_PRECIS: DWORD = 0;
pub const CLEARTYPE_QUALITY: DWORD = 5;
pub const DEFAULT_PITCH: DWORD = 0;
pub const PS_SOLID: c_int = 0;

pub const MF_STRING: UINT = 0x00000000;
pub const MF_SEPARATOR: UINT = 0x00000800;
pub const TPM_RIGHTBUTTON: UINT = 0x0002;
pub const TPM_RETURNCMD: UINT = 0x0100;
pub const TPM_NONOTIFY: UINT = 0x0080;

pub const MB_OK: UINT = 0x00000000;
pub const MB_ICONERROR: UINT = 0x00000010;
pub const MB_ICONINFORMATION: UINT = 0x00000040;

pub const CB_GETCURSEL: UINT = 0x0147;
pub const CB_ADDSTRING: UINT = 0x0143;
pub const CB_SETCURSEL: UINT = 0x014E;

pub const BM_GETCHECK: UINT = 0x00F0;
pub const BM_SETCHECK: UINT = 0x00F1;
pub const BST_CHECKED: WPARAM = 1;
pub const BST_UNCHECKED: WPARAM = 0;

pub const DTM_FIRST: UINT = 0x1000;
pub const DTM_GETSYSTEMTIME: UINT = DTM_FIRST + 1;
pub const DTM_SETSYSTEMTIME: UINT = DTM_FIRST + 2;
pub const DTM_SETFORMATW: UINT = DTM_FIRST + 50;
pub const GDT_VALID: WPARAM = 0;
pub const GDT_NONE: WPARAM = 1;

pub const KEY_SET_VALUE: DWORD = 0x0002;
pub const REG_OPTION_NON_VOLATILE: DWORD = 0x00000000;
pub const REG_SZ: DWORD = 1;
pub const HKEY_CURRENT_USER: HKEY = 0x80000001u32 as isize;

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn GetModuleHandleW(lpModuleName: LPCWSTR) -> HINSTANCE;
    pub fn GetLocalTime(lpSystemTime: *mut SYSTEMTIME);
}

#[link(name = "comctl32")]
unsafe extern "system" {
    pub fn InitCommonControlsEx(picce: *const INITCOMMONCONTROLSEX) -> BOOL;
}

#[link(name = "comdlg32")]
unsafe extern "system" {
    pub fn ChooseColorW(lpcc: *mut CHOOSECOLORW) -> BOOL;
}

#[link(name = "user32")]
unsafe extern "system" {
    pub fn RegisterClassW(lpWndClass: *const WNDCLASSW) -> u16;
    pub fn CreateWindowExW(
        dwExStyle: DWORD,
        lpClassName: LPCWSTR,
        lpWindowName: LPCWSTR,
        dwStyle: DWORD,
        X: c_int,
        Y: c_int,
        nWidth: c_int,
        nHeight: c_int,
        hWndParent: HWND,
        hMenu: HMENU,
        hInstance: HINSTANCE,
        lpParam: *mut c_void,
    ) -> HWND;
    pub fn DefWindowProcW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn ShowWindow(hWnd: HWND, nCmdShow: c_int) -> BOOL;
    pub fn UpdateWindow(hWnd: HWND) -> BOOL;
    pub fn DestroyWindow(hWnd: HWND) -> BOOL;
    pub fn IsWindow(hWnd: HWND) -> BOOL;
    pub fn GetMessageW(
        lpMsg: *mut MSG,
        hWnd: HWND,
        wMsgFilterMin: UINT,
        wMsgFilterMax: UINT,
    ) -> BOOL;
    pub fn TranslateMessage(lpMsg: *const MSG) -> BOOL;
    pub fn DispatchMessageW(lpMsg: *const MSG) -> LRESULT;
    pub fn PostQuitMessage(nExitCode: c_int);
    pub fn PostMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> BOOL;
    pub fn LoadCursorW(hInstance: HINSTANCE, lpCursorName: LPCWSTR) -> HCURSOR;
    pub fn LoadIconW(hInstance: HINSTANCE, lpIconName: LPCWSTR) -> HICON;
    pub fn SetLayeredWindowAttributes(
        hwnd: HWND,
        crKey: COLORREF,
        bAlpha: BYTE,
        dwFlags: DWORD,
    ) -> BOOL;
    pub fn BeginPaint(hWnd: HWND, lpPaint: *mut PAINTSTRUCT) -> HDC;
    pub fn EndPaint(hWnd: HWND, lpPaint: *const PAINTSTRUCT) -> BOOL;
    pub fn InvalidateRect(hWnd: HWND, lpRect: *const RECT, bErase: BOOL) -> BOOL;
    pub fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    pub fn FillRect(hDC: HDC, lprc: *const RECT, hbr: HBRUSH) -> c_int;
    pub fn DrawTextW(
        hdc: HDC,
        lpchText: LPCWSTR,
        cchText: c_int,
        lprc: *mut RECT,
        format: UINT,
    ) -> c_int;
    pub fn MessageBoxW(hWnd: HWND, lpText: LPCWSTR, lpCaption: LPCWSTR, uType: UINT) -> c_int;
    pub fn SendMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn ReleaseCapture() -> BOOL;
    pub fn SetTimer(hWnd: HWND, nIDEvent: usize, uElapse: UINT, lpTimerFunc: isize) -> usize;
    pub fn KillTimer(hWnd: HWND, uIDEvent: usize) -> BOOL;
    pub fn CreatePopupMenu() -> HMENU;
    pub fn AppendMenuW(hMenu: HMENU, uFlags: UINT, uIDNewItem: usize, lpNewItem: LPCWSTR) -> BOOL;
    pub fn TrackPopupMenu(
        hMenu: HMENU,
        uFlags: UINT,
        x: c_int,
        y: c_int,
        nReserved: c_int,
        hWnd: HWND,
        prcRect: *const RECT,
    ) -> c_int;
    pub fn DestroyMenu(hMenu: HMENU) -> BOOL;
    pub fn GetCursorPos(lpPoint: *mut POINT) -> BOOL;
    pub fn GetWindowRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    pub fn ScreenToClient(hWnd: HWND, lpPoint: *mut POINT) -> BOOL;
    pub fn ClientToScreen(hWnd: HWND, lpPoint: *mut POINT) -> BOOL;
    pub fn CreateCaret(hWnd: HWND, hBitmap: isize, nWidth: c_int, nHeight: c_int) -> BOOL;
    pub fn SetCapture(hWnd: HWND) -> HWND;
    pub fn SetFocus(hWnd: HWND) -> HWND;
    pub fn EnableWindow(hWnd: HWND, bEnable: BOOL) -> BOOL;
    pub fn GetWindowTextLengthW(hWnd: HWND) -> c_int;
    pub fn GetWindowTextW(hWnd: HWND, lpString: LPWSTR, nMaxCount: c_int) -> c_int;
    pub fn SetWindowTextW(hWnd: HWND, lpString: LPCWSTR) -> BOOL;
    pub fn MoveWindow(
        hWnd: HWND,
        X: c_int,
        Y: c_int,
        nWidth: c_int,
        nHeight: c_int,
        bRepaint: BOOL,
    ) -> BOOL;
    pub fn SetWindowPos(
        hWnd: HWND,
        hWndInsertAfter: HWND,
        X: c_int,
        Y: c_int,
        cx: c_int,
        cy: c_int,
        uFlags: UINT,
    ) -> BOOL;
}

#[link(name = "advapi32")]
unsafe extern "system" {
    pub fn RegCreateKeyExW(
        hKey: HKEY,
        lpSubKey: LPCWSTR,
        Reserved: DWORD,
        lpClass: LPWSTR,
        dwOptions: DWORD,
        samDesired: DWORD,
        lpSecurityAttributes: *const c_void,
        phkResult: *mut HKEY,
        lpdwDisposition: *mut DWORD,
    ) -> LONG;
    pub fn RegSetValueExW(
        hKey: HKEY,
        lpValueName: LPCWSTR,
        Reserved: DWORD,
        dwType: DWORD,
        lpData: *const u8,
        cbData: DWORD,
    ) -> LONG;
    pub fn RegDeleteValueW(hKey: HKEY, lpValueName: LPCWSTR) -> LONG;
    pub fn RegCloseKey(hKey: HKEY) -> LONG;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    pub fn CreateSolidBrush(color: COLORREF) -> HBRUSH;
    pub fn CreatePen(iStyle: c_int, cWidth: c_int, color: COLORREF) -> HPEN;
    pub fn DeleteObject(ho: HGDIOBJ) -> BOOL;
    pub fn SelectObject(hdc: HDC, h: HGDIOBJ) -> HGDIOBJ;
    pub fn SetBkMode(hdc: HDC, mode: c_int) -> c_int;
    pub fn SetTextColor(hdc: HDC, color: COLORREF) -> COLORREF;
    pub fn RoundRect(
        hdc: HDC,
        left: c_int,
        top: c_int,
        right: c_int,
        bottom: c_int,
        width: c_int,
        height: c_int,
    ) -> BOOL;
    #[allow(clippy::too_many_arguments)]
    pub fn CreateFontW(
        cHeight: c_int,
        cWidth: c_int,
        cEscapement: c_int,
        cOrientation: c_int,
        cWeight: c_int,
        bItalic: DWORD,
        bUnderline: DWORD,
        bStrikeOut: DWORD,
        iCharSet: DWORD,
        iOutPrecision: DWORD,
        iClipPrecision: DWORD,
        iQuality: DWORD,
        iPitchAndFamily: DWORD,
        pszFaceName: LPCWSTR,
    ) -> HFONT;
}

pub fn to_wide(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

pub fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    (r as COLORREF) | ((g as COLORREF) << 8) | ((b as COLORREF) << 16)
}

pub fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT {
        left,
        top,
        right,
        bottom,
    }
}

pub fn loword(value: WPARAM) -> u16 {
    (value & 0xffff) as u16
}

pub fn get_x_lparam(value: LPARAM) -> i32 {
    (value as u16 as i16) as i32
}

pub fn get_y_lparam(value: LPARAM) -> i32 {
    (((value >> 16) as u16) as i16) as i32
}

pub fn wheel_delta(value: WPARAM) -> i16 {
    (((value >> 16) & 0xffff) as u16) as i16
}

pub unsafe fn set_text(hwnd: HWND, text: &str) {
    let wide = to_wide(text);
    unsafe {
        SetWindowTextW(hwnd, wide.as_ptr());
    }
}

pub unsafe fn get_text(hwnd: HWND) -> String {
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return String::new();
    }
    let mut buf = vec![0u16; len as usize + 1];
    let read = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1) };
    String::from_utf16_lossy(&buf[..read.max(0) as usize])
}

pub unsafe fn message_box(hwnd: HWND, title: &str, text: &str, flags: UINT) {
    let title = to_wide(title);
    let text = to_wide(text);
    unsafe {
        MessageBoxW(hwnd, text.as_ptr(), title.as_ptr(), flags);
    }
}

pub unsafe fn set_run_at_startup(value_name: &str, command: Option<&str>) -> Result<(), String> {
    let path = to_wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    let mut key = 0;
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            path.as_ptr(),
            0,
            std::ptr::null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        )
    };
    if status != 0 {
        return Err(format!("无法打开开机启动注册表项，错误码 {}", status));
    }

    let name = to_wide(value_name);
    let result = match command {
        Some(command) => {
            let data = to_wide(command);
            unsafe {
                RegSetValueExW(
                    key,
                    name.as_ptr(),
                    0,
                    REG_SZ,
                    data.as_ptr() as *const u8,
                    (data.len() * std::mem::size_of::<u16>()) as DWORD,
                )
            }
        }
        None => unsafe { RegDeleteValueW(key, name.as_ptr()) },
    };
    unsafe {
        RegCloseKey(key);
    }

    if result == 0 || (command.is_none() && result == ERROR_FILE_NOT_FOUND) {
        Ok(())
    } else {
        Err(format!("更新开机启动失败，错误码 {}", result))
    }
}
