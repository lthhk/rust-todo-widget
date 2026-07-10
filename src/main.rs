#![windows_subsystem = "windows"]

mod datetime;
mod store;
mod summary;
mod win32;

use crate::datetime::{weekday_name, LocalDateTime};
use crate::store::{next_task_id, Settings, Store, Task};
use crate::summary::{generate_and_store_summary, SummaryResult};
use crate::win32::*;
use std::cmp::Reverse;
use std::ptr::null;
use std::sync::{Mutex, OnceLock};
use std::thread;

const MAIN_CLASS: &str = "RustTodoWidgetMain";
const TODO_FORM_CLASS: &str = "RustTodoWidgetTodoForm";
const SEARCH_FORM_CLASS: &str = "RustTodoWidgetSearchForm";
const SETTINGS_FORM_CLASS: &str = "RustTodoWidgetSettingsForm";
const TEXT_FORM_CLASS: &str = "RustTodoWidgetTextForm";
const STARTUP_VALUE_NAME: &str = "RustTodoWidget";

const MIN_MAIN_WIDTH: i32 = 340;
const MIN_MAIN_HEIGHT: i32 = 460;
const RESIZE_BORDER: i32 = 8;
const TIMER_AUTO_SUMMARY: usize = 1;
const WM_SUMMARY_READY: UINT = WM_APP + 10;

const MENU_COMPLETE: usize = 1001;
const MENU_EDIT: usize = 1002;
const MENU_NOTE: usize = 1003;
const MENU_DELETE: usize = 1004;

const ID_TODO_TITLE: usize = 2001;
const ID_TODO_DUE: usize = 2002;
const ID_TODO_NOTE: usize = 2003;
const ID_TODO_SAVE: usize = 2004;
const ID_TODO_CANCEL: usize = 2005;
const ID_TODO_TIME: usize = 2006;

const ID_SEARCH_NAME: usize = 3001;
const ID_SEARCH_START: usize = 3002;
const ID_SEARCH_END: usize = 3003;
const ID_SEARCH_BUTTON: usize = 3004;
const ID_SEARCH_RESULT: usize = 3005;
const ID_SEARCH_CLOSE: usize = 3006;

const ID_SETTINGS_WEEKDAY: usize = 4001;
const ID_SETTINGS_AUTO: usize = 4002;
const ID_SETTINGS_STARTUP: usize = 4003;
const ID_SETTINGS_BG: usize = 4004;
const ID_SETTINGS_ALPHA: usize = 4005;
const ID_SETTINGS_API: usize = 4006;
const ID_SETTINGS_MODEL: usize = 4007;
const ID_SETTINGS_KEY: usize = 4008;
const ID_SETTINGS_SAVE: usize = 4009;
const ID_SETTINGS_CANCEL: usize = 4010;
const ID_SETTINGS_BG_PICK: usize = 4011;

const ID_TEXT_CLOSE: usize = 5001;

static APP: OnceLock<Mutex<AppState>> = OnceLock::new();
static TODO_FORM: OnceLock<Mutex<Option<TodoFormState>>> = OnceLock::new();
static SEARCH_FORM: OnceLock<Mutex<Option<SearchFormState>>> = OnceLock::new();
static SETTINGS_FORM: OnceLock<Mutex<Option<SettingsFormState>>> = OnceLock::new();
static TEXT_FORM: OnceLock<Mutex<Option<TextFormState>>> = OnceLock::new();
static SUMMARY_PAYLOAD: OnceLock<Mutex<Option<SummaryPayload>>> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolbarAction {
    Add,
    Complete,
    Search,
    Summary,
    Settings,
    Close,
}

#[derive(Clone, Copy, Debug)]
struct RectI {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl RectI {
    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x <= self.right && y >= self.top && y <= self.bottom
    }

    fn to_rect(self) -> RECT {
        rect(self.left, self.top, self.right, self.bottom)
    }
}

#[derive(Clone, Copy)]
struct DragState {
    cursor_start: POINT,
    window_start: RECT,
}

struct Fonts {
    title: HFONT,
    row: HFONT,
    small: HFONT,
    icon: HFONT,
}

#[derive(Clone, Copy)]
struct Theme {
    window_bg: COLORREF,
    header_bg: COLORREF,
    card_bg: COLORREF,
    selected_bg: COLORREF,
    text_primary: COLORREF,
    text_secondary: COLORREF,
    footer_bg: COLORREF,
    button_bg: COLORREF,
}

struct AppState {
    store: Store,
    tasks: Vec<Task>,
    settings: Settings,
    selected_id: Option<u64>,
    scroll: usize,
    row_rects: Vec<(u64, RectI)>,
    button_rects: Vec<(ToolbarAction, RectI)>,
    dragging: Option<DragState>,
    status: String,
    summary_in_progress: bool,
    fonts: Fonts,
}

impl AppState {
    fn new(store: Store) -> Self {
        let settings = store.load_settings();
        let tasks = store.load_tasks();
        Self {
            store,
            tasks,
            settings,
            selected_id: None,
            scroll: 0,
            row_rects: Vec::new(),
            button_rects: Vec::new(),
            dragging: None,
            status: "准备就绪".to_string(),
            summary_in_progress: false,
            fonts: Fonts::new(),
        }
    }

    fn active_sorted(&self) -> Vec<Task> {
        let mut tasks: Vec<Task> = self
            .tasks
            .iter()
            .filter(|task| !task.completed)
            .cloned()
            .collect();
        tasks.sort_by_key(|task| task.due);
        tasks
    }

    fn completed_between(&self, start: LocalDateTime, end: LocalDateTime) -> Vec<Task> {
        let mut tasks: Vec<Task> = self
            .tasks
            .iter()
            .filter(|task| {
                task.completed && task.completed_at.is_some_and(|dt| dt >= start && dt <= end)
            })
            .cloned()
            .collect();
        tasks.sort_by_key(|task| task.completed_at.map(Reverse));
        tasks
    }

    fn add_task(&mut self, title: String, due: LocalDateTime, note: String) -> Result<(), String> {
        let id = next_task_id(&self.tasks);
        let task = Task {
            id,
            title,
            due,
            note,
            created_at: LocalDateTime::now(),
            completed: false,
            completed_at: None,
        };
        self.tasks.push(task);
        self.selected_id = Some(id);
        self.scroll = 0;
        self.save_tasks("已新建待办")
    }

    fn update_task(
        &mut self,
        id: u64,
        title: String,
        due: LocalDateTime,
        note: String,
    ) -> Result<(), String> {
        let Some(task) = self.tasks.iter_mut().find(|task| task.id == id) else {
            return Err("没有找到要编辑的待办".to_string());
        };
        task.title = title;
        task.due = due;
        task.note = note;
        self.save_tasks("已更新待办")
    }

    fn complete_task(&mut self, id: u64) -> Result<(), String> {
        let Some(task) = self.tasks.iter_mut().find(|task| task.id == id) else {
            return Err("没有找到要完成的待办".to_string());
        };
        task.completed = true;
        task.completed_at = Some(LocalDateTime::now());
        if self.selected_id == Some(id) {
            self.selected_id = None;
        }
        self.save_tasks("已标记完成")
    }

    fn delete_task(&mut self, id: u64) -> Result<(), String> {
        let before = self.tasks.len();
        self.tasks.retain(|task| task.id != id);
        if before == self.tasks.len() {
            return Err("没有找到要删除的记录".to_string());
        }
        if self.selected_id == Some(id) {
            self.selected_id = None;
        }
        self.save_tasks("已从所有记录中删除")
    }

    fn save_tasks(&mut self, ok_status: &str) -> Result<(), String> {
        match self.store.save_tasks(&self.tasks) {
            Ok(()) => {
                self.status = ok_status.to_string();
                Ok(())
            }
            Err(err) => {
                let msg = format!("保存失败：{}", err);
                self.status = msg.clone();
                Err(msg)
            }
        }
    }

    fn save_settings(&mut self, ok_status: &str) -> Result<(), String> {
        match self.store.save_settings(&self.settings) {
            Ok(()) => {
                self.status = ok_status.to_string();
                Ok(())
            }
            Err(err) => {
                let msg = format!("保存设置失败：{}", err);
                self.status = msg.clone();
                Err(msg)
            }
        }
    }

    fn action_at(&self, x: i32, y: i32) -> Option<ToolbarAction> {
        self.button_rects
            .iter()
            .find(|(_, rc)| rc.contains(x, y))
            .map(|(action, _)| *action)
    }

    fn row_at(&self, x: i32, y: i32) -> Option<u64> {
        self.row_rects
            .iter()
            .find(|(_, rc)| rc.contains(x, y))
            .map(|(id, _)| *id)
    }
}

impl Fonts {
    fn new() -> Self {
        unsafe {
            Self {
                title: create_font(24, FW_BOLD),
                row: create_font(18, FW_SEMIBOLD),
                small: create_font(14, FW_NORMAL),
                icon: create_font(16, FW_BOLD),
            }
        }
    }
}

impl Theme {
    fn from_settings(settings: &Settings) -> Self {
        let base = parse_hex_color(&settings.background_color).unwrap_or((245, 247, 250));
        Self {
            window_bg: rgb(base.0, base.1, base.2),
            header_bg: mix_rgb(base, (255, 255, 255), 78),
            card_bg: mix_rgb(base, (255, 255, 255), 88),
            selected_bg: mix_rgb(base, (217, 234, 255), 72),
            text_primary: rgb(26, 33, 44),
            text_secondary: rgb(95, 107, 124),
            footer_bg: mix_rgb(base, (255, 255, 255), 70),
            button_bg: mix_rgb(base, (255, 255, 255), 82),
        }
    }
}

struct TodoFormResult {
    title: String,
    due: String,
    note: String,
}

struct TodoFormState {
    title: String,
    due: String,
    note: String,
    result: Option<TodoFormResult>,
    done: bool,
    title_edit: HWND,
    due_date_picker: HWND,
    due_time_edit: HWND,
    note_edit: HWND,
}

struct SearchFormState {
    done: bool,
    name_edit: HWND,
    start_picker: HWND,
    end_picker: HWND,
    result_edit: HWND,
}

struct SettingsFormState {
    settings: Settings,
    result: Option<Settings>,
    done: bool,
    weekday_combo: HWND,
    auto_check: HWND,
    startup_check: HWND,
    bg_edit: HWND,
    alpha_edit: HWND,
    api_edit: HWND,
    model_edit: HWND,
    key_edit: HWND,
}

struct TextFormState {
    text: String,
    done: bool,
}

struct SummaryPayload {
    result: Result<SummaryResult, String>,
    manual: bool,
}

fn main() {
    let store = match Store::new() {
        Ok(store) => store,
        Err(err) => {
            unsafe {
                message_box(
                    0,
                    "Rust Todo Widget",
                    &format!("初始化数据目录失败：{}", err),
                    MB_OK | MB_ICONERROR,
                );
            }
            return;
        }
    };

    let app = AppState::new(store);
    if let Err(err) = sync_launch_on_startup(app.settings.launch_on_startup) {
        unsafe {
            message_box(
                0,
                "Rust Todo Widget",
                &format!("同步开机自启动失败：{}", err),
                MB_OK | MB_ICONERROR,
            );
        }
    }
    let x = app.settings.window_x;
    let y = app.settings.window_y;
    let width = app.settings.window_width.max(MIN_MAIN_WIDTH);
    let height = app.settings.window_height.max(MIN_MAIN_HEIGHT);
    let _ = APP.set(Mutex::new(app));
    let _ = TODO_FORM.set(Mutex::new(None));
    let _ = SEARCH_FORM.set(Mutex::new(None));
    let _ = SETTINGS_FORM.set(Mutex::new(None));
    let _ = TEXT_FORM.set(Mutex::new(None));
    let _ = SUMMARY_PAYLOAD.set(Mutex::new(None));

    unsafe {
        let hinstance = GetModuleHandleW(null());
        init_common_controls();
        register_window_classes(hinstance);
        let class = to_wide(MAIN_CLASS);
        let title = to_wide("Rust Todo Widget");
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW,
            class.as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_VISIBLE | WS_THICKFRAME,
            x,
            y,
            width,
            height,
            0,
            0,
            hinstance,
            null_mut(),
        );
        if hwnd == 0 {
            message_box(
                0,
                "Rust Todo Widget",
                "创建主窗口失败",
                MB_OK | MB_ICONERROR,
            );
            return;
        }
        apply_window_settings(hwnd);
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe fn register_window_classes(hinstance: HINSTANCE) {
    unsafe {
        register_class(hinstance, MAIN_CLASS, Some(main_wnd_proc));
        register_class(hinstance, TODO_FORM_CLASS, Some(todo_form_proc));
        register_class(hinstance, SEARCH_FORM_CLASS, Some(search_form_proc));
        register_class(hinstance, SETTINGS_FORM_CLASS, Some(settings_form_proc));
        register_class(hinstance, TEXT_FORM_CLASS, Some(text_form_proc));
    }
}

unsafe fn init_common_controls() {
    let controls = INITCOMMONCONTROLSEX {
        dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as DWORD,
        dwICC: ICC_DATE_CLASSES,
    };
    unsafe {
        InitCommonControlsEx(&controls);
    }
}

unsafe fn register_class(hinstance: HINSTANCE, name: &str, proc: WNDPROC) {
    let class = to_wide(name);
    let wc = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
        lpfnWndProc: proc,
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: unsafe { LoadIconW(0, IDI_APPLICATION) },
        hCursor: unsafe { LoadCursorW(0, IDC_ARROW) },
        hbrBackground: 0,
        lpszMenuName: null(),
        lpszClassName: class.as_ptr(),
    };
    unsafe {
        RegisterClassW(&wc);
    }
}

unsafe extern "system" fn main_wnd_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            unsafe {
                SetTimer(hwnd, TIMER_AUTO_SUMMARY, 60_000, 0);
                apply_window_settings(hwnd);
            }
            0
        }
        WM_GETMINMAXINFO => {
            let info = lparam as *mut MINMAXINFO;
            if !info.is_null() {
                unsafe {
                    (*info).ptMinTrackSize.x = MIN_MAIN_WIDTH;
                    (*info).ptMinTrackSize.y = MIN_MAIN_HEIGHT;
                }
            }
            0
        }
        WM_NCHITTEST => unsafe { hit_test_resize(hwnd, lparam) as LRESULT },
        WM_SIZE => {
            unsafe {
                sync_window_bounds(hwnd);
            }
            0
        }
        WM_PAINT => {
            unsafe {
                paint_main(hwnd);
            }
            0
        }
        WM_LBUTTONDOWN => {
            let x = get_x_lparam(lparam);
            let y = get_y_lparam(lparam);
            unsafe {
                handle_left_down(hwnd, x, y);
            }
            0
        }
        WM_MOUSEMOVE => {
            unsafe {
                handle_mouse_move(hwnd);
            }
            0
        }
        WM_LBUTTONUP => {
            unsafe {
                handle_left_up(hwnd);
            }
            0
        }
        WM_LBUTTONDBLCLK => {
            let x = get_x_lparam(lparam);
            let y = get_y_lparam(lparam);
            unsafe {
                let row = {
                    let app = app_lock();
                    app.row_at(x, y)
                };
                if let Some(id) = row {
                    let result = {
                        let mut app = app_lock();
                        app.complete_task(id)
                    };
                    show_error_if_needed(hwnd, result);
                    InvalidateRect(hwnd, null(), 1);
                }
            }
            0
        }
        WM_RBUTTONUP => {
            let x = get_x_lparam(lparam);
            let y = get_y_lparam(lparam);
            unsafe {
                handle_right_click(hwnd, x, y);
            }
            0
        }
        WM_MOUSEWHEEL => {
            unsafe {
                handle_mouse_wheel(hwnd, wheel_delta(wparam));
            }
            0
        }
        WM_TIMER => {
            if wparam == TIMER_AUTO_SUMMARY {
                unsafe {
                    maybe_start_auto_summary(hwnd);
                }
            }
            0
        }
        WM_SUMMARY_READY => {
            unsafe {
                handle_summary_ready(hwnd);
            }
            0
        }
        WM_CLOSE => {
            unsafe {
                DestroyWindow(hwnd);
            }
            0
        }
        WM_DESTROY => {
            unsafe {
                KillTimer(hwnd, TIMER_AUTO_SUMMARY);
            }
            if let Some(app) = APP.get() {
                let mut app = app.lock().unwrap();
                let _ = app.save_settings("已保存设置");
            }
            unsafe {
                PostQuitMessage(0);
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe fn paint_main(hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
    let mut client = RECT::default();
    unsafe {
        GetClientRect(hwnd, &mut client);
    }
    let width = client.right - client.left;
    let height = client.bottom - client.top;

    let theme = {
        let app = app_lock();
        Theme::from_settings(&app.settings)
    };
    let bg = unsafe { CreateSolidBrush(theme.window_bg) };
    unsafe {
        FillRect(hdc, &client, bg);
        DeleteObject(bg as HGDIOBJ);
        SetBkMode(hdc, TRANSPARENT);
    }

    {
        let mut app = app_lock();
        let theme = Theme::from_settings(&app.settings);
        app.button_rects = layout_toolbar(width);
        let now = LocalDateTime::now();

        fill_round_rect(
            hdc,
            RectI {
                left: 12,
                top: 12,
                right: width - 12,
                bottom: 104,
            },
            theme.header_bg,
            18,
        );

        draw_text(
            hdc,
            RectI {
                left: 26,
                top: 22,
                right: width - 210,
                bottom: 56,
            },
            "桌面待办",
            theme.text_primary,
            app.fonts.title,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        draw_text(
            hdc,
            RectI {
                left: 28,
                top: 58,
                right: width - 190,
                bottom: 86,
            },
            &format!(
                "{}  {}  ·  拖动顶部移动，拖动边缘调大小",
                now.date_string(),
                now.cn_weekday()
            ),
            theme.text_secondary,
            app.fonts.small,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );

        for (action, rc) in app.button_rects.clone() {
            draw_button(hdc, &app, &theme, action, rc);
        }

        let active = app.active_sorted();
        let top = 118;
        let row_h = 86;
        let footer_h = 48;
        let visible_rows = ((height - top - footer_h).max(row_h) / row_h) as usize;
        let max_scroll = active.len().saturating_sub(visible_rows);
        app.scroll = app.scroll.min(max_scroll);
        app.row_rects.clear();

        if active.is_empty() {
            fill_round_rect(
                hdc,
                RectI {
                    left: 14,
                    top: 126,
                    right: width - 14,
                    bottom: (height - footer_h - 10).max(188),
                },
                theme.card_bg,
                18,
            );
            draw_text(
                hdc,
                RectI {
                    left: 28,
                    top: 166,
                    right: width - 28,
                    bottom: 228,
                },
                "暂无待办，点击右上角 + 新建一条任务",
                theme.text_secondary,
                app.fonts.row,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
        } else {
            for (index, task) in active
                .iter()
                .skip(app.scroll)
                .take(visible_rows)
                .enumerate()
            {
                let y = top + index as i32 * row_h;
                let rc = RectI {
                    left: 14,
                    top: y,
                    right: width - 14,
                    bottom: y + row_h - 10,
                };
                app.row_rects.push((task.id, rc));
                draw_task_row(hdc, &app, &theme, task, rc, now);
            }
        }

        let footer = if app.summary_in_progress {
            "周报生成中..."
        } else {
            &app.status
        };
        fill_round_rect(
            hdc,
            RectI {
                left: 14,
                top: height - 42,
                right: width - 14,
                bottom: height - 12,
            },
            theme.footer_bg,
            14,
        );
        draw_text(
            hdc,
            RectI {
                left: 26,
                top: height - 38,
                right: width - 26,
                bottom: height - 14,
            },
            footer,
            theme.text_secondary,
            app.fonts.small,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
    }

    unsafe {
        EndPaint(hwnd, &ps);
    }
}

fn layout_toolbar(width: i32) -> Vec<(ToolbarAction, RectI)> {
    let actions = [
        ToolbarAction::Add,
        ToolbarAction::Complete,
        ToolbarAction::Search,
        ToolbarAction::Summary,
        ToolbarAction::Settings,
        ToolbarAction::Close,
    ];
    let mut x = width - 212;
    let mut out = Vec::new();
    for action in actions {
        out.push((
            action,
            RectI {
                left: x,
                top: 26,
                right: x + 28,
                bottom: 54,
            },
        ));
        x += 32;
    }
    out
}

fn draw_button(hdc: HDC, app: &AppState, theme: &Theme, action: ToolbarAction, rc: RectI) {
    let label = match action {
        ToolbarAction::Add => "+",
        ToolbarAction::Complete => "完",
        ToolbarAction::Search => "查",
        ToolbarAction::Summary => "周",
        ToolbarAction::Settings => "设",
        ToolbarAction::Close => "×",
    };
    let color = match action {
        ToolbarAction::Close => rgb(192, 66, 66),
        ToolbarAction::Complete => rgb(37, 128, 85),
        ToolbarAction::Summary => rgb(59, 103, 189),
        ToolbarAction::Add => rgb(42, 92, 196),
        _ => theme.text_primary,
    };
    fill_round_rect(hdc, rc, theme.button_bg, 10);
    draw_text(
        hdc,
        rc,
        label,
        color,
        app.fonts.icon,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
}

fn draw_task_row(
    hdc: HDC,
    app: &AppState,
    theme: &Theme,
    task: &Task,
    rc: RectI,
    now: LocalDateTime,
) {
    let selected = app.selected_id == Some(task.id);
    fill_round_rect(
        hdc,
        rc,
        if selected {
            theme.selected_bg
        } else {
            theme.card_bg
        },
        16,
    );

    let diff = task.due.diff_minutes_from(now);
    let accent = if diff <= 0 {
        rgb(205, 63, 63)
    } else if diff <= 1440 {
        rgb(220, 124, 24)
    } else if diff <= 2880 {
        rgb(185, 142, 36)
    } else {
        rgb(59, 103, 189)
    };
    fill_round_rect(
        hdc,
        RectI {
            left: rc.left + 10,
            top: rc.top + 12,
            right: rc.left + 18,
            bottom: rc.bottom - 12,
        },
        accent,
        8,
    );
    draw_text(
        hdc,
        RectI {
            left: rc.left + 28,
            top: rc.top + 10,
            right: rc.right - 104,
            bottom: rc.top + 38,
        },
        &task.title,
        theme.text_primary,
        app.fonts.row,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
    );
    fill_round_rect(
        hdc,
        RectI {
            left: rc.right - 102,
            top: rc.top + 12,
            right: rc.right - 14,
            bottom: rc.top + 38,
        },
        accent,
        12,
    );
    draw_text(
        hdc,
        RectI {
            left: rc.right - 98,
            top: rc.top + 12,
            right: rc.right - 18,
            bottom: rc.top + 38,
        },
        &task.due.short_string(),
        rgb(255, 255, 255),
        app.fonts.small,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    let note = if task.note.trim().is_empty() {
        due_status(diff)
    } else {
        task.note.trim()
    };
    draw_text(
        hdc,
        RectI {
            left: rc.left + 28,
            top: rc.top + 42,
            right: rc.right - 16,
            bottom: rc.bottom - 10,
        },
        note,
        theme.text_secondary,
        app.fonts.small,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
    );
}

fn draw_text(hdc: HDC, rc: RectI, text: &str, color: COLORREF, font: HFONT, flags: UINT) {
    let wide = to_wide(text);
    let mut rect = rc.to_rect();
    unsafe {
        SelectObject(hdc, font as HGDIOBJ);
        SetTextColor(hdc, color);
        DrawTextW(hdc, wide.as_ptr(), -1, &mut rect, flags);
    }
}

fn fill_round_rect(hdc: HDC, rc: RectI, color: COLORREF, radius: i32) {
    let brush = unsafe { CreateSolidBrush(color) };
    let pen = unsafe { CreatePen(PS_SOLID, 1, color) };
    unsafe {
        let old_brush = SelectObject(hdc, brush as HGDIOBJ);
        let old_pen = SelectObject(hdc, pen as HGDIOBJ);
        RoundRect(hdc, rc.left, rc.top, rc.right, rc.bottom, radius, radius);
        if old_pen != 0 {
            SelectObject(hdc, old_pen);
        }
        if old_brush != 0 {
            SelectObject(hdc, old_brush);
        }
        DeleteObject(pen as HGDIOBJ);
        DeleteObject(brush as HGDIOBJ);
    }
}

unsafe fn handle_left_down(hwnd: HWND, x: i32, y: i32) {
    let action = {
        let app = app_lock();
        app.action_at(x, y)
    };

    if let Some(action) = action {
        unsafe {
            handle_toolbar_action(hwnd, action);
        }
        return;
    }

    let row = {
        let app = app_lock();
        app.row_at(x, y)
    };
    if let Some(id) = row {
        app_lock().selected_id = Some(id);
        unsafe {
            InvalidateRect(hwnd, null(), 1);
        }
        return;
    }

    if y < 104 {
        unsafe {
            start_window_drag(hwnd);
        }
    }
}

unsafe fn start_window_drag(hwnd: HWND) {
    let mut cursor = POINT::default();
    let mut window = RECT::default();
    unsafe {
        GetCursorPos(&mut cursor);
        GetWindowRect(hwnd, &mut window);
        SetCapture(hwnd);
    }
    let mut app = app_lock();
    app.dragging = Some(DragState {
        cursor_start: cursor,
        window_start: window,
    });
}

unsafe fn handle_mouse_move(hwnd: HWND) {
    let dragging = {
        let app = app_lock();
        app.dragging
    };
    let Some(dragging) = dragging else {
        return;
    };

    let mut cursor = POINT::default();
    unsafe {
        GetCursorPos(&mut cursor);
    }
    let dx = cursor.x - dragging.cursor_start.x;
    let dy = cursor.y - dragging.cursor_start.y;
    let width = dragging.window_start.right - dragging.window_start.left;
    let height = dragging.window_start.bottom - dragging.window_start.top;
    unsafe {
        MoveWindow(
            hwnd,
            dragging.window_start.left + dx,
            dragging.window_start.top + dy,
            width,
            height,
            1,
        );
    }
}

unsafe fn handle_left_up(hwnd: HWND) {
    let was_dragging = {
        let mut app = app_lock();
        app.dragging.take().is_some()
    };
    if !was_dragging {
        return;
    }

    unsafe {
        ReleaseCapture();
        sync_window_bounds(hwnd);
        apply_window_settings(hwnd);
    }
}

unsafe fn handle_toolbar_action(hwnd: HWND, action: ToolbarAction) {
    match action {
        ToolbarAction::Add => unsafe {
            add_or_edit_task(hwnd, None);
        },
        ToolbarAction::Complete => {
            let selected = {
                let app = app_lock();
                app.selected_id
            };
            if let Some(id) = selected {
                let result = {
                    let mut app = app_lock();
                    app.complete_task(id)
                };
                unsafe {
                    show_error_if_needed(hwnd, result);
                    InvalidateRect(hwnd, null(), 1);
                }
            } else {
                unsafe {
                    message_box(hwnd, "待办", "请先选中一条待办", MB_OK | MB_ICONINFORMATION);
                }
            }
        }
        ToolbarAction::Search => unsafe {
            show_search_form(hwnd);
        },
        ToolbarAction::Summary => unsafe {
            start_summary(hwnd, true);
        },
        ToolbarAction::Settings => unsafe {
            show_settings_form(hwnd);
        },
        ToolbarAction::Close => unsafe {
            DestroyWindow(hwnd);
        },
    }
}

unsafe fn handle_right_click(hwnd: HWND, x: i32, y: i32) {
    let row = {
        let app = app_lock();
        app.row_at(x, y)
    };
    let Some(id) = row else {
        return;
    };

    app_lock().selected_id = Some(id);
    unsafe {
        InvalidateRect(hwnd, null(), 1);
    }

    let menu = unsafe { CreatePopupMenu() };
    unsafe {
        append_menu(menu, MENU_COMPLETE, "标记完成");
        append_menu(menu, MENU_EDIT, "编辑");
        append_menu(menu, MENU_NOTE, "查看备注");
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        append_menu(menu, MENU_DELETE, "删除");
    }

    let mut pt = POINT { x, y };
    unsafe {
        ClientToScreen(hwnd, &mut pt);
    }
    let cmd = unsafe {
        TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY,
            pt.x,
            pt.y,
            0,
            hwnd,
            null(),
        )
    } as usize;
    unsafe {
        DestroyMenu(menu);
    }

    match cmd {
        MENU_COMPLETE => {
            let result = app_lock().complete_task(id);
            unsafe {
                show_error_if_needed(hwnd, result);
            }
        }
        MENU_EDIT => unsafe {
            add_or_edit_task(hwnd, Some(id));
        },
        MENU_NOTE => {
            let note = {
                let app = app_lock();
                app.tasks
                    .iter()
                    .find(|task| task.id == id)
                    .map(|task| {
                        format!(
                            "{}\n截止：{}\n\n{}",
                            task.title,
                            task.due.storage_string(),
                            if task.note.trim().is_empty() {
                                "无备注"
                            } else {
                                task.note.trim()
                            }
                        )
                    })
                    .unwrap_or_else(|| "未找到记录".to_string())
            };
            unsafe {
                show_text_window(hwnd, "待办详情", &note);
            }
        }
        MENU_DELETE => {
            let result = app_lock().delete_task(id);
            unsafe {
                show_error_if_needed(hwnd, result);
            }
        }
        _ => {}
    }
    unsafe {
        InvalidateRect(hwnd, null(), 1);
    }
}

unsafe fn handle_mouse_wheel(hwnd: HWND, delta: i16) {
    let mut client = RECT::default();
    unsafe {
        GetClientRect(hwnd, &mut client);
    }
    let mut app = app_lock();
    let active_len = app.active_sorted().len();
    let visible = ((client.bottom - 118 - 48).max(86) / 86) as usize;
    let max_scroll = active_len.saturating_sub(visible);
    if delta < 0 {
        app.scroll = (app.scroll + 1).min(max_scroll);
    } else {
        app.scroll = app.scroll.saturating_sub(1);
    }
    unsafe {
        InvalidateRect(hwnd, null(), 1);
    }
}

unsafe fn append_menu(menu: HMENU, id: usize, text: &str) {
    let text = to_wide(text);
    unsafe {
        AppendMenuW(menu, MF_STRING, id, text.as_ptr());
    }
}

unsafe fn add_or_edit_task(hwnd: HWND, id: Option<u64>) {
    let initial = id.and_then(|id| app_lock().tasks.iter().find(|task| task.id == id).cloned());
    let mut retry = initial.clone().map(|task| TodoFormResult {
        title: task.title,
        due: task.due.storage_string(),
        note: task.note,
    });

    loop {
        let form_result = unsafe { show_todo_form(hwnd, retry.as_ref()) };
        let Some(form) = form_result else {
            break;
        };
        let title = form.title.trim().to_string();
        if title.is_empty() {
            unsafe {
                message_box(hwnd, "待办", "待办事项不能为空", MB_OK | MB_ICONERROR);
            }
            retry = Some(form);
            continue;
        }
        let due = match LocalDateTime::parse(&form.due) {
            Ok(due) => due,
            Err(err) => {
                unsafe {
                    message_box(hwnd, "日期格式", &err, MB_OK | MB_ICONERROR);
                }
                retry = Some(form);
                continue;
            }
        };
        let result = if let Some(id) = id {
            app_lock().update_task(id, title, due, form.note)
        } else {
            app_lock().add_task(title, due, form.note)
        };
        unsafe {
            show_error_if_needed(hwnd, result);
            InvalidateRect(hwnd, null(), 1);
        }
        break;
    }
}

unsafe fn show_todo_form(owner: HWND, initial: Option<&TodoFormResult>) -> Option<TodoFormResult> {
    let initial_title = initial.map(|v| v.title.clone()).unwrap_or_default();
    let initial_due = initial
        .map(|v| v.due.clone())
        .unwrap_or_else(|| LocalDateTime::now().storage_string());
    let initial_note = initial.map(|v| v.note.clone()).unwrap_or_default();
    {
        let mut state = TODO_FORM.get().unwrap().lock().unwrap();
        *state = Some(TodoFormState {
            title: initial_title,
            due: initial_due,
            note: initial_note,
            result: None,
            done: false,
            title_edit: 0,
            due_date_picker: 0,
            due_time_edit: 0,
            note_edit: 0,
        });
    }
    unsafe {
        EnableWindow(owner, 0);
    }
    let hinstance = unsafe { GetModuleHandleW(null()) };
    let class = to_wide(TODO_FORM_CLASS);
    let title = to_wide(if initial.is_some() {
        "编辑待办"
    } else {
        "新建待办"
    });
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            class.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            450,
            350,
            owner,
            0,
            hinstance,
            null_mut(),
        )
    };
    if hwnd == 0 {
        unsafe {
            EnableWindow(owner, 1);
        }
        return None;
    }
    unsafe {
        modal_loop_until_done(owner, || {
            TODO_FORM
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .as_ref()
                .is_none_or(|state| state.done)
        });
        EnableWindow(owner, 1);
        SetFocus(owner);
    }
    TODO_FORM
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .take()
        .and_then(|state| state.result)
}

unsafe extern "system" fn todo_form_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            unsafe {
                create_todo_form_controls(hwnd);
            }
            0
        }
        WM_COMMAND => {
            let id = loword(wparam) as usize;
            match id {
                ID_TODO_SAVE => unsafe {
                    let result = {
                        let state = TODO_FORM.get().unwrap().lock().unwrap();
                        state.as_ref().map(|state| TodoFormResult {
                            title: get_text(state.title_edit),
                            due: get_todo_due_text(state),
                            note: get_text(state.note_edit),
                        })
                    };
                    {
                        let mut state = TODO_FORM.get().unwrap().lock().unwrap();
                        if let Some(state) = state.as_mut() {
                            state.result = result;
                            state.done = true;
                        }
                    }
                    DestroyWindow(hwnd);
                },
                ID_TODO_CANCEL => unsafe {
                    mark_todo_done();
                    DestroyWindow(hwnd);
                },
                _ => {}
            }
            0
        }
        WM_CLOSE => {
            unsafe {
                mark_todo_done();
                DestroyWindow(hwnd);
            }
            0
        }
        WM_DESTROY => {
            unsafe {
                mark_todo_done();
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe fn create_todo_form_controls(hwnd: HWND) {
    let font = app_lock().fonts.small;
    let hinstance = unsafe { GetModuleHandleW(null()) };
    unsafe {
        create_label(hwnd, hinstance, "待办事项", 20, 22, 80, 24, font);
        create_label(hwnd, hinstance, "截止时间", 20, 62, 80, 24, font);
        create_label(hwnd, hinstance, "备注", 20, 102, 80, 24, font);
    }
    let (title_value, due_value, note_value) = {
        let state = TODO_FORM.get().unwrap().lock().unwrap();
        let state = state.as_ref().unwrap();
        (state.title.clone(), state.due.clone(), state.note.clone())
    };
    let (due_date, due_time) = split_due_initial(&due_value);
    let title_edit = unsafe {
        create_edit(
            hwnd,
            hinstance,
            &title_value,
            110,
            20,
            300,
            26,
            ES_AUTOHSCROLL,
            ID_TODO_TITLE,
            font,
        )
    };
    let due_date_picker = unsafe {
        create_date_picker(
            hwnd,
            hinstance,
            Some(due_date),
            110,
            60,
            172,
            26,
            ID_TODO_DUE,
            font,
            false,
        )
    };
    let due_time_edit = unsafe {
        create_edit(
            hwnd,
            hinstance,
            &due_time,
            294,
            60,
            116,
            26,
            ES_AUTOHSCROLL,
            ID_TODO_TIME,
            font,
        )
    };
    let note_edit = unsafe {
        create_edit(
            hwnd,
            hinstance,
            &note_value,
            110,
            100,
            300,
            130,
            ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN | WS_VSCROLL,
            ID_TODO_NOTE,
            font,
        )
    };
    unsafe {
        create_button(
            hwnd,
            hinstance,
            "保存",
            230,
            255,
            82,
            30,
            ID_TODO_SAVE,
            font,
        );
        create_button(
            hwnd,
            hinstance,
            "取消",
            328,
            255,
            82,
            30,
            ID_TODO_CANCEL,
            font,
        );
        SetFocus(title_edit);
    }
    let mut state = TODO_FORM.get().unwrap().lock().unwrap();
    if let Some(state) = state.as_mut() {
        state.title_edit = title_edit;
        state.due_date_picker = due_date_picker;
        state.due_time_edit = due_time_edit;
        state.note_edit = note_edit;
    }
}

unsafe fn mark_todo_done() {
    let mut state = TODO_FORM.get().unwrap().lock().unwrap();
    if let Some(state) = state.as_mut() {
        state.done = true;
    }
}

unsafe fn show_search_form(owner: HWND) {
    {
        let mut state = SEARCH_FORM.get().unwrap().lock().unwrap();
        *state = Some(SearchFormState {
            done: false,
            name_edit: 0,
            start_picker: 0,
            end_picker: 0,
            result_edit: 0,
        });
    }
    unsafe {
        EnableWindow(owner, 0);
    }
    let hinstance = unsafe { GetModuleHandleW(null()) };
    let class = to_wide(SEARCH_FORM_CLASS);
    let title = to_wide("完成记录查询");
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            class.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            680,
            520,
            owner,
            0,
            hinstance,
            null_mut(),
        )
    };
    if hwnd != 0 {
        unsafe {
            modal_loop_until_done(owner, || {
                SEARCH_FORM
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .as_ref()
                    .is_none_or(|state| state.done)
            });
        }
    }
    unsafe {
        EnableWindow(owner, 1);
        SetFocus(owner);
    }
    let _ = SEARCH_FORM.get().unwrap().lock().unwrap().take();
}

unsafe extern "system" fn search_form_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            unsafe {
                create_search_controls(hwnd);
            }
            0
        }
        WM_COMMAND => {
            let id = loword(wparam) as usize;
            match id {
                ID_SEARCH_BUTTON => unsafe {
                    run_search(hwnd);
                },
                ID_SEARCH_CLOSE => unsafe {
                    mark_search_done();
                    DestroyWindow(hwnd);
                },
                _ => {}
            }
            0
        }
        WM_CLOSE => {
            unsafe {
                mark_search_done();
                DestroyWindow(hwnd);
            }
            0
        }
        WM_DESTROY => {
            unsafe {
                mark_search_done();
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe fn create_search_controls(hwnd: HWND) {
    let font = app_lock().fonts.small;
    let hinstance = unsafe { GetModuleHandleW(null()) };
    unsafe {
        create_label(hwnd, hinstance, "名称", 18, 20, 46, 24, font);
        create_label(hwnd, hinstance, "开始", 230, 20, 46, 24, font);
        create_label(hwnd, hinstance, "结束", 440, 20, 46, 24, font);
    }
    let name = unsafe {
        create_edit(
            hwnd,
            hinstance,
            "",
            66,
            18,
            150,
            26,
            ES_AUTOHSCROLL,
            ID_SEARCH_NAME,
            font,
        )
    };
    let start = unsafe {
        create_date_picker(
            hwnd,
            hinstance,
            None,
            278,
            18,
            148,
            26,
            ID_SEARCH_START,
            font,
            true,
        )
    };
    let end = unsafe {
        create_date_picker(
            hwnd,
            hinstance,
            None,
            488,
            18,
            148,
            26,
            ID_SEARCH_END,
            font,
            true,
        )
    };
    let result = unsafe {
        create_edit(
            hwnd,
            hinstance,
            "",
            18,
            62,
            618,
            360,
            ES_MULTILINE | ES_AUTOVSCROLL | ES_READONLY | WS_VSCROLL,
            ID_SEARCH_RESULT,
            font,
        )
    };
    unsafe {
        create_button(
            hwnd,
            hinstance,
            "查询",
            454,
            438,
            82,
            30,
            ID_SEARCH_BUTTON,
            font,
        );
        create_button(
            hwnd,
            hinstance,
            "关闭",
            554,
            438,
            82,
            30,
            ID_SEARCH_CLOSE,
            font,
        );
    }
    let mut state = SEARCH_FORM.get().unwrap().lock().unwrap();
    if let Some(state) = state.as_mut() {
        state.name_edit = name;
        state.start_picker = start;
        state.end_picker = end;
        state.result_edit = result;
    }
}

unsafe fn run_search(hwnd: HWND) {
    let (name, start, end, result_hwnd) = {
        let state = SEARCH_FORM.get().unwrap().lock().unwrap();
        let Some(state) = state.as_ref() else {
            return;
        };
        unsafe {
            (
                get_text(state.name_edit),
                get_optional_picker_datetime(state.start_picker, 0, 0),
                get_optional_picker_datetime(state.end_picker, 23, 59),
                state.result_edit,
            )
        }
    };
    if start.zip(end).is_some_and(|(start, end)| start > end) {
        unsafe {
            message_box(
                hwnd,
                "查询时间",
                "开始日期不能晚于结束日期",
                MB_OK | MB_ICONERROR,
            );
        }
        return;
    }
    let name = name.trim().to_lowercase();
    let mut results: Vec<Task> = {
        let app = app_lock();
        app.tasks
            .iter()
            .filter(|task| task.completed)
            .filter(|task| name.is_empty() || task.title.to_lowercase().contains(name.as_str()))
            .filter(|task| {
                let Some(done) = task.completed_at else {
                    return false;
                };
                start.is_none_or(|start| done >= start) && end.is_none_or(|end| done <= end)
            })
            .cloned()
            .collect()
    };
    results.sort_by_key(|task| task.completed_at.map(Reverse));
    let mut text = String::new();
    text.push_str(&format!("共找到 {} 条完成记录\r\n\r\n", results.len()));
    for task in results {
        text.push_str(&format!(
            "[{}] {}\r\n完成：{}\r\n截止：{}\r\n备注：{}\r\n\r\n",
            task.id,
            task.title,
            task.completed_at
                .map(|dt| dt.storage_string())
                .unwrap_or_else(|| "未记录".to_string()),
            task.due.storage_string(),
            if task.note.trim().is_empty() {
                "无"
            } else {
                task.note.trim()
            }
        ));
    }
    unsafe {
        set_text(result_hwnd, &text);
    }
}

unsafe fn mark_search_done() {
    let mut state = SEARCH_FORM.get().unwrap().lock().unwrap();
    if let Some(state) = state.as_mut() {
        state.done = true;
    }
}

unsafe fn show_settings_form(owner: HWND) {
    let settings = app_lock().settings.clone();
    {
        let mut state = SETTINGS_FORM.get().unwrap().lock().unwrap();
        *state = Some(SettingsFormState {
            settings,
            result: None,
            done: false,
            weekday_combo: 0,
            auto_check: 0,
            startup_check: 0,
            bg_edit: 0,
            alpha_edit: 0,
            api_edit: 0,
            model_edit: 0,
            key_edit: 0,
        });
    }
    unsafe {
        EnableWindow(owner, 0);
    }
    let hinstance = unsafe { GetModuleHandleW(null()) };
    let class = to_wide(SETTINGS_FORM_CLASS);
    let title = to_wide("设置");
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            class.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            560,
            452,
            owner,
            0,
            hinstance,
            null_mut(),
        )
    };
    if hwnd != 0 {
        unsafe {
            modal_loop_until_done(owner, || {
                SETTINGS_FORM
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .as_ref()
                    .is_none_or(|state| state.done)
            });
        }
    }
    unsafe {
        EnableWindow(owner, 1);
        SetFocus(owner);
    }
    let result = SETTINGS_FORM
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .take()
        .and_then(|state| state.result);
    if let Some(settings) = result {
        let save_result = {
            let mut app = app_lock();
            app.settings = settings.clone();
            app.save_settings("已保存设置")
        };
        match save_result {
            Ok(()) => {
                if let Err(err) = sync_launch_on_startup(settings.launch_on_startup) {
                    unsafe {
                        message_box(
                            owner,
                            "设置",
                            &format!("设置已保存，但同步开机自启动失败：{}", err),
                            MB_OK | MB_ICONERROR,
                        );
                    }
                }
                unsafe {
                    apply_window_settings(owner);
                    InvalidateRect(owner, null(), 1);
                }
            }
            Err(err) => unsafe {
                message_box(owner, "设置", &err, MB_OK | MB_ICONERROR);
            },
        }
    }
}

unsafe extern "system" fn settings_form_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            unsafe {
                create_settings_controls(hwnd);
            }
            0
        }
        WM_COMMAND => {
            let id = loword(wparam) as usize;
            match id {
                ID_SETTINGS_SAVE => unsafe {
                    match save_settings_from_form() {
                        Ok(()) => {
                            DestroyWindow(hwnd);
                        }
                        Err(err) => {
                            message_box(hwnd, "设置", &err, MB_OK | MB_ICONERROR);
                        }
                    }
                },
                ID_SETTINGS_BG_PICK => unsafe {
                    choose_background_color(hwnd);
                },
                ID_SETTINGS_CANCEL => unsafe {
                    mark_settings_done();
                    DestroyWindow(hwnd);
                },
                _ => {}
            }
            0
        }
        WM_CLOSE => {
            unsafe {
                mark_settings_done();
                DestroyWindow(hwnd);
            }
            0
        }
        WM_DESTROY => {
            unsafe {
                mark_settings_done();
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe fn create_settings_controls(hwnd: HWND) {
    let font = app_lock().fonts.small;
    let hinstance = unsafe { GetModuleHandleW(null()) };
    let settings = SETTINGS_FORM
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .settings
        .clone();

    unsafe {
        create_label(hwnd, hinstance, "周报日", 22, 24, 96, 24, font);
        create_label(hwnd, hinstance, "背景色", 22, 102, 96, 24, font);
        create_label(hwnd, hinstance, "透明度", 22, 144, 96, 24, font);
        create_label(hwnd, hinstance, "API URL", 22, 228, 96, 24, font);
        create_label(hwnd, hinstance, "模型名", 22, 270, 96, 24, font);
        create_label(hwnd, hinstance, "API Key", 22, 312, 96, 24, font);
        create_label(hwnd, hinstance, "或输入 #F5F7FA", 360, 102, 150, 24, font);
        create_label(hwnd, hinstance, "范围：30 - 255", 322, 144, 180, 24, font);
    }
    let combo = unsafe {
        create_control(
            hwnd,
            hinstance,
            "COMBOBOX",
            "",
            128,
            22,
            160,
            160,
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | CBS_DROPDOWNLIST | CBS_HASSTRINGS,
            0,
            ID_SETTINGS_WEEKDAY,
            font,
        )
    };
    for weekday in 1..=7 {
        let text = to_wide(weekday_name(weekday));
        unsafe {
            SendMessageW(combo, CB_ADDSTRING, 0, text.as_ptr() as LPARAM);
        }
    }
    unsafe {
        SendMessageW(
            combo,
            CB_SETCURSEL,
            settings.summary_weekday.saturating_sub(1) as WPARAM,
            0,
        );
    }
    let auto = unsafe {
        create_control(
            hwnd,
            hinstance,
            "BUTTON",
            "自动生成",
            128,
            60,
            140,
            26,
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_AUTOCHECKBOX,
            0,
            ID_SETTINGS_AUTO,
            font,
        )
    };
    let startup = unsafe {
        create_control(
            hwnd,
            hinstance,
            "BUTTON",
            "开机自启动",
            282,
            60,
            140,
            26,
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_AUTOCHECKBOX,
            0,
            ID_SETTINGS_STARTUP,
            font,
        )
    };
    unsafe {
        SendMessageW(
            auto,
            BM_SETCHECK,
            if settings.auto_summary {
                BST_CHECKED
            } else {
                BST_UNCHECKED
            },
            0,
        );
        SendMessageW(
            startup,
            BM_SETCHECK,
            if settings.launch_on_startup {
                BST_CHECKED
            } else {
                BST_UNCHECKED
            },
            0,
        );
    }
    let bg = unsafe {
        create_edit(
            hwnd,
            hinstance,
            &settings.background_color,
            128,
            100,
            146,
            26,
            ES_AUTOHSCROLL,
            ID_SETTINGS_BG,
            font,
        )
    };
    unsafe {
        create_button(
            hwnd,
            hinstance,
            "选择",
            284,
            100,
            64,
            26,
            ID_SETTINGS_BG_PICK,
            font,
        );
    }
    let alpha = unsafe {
        create_edit(
            hwnd,
            hinstance,
            &settings.window_alpha.to_string(),
            128,
            142,
            180,
            26,
            ES_AUTOHSCROLL,
            ID_SETTINGS_ALPHA,
            font,
        )
    };
    let api = unsafe {
        create_edit(
            hwnd,
            hinstance,
            &settings.llm_api_url,
            128,
            228,
            390,
            26,
            ES_AUTOHSCROLL,
            ID_SETTINGS_API,
            font,
        )
    };
    let model = unsafe {
        create_edit(
            hwnd,
            hinstance,
            &settings.llm_model,
            128,
            270,
            390,
            26,
            ES_AUTOHSCROLL,
            ID_SETTINGS_MODEL,
            font,
        )
    };
    let key = unsafe {
        create_edit(
            hwnd,
            hinstance,
            &settings.llm_api_key,
            128,
            312,
            390,
            26,
            ES_AUTOHSCROLL | ES_PASSWORD,
            ID_SETTINGS_KEY,
            font,
        )
    };
    unsafe {
        create_button(
            hwnd,
            hinstance,
            "保存",
            376,
            376,
            82,
            30,
            ID_SETTINGS_SAVE,
            font,
        );
        create_button(
            hwnd,
            hinstance,
            "取消",
            470,
            376,
            82,
            30,
            ID_SETTINGS_CANCEL,
            font,
        );
    }
    let mut state = SETTINGS_FORM.get().unwrap().lock().unwrap();
    if let Some(state) = state.as_mut() {
        state.weekday_combo = combo;
        state.auto_check = auto;
        state.startup_check = startup;
        state.bg_edit = bg;
        state.alpha_edit = alpha;
        state.api_edit = api;
        state.model_edit = model;
        state.key_edit = key;
    }
}

unsafe fn choose_background_color(owner: HWND) {
    let bg_edit = {
        let state = SETTINGS_FORM.get().unwrap().lock().unwrap();
        let Some(state) = state.as_ref() else {
            return;
        };
        state.bg_edit
    };
    if bg_edit == 0 {
        return;
    }

    let initial = unsafe { get_text(bg_edit) };
    let initial = parse_hex_color(&initial)
        .map(|(r, g, b)| rgb(r, g, b))
        .unwrap_or_else(|_| rgb(245, 247, 250));
    let mut custom_colors = [
        rgb(245, 247, 250),
        rgb(255, 255, 255),
        rgb(239, 246, 255),
        rgb(240, 253, 244),
        rgb(255, 247, 237),
        rgb(250, 245, 255),
        rgb(254, 242, 242),
        rgb(244, 244, 245),
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    let mut chooser = CHOOSECOLORW {
        lStructSize: std::mem::size_of::<CHOOSECOLORW>() as DWORD,
        hwndOwner: owner,
        hInstance: 0,
        rgbResult: initial,
        lpCustColors: custom_colors.as_mut_ptr(),
        Flags: CC_RGBINIT | CC_FULLOPEN,
        lCustData: 0,
        lpfnHook: 0,
        lpTemplateName: null(),
    };

    if unsafe { ChooseColorW(&mut chooser) } != 0 {
        let value = colorref_to_hex(chooser.rgbResult);
        unsafe {
            set_text(bg_edit, &value);
        }
    }
}

unsafe fn save_settings_from_form() -> Result<(), String> {
    let mut state = SETTINGS_FORM.get().unwrap().lock().unwrap();
    let Some(state) = state.as_mut() else {
        return Ok(());
    };
    let weekday = unsafe { SendMessageW(state.weekday_combo, CB_GETCURSEL, 0, 0) };
    let mut settings = state.settings.clone();
    settings.summary_weekday = (weekday as u32 + 1).clamp(1, 7);
    settings.auto_summary =
        unsafe { SendMessageW(state.auto_check, BM_GETCHECK, 0, 0) } == BST_CHECKED as LRESULT;
    settings.launch_on_startup =
        unsafe { SendMessageW(state.startup_check, BM_GETCHECK, 0, 0) } == BST_CHECKED as LRESULT;
    unsafe {
        settings.background_color = normalize_color_text(&get_text(state.bg_edit))?;
        settings.window_alpha = parse_alpha_text(&get_text(state.alpha_edit))?;
        settings.llm_api_url = get_text(state.api_edit);
        settings.llm_model = get_text(state.model_edit);
        settings.llm_api_key = get_text(state.key_edit);
    }
    state.result = Some(settings);
    state.done = true;
    Ok(())
}

unsafe fn mark_settings_done() {
    let mut state = SETTINGS_FORM.get().unwrap().lock().unwrap();
    if let Some(state) = state.as_mut() {
        state.done = true;
    }
}

unsafe fn apply_window_settings(hwnd: HWND) {
    let alpha = {
        let app = app_lock();
        app.settings.window_alpha.clamp(30, 255)
    };
    unsafe {
        SetLayeredWindowAttributes(hwnd, 0, alpha as BYTE, LWA_ALPHA);
        SetWindowPos(
            hwnd,
            HWND_BOTTOM,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

unsafe fn sync_window_bounds(hwnd: HWND) {
    let mut window = RECT::default();
    unsafe {
        GetWindowRect(hwnd, &mut window);
    }
    let mut app = app_lock();
    app.settings.window_x = window.left;
    app.settings.window_y = window.top;
    app.settings.window_width = (window.right - window.left).max(MIN_MAIN_WIDTH);
    app.settings.window_height = (window.bottom - window.top).max(MIN_MAIN_HEIGHT);
    let _ = app.store.save_settings(&app.settings);
}

unsafe fn hit_test_resize(hwnd: HWND, lparam: LPARAM) -> WPARAM {
    let mut window = RECT::default();
    unsafe {
        GetWindowRect(hwnd, &mut window);
    }
    let x = get_x_lparam(lparam);
    let y = get_y_lparam(lparam);
    let left = x < window.left + RESIZE_BORDER;
    let right = x >= window.right - RESIZE_BORDER;
    let top = y < window.top + RESIZE_BORDER;
    let bottom = y >= window.bottom - RESIZE_BORDER;

    if top && left {
        HTTOPLEFT
    } else if top && right {
        HTTOPRIGHT
    } else if bottom && left {
        HTBOTTOMLEFT
    } else if bottom && right {
        HTBOTTOMRIGHT
    } else if left {
        HTLEFT
    } else if right {
        HTRIGHT
    } else if top {
        HTTOP
    } else if bottom {
        HTBOTTOM
    } else {
        HTCLIENT
    }
}

fn sync_launch_on_startup(enabled: bool) -> Result<(), String> {
    let command = if enabled {
        let exe = std::env::current_exe()
            .map_err(|err| format!("读取程序路径失败：{}", err))?
            .to_string_lossy()
            .into_owned();
        Some(format!("\"{}\"", exe))
    } else {
        None
    };
    unsafe { set_run_at_startup(STARTUP_VALUE_NAME, command.as_deref()) }
}

fn normalize_color_text(value: &str) -> Result<String, String> {
    let (r, g, b) = parse_hex_color(value)?;
    Ok(format!("#{:02X}{:02X}{:02X}", r, g, b))
}

fn colorref_to_hex(color: COLORREF) -> String {
    let r = (color & 0xff) as u8;
    let g = ((color >> 8) & 0xff) as u8;
    let b = ((color >> 16) & 0xff) as u8;
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

fn parse_alpha_text(value: &str) -> Result<u8, String> {
    let alpha = value
        .trim()
        .parse::<u16>()
        .map_err(|_| "透明度请输入 30 到 255 之间的数字".to_string())?;
    if !(30..=255).contains(&alpha) {
        return Err("透明度请输入 30 到 255 之间的数字".to_string());
    }
    Ok(alpha as u8)
}

fn parse_hex_color(value: &str) -> Result<(u8, u8, u8), String> {
    let trimmed = value.trim().trim_start_matches('#');
    if trimmed.len() != 6 {
        return Err("背景色请使用 6 位十六进制格式，例如 #F5F7FA".to_string());
    }
    let r = u8::from_str_radix(&trimmed[0..2], 16)
        .map_err(|_| "背景色请使用 6 位十六进制格式，例如 #F5F7FA".to_string())?;
    let g = u8::from_str_radix(&trimmed[2..4], 16)
        .map_err(|_| "背景色请使用 6 位十六进制格式，例如 #F5F7FA".to_string())?;
    let b = u8::from_str_radix(&trimmed[4..6], 16)
        .map_err(|_| "背景色请使用 6 位十六进制格式，例如 #F5F7FA".to_string())?;
    Ok((r, g, b))
}

fn mix_rgb(base: (u8, u8, u8), target: (u8, u8, u8), percent: u8) -> COLORREF {
    rgb(
        mix_channel(base.0, target.0, percent),
        mix_channel(base.1, target.1, percent),
        mix_channel(base.2, target.2, percent),
    )
}

fn mix_channel(base: u8, target: u8, percent: u8) -> u8 {
    let base = base as i32;
    let target = target as i32;
    (base + (target - base) * percent as i32 / 100) as u8
}

fn due_status(diff_minutes: i64) -> &'static str {
    if diff_minutes <= 0 {
        "已逾期，建议优先处理"
    } else if diff_minutes <= 1440 {
        "今天截止，请尽快完成"
    } else if diff_minutes <= 2880 {
        "明后天截止，可提前安排"
    } else {
        "进度稳定，可按计划推进"
    }
}

unsafe fn show_text_window(owner: HWND, title: &str, text: &str) {
    {
        let mut state = TEXT_FORM.get().unwrap().lock().unwrap();
        *state = Some(TextFormState {
            text: text.to_string(),
            done: false,
        });
    }
    unsafe {
        EnableWindow(owner, 0);
    }
    let hinstance = unsafe { GetModuleHandleW(null()) };
    let class = to_wide(TEXT_FORM_CLASS);
    let title_w = to_wide(title);
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            class.as_ptr(),
            title_w.as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            680,
            540,
            owner,
            0,
            hinstance,
            null_mut(),
        )
    };
    if hwnd != 0 {
        unsafe {
            modal_loop_until_done(owner, || {
                TEXT_FORM
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .as_ref()
                    .is_none_or(|state| state.done)
            });
        }
    }
    unsafe {
        EnableWindow(owner, 1);
        SetFocus(owner);
    }
    let _ = TEXT_FORM.get().unwrap().lock().unwrap().take();
}

unsafe extern "system" fn text_form_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            unsafe {
                create_text_controls(hwnd);
            }
            0
        }
        WM_COMMAND => {
            if loword(wparam) as usize == ID_TEXT_CLOSE {
                unsafe {
                    mark_text_done();
                    DestroyWindow(hwnd);
                }
            }
            0
        }
        WM_CLOSE => {
            unsafe {
                mark_text_done();
                DestroyWindow(hwnd);
            }
            0
        }
        WM_DESTROY => {
            unsafe {
                mark_text_done();
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe fn create_text_controls(hwnd: HWND) {
    let font = app_lock().fonts.small;
    let hinstance = unsafe { GetModuleHandleW(null()) };
    let text = TEXT_FORM
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .as_ref()
        .map(|state| state.text.clone())
        .unwrap_or_default();
    unsafe {
        create_edit(
            hwnd,
            hinstance,
            &text,
            18,
            18,
            626,
            410,
            ES_MULTILINE | ES_AUTOVSCROLL | ES_READONLY | WS_VSCROLL,
            0,
            font,
        );
        create_button(
            hwnd,
            hinstance,
            "关闭",
            562,
            444,
            82,
            30,
            ID_TEXT_CLOSE,
            font,
        );
    }
}

unsafe fn mark_text_done() {
    let mut state = TEXT_FORM.get().unwrap().lock().unwrap();
    if let Some(state) = state.as_mut() {
        state.done = true;
    }
}

unsafe fn maybe_start_auto_summary(hwnd: HWND) {
    let now = LocalDateTime::now();
    let should_start = {
        let app = app_lock();
        app.settings.auto_summary
            && !app.summary_in_progress
            && app.settings.summary_weekday == now.weekday_mon1()
            && app.settings.last_summary_date != now.date_string()
    };
    if should_start {
        unsafe {
            start_summary(hwnd, false);
        }
    }
}

unsafe fn start_summary(hwnd: HWND, manual: bool) {
    let (store, settings, tasks, start, end) = {
        let mut app = app_lock();
        if app.summary_in_progress {
            app.status = "周报已经在生成中".to_string();
            unsafe {
                InvalidateRect(hwnd, null(), 1);
            }
            return;
        }
        let end = LocalDateTime::now();
        let start = end.add_days(-7);
        let tasks = app.completed_between(start, end);
        app.summary_in_progress = true;
        app.status = "正在生成周报...".to_string();
        unsafe {
            InvalidateRect(hwnd, null(), 1);
        }
        (app.store.clone(), app.settings.clone(), tasks, start, end)
    };

    thread::spawn(move || {
        let result = generate_and_store_summary(&store, &settings, &tasks, start, end);
        if let Some(slot) = SUMMARY_PAYLOAD.get() {
            *slot.lock().unwrap() = Some(SummaryPayload { result, manual });
        }
        unsafe {
            PostMessageW(hwnd, WM_SUMMARY_READY, 0, 0);
        }
    });
}

unsafe fn handle_summary_ready(hwnd: HWND) {
    let payload = SUMMARY_PAYLOAD.get().unwrap().lock().unwrap().take();
    let Some(payload) = payload else {
        return;
    };
    let mut text_to_show: Option<(String, String)> = None;
    {
        let mut app = app_lock();
        app.summary_in_progress = false;
        match payload.result {
            Ok(result) => {
                app.settings.last_summary_date = result.date.clone();
                let _ = app.store.save_settings(&app.settings);
                app.status = format!("{}：{}", result.note, result.path.display());
                if payload.manual {
                    let title = if result.used_llm {
                        "周报（大模型）"
                    } else {
                        "周报（本地草稿）"
                    }
                    .to_string();
                    text_to_show = Some((title, result.text));
                }
            }
            Err(err) => {
                app.status = format!("周报生成失败：{}", err);
                if payload.manual {
                    text_to_show = Some(("周报生成失败".to_string(), err));
                }
            }
        }
    }
    unsafe {
        InvalidateRect(hwnd, null(), 1);
    }
    if let Some((title, text)) = text_to_show {
        unsafe {
            show_text_window(hwnd, &title, &text);
        }
    }
}

unsafe fn modal_loop_until_done<F: Fn() -> bool>(owner: HWND, is_done: F) {
    let mut msg = MSG::default();
    while !is_done() {
        let result = unsafe { GetMessageW(&mut msg, 0, 0, 0) };
        if result <= 0 {
            break;
        }
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if unsafe { IsWindow(owner) } == 0 {
            break;
        }
    }
}

fn split_due_initial(value: &str) -> (LocalDateTime, String) {
    if let Ok(dt) = LocalDateTime::parse(value) {
        return (dt, format!("{:02}:{:02}", dt.hour, dt.minute));
    }

    let mut parts = value.split_whitespace();
    let date = parts
        .next()
        .and_then(|date| LocalDateTime::parse_start_bound(date).ok().flatten())
        .unwrap_or_else(LocalDateTime::now);
    let time = parts
        .next()
        .filter(|time| !time.trim().is_empty())
        .unwrap_or("18:00")
        .to_string();
    (date, time)
}

unsafe fn get_todo_due_text(state: &TodoFormState) -> String {
    let date = unsafe { get_optional_picker_datetime(state.due_date_picker, 18, 0) }
        .unwrap_or_else(LocalDateTime::now);
    let time = unsafe { get_text(state.due_time_edit) };
    let time = time.trim();
    if time.is_empty() {
        date.date_string()
    } else {
        format!("{} {}", date.date_string(), time)
    }
}

unsafe fn get_optional_picker_datetime(
    hwnd: HWND,
    default_hour: u32,
    default_minute: u32,
) -> Option<LocalDateTime> {
    let mut st = SYSTEMTIME::default();
    let status = unsafe {
        SendMessageW(
            hwnd,
            DTM_GETSYSTEMTIME,
            0,
            (&mut st as *mut SYSTEMTIME) as LPARAM,
        )
    };
    if status != GDT_VALID as LRESULT {
        return None;
    }
    let dt = LocalDateTime {
        year: st.wYear as i32,
        month: st.wMonth as u32,
        day: st.wDay as u32,
        hour: default_hour,
        minute: default_minute,
    };
    dt.validate().ok().map(|_| dt)
}

fn system_time_from_date(date: LocalDateTime) -> SYSTEMTIME {
    SYSTEMTIME {
        wYear: date.year as WORD,
        wMonth: date.month as WORD,
        wDayOfWeek: 0,
        wDay: date.day as WORD,
        wHour: 0,
        wMinute: 0,
        wSecond: 0,
        wMilliseconds: 0,
    }
}

unsafe fn create_label(
    parent: HWND,
    hinstance: HINSTANCE,
    text: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    font: HFONT,
) -> HWND {
    unsafe {
        create_control(
            parent,
            hinstance,
            "STATIC",
            text,
            x,
            y,
            w,
            h,
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            font,
        )
    }
}

unsafe fn create_button(
    parent: HWND,
    hinstance: HINSTANCE,
    text: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    id: usize,
    font: HFONT,
) -> HWND {
    unsafe {
        create_control(
            parent,
            hinstance,
            "BUTTON",
            text,
            x,
            y,
            w,
            h,
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON,
            0,
            id,
            font,
        )
    }
}

unsafe fn create_date_picker(
    parent: HWND,
    hinstance: HINSTANCE,
    initial: Option<LocalDateTime>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    id: usize,
    font: HFONT,
    allow_none: bool,
) -> HWND {
    let style = WS_CHILD | WS_VISIBLE | WS_TABSTOP | if allow_none { DTS_SHOWNONE } else { 0 };
    let hwnd = unsafe {
        create_control(
            parent,
            hinstance,
            "SysDateTimePick32",
            "",
            x,
            y,
            w,
            h,
            style,
            0,
            id,
            font,
        )
    };
    if hwnd != 0 {
        let format = to_wide("yyyy-MM-dd");
        unsafe {
            SendMessageW(hwnd, DTM_SETFORMATW, 0, format.as_ptr() as LPARAM);
        }
        if let Some(date) = initial {
            let mut st = system_time_from_date(date);
            unsafe {
                SendMessageW(
                    hwnd,
                    DTM_SETSYSTEMTIME,
                    GDT_VALID,
                    (&mut st as *mut SYSTEMTIME) as LPARAM,
                );
            }
        } else if allow_none {
            unsafe {
                SendMessageW(hwnd, DTM_SETSYSTEMTIME, GDT_NONE, 0);
            }
        }
    }
    hwnd
}

unsafe fn create_edit(
    parent: HWND,
    hinstance: HINSTANCE,
    text: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    style: DWORD,
    id: usize,
    font: HFONT,
) -> HWND {
    unsafe {
        create_control(
            parent,
            hinstance,
            "EDIT",
            text,
            x,
            y,
            w,
            h,
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_BORDER | style,
            WS_EX_CLIENTEDGE,
            id,
            font,
        )
    }
}

unsafe fn create_control(
    parent: HWND,
    hinstance: HINSTANCE,
    class: &str,
    text: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    style: DWORD,
    ex_style: DWORD,
    id: usize,
    font: HFONT,
) -> HWND {
    let class = to_wide(class);
    let text = to_wide(text);
    let hwnd = unsafe {
        CreateWindowExW(
            ex_style,
            class.as_ptr(),
            text.as_ptr(),
            style,
            x,
            y,
            w,
            h,
            parent,
            id as HMENU,
            hinstance,
            null_mut(),
        )
    };
    if hwnd != 0 && font != 0 {
        unsafe {
            SendMessageW(hwnd, WM_SETFONT, font as WPARAM, 1);
        }
    }
    hwnd
}

unsafe fn create_font(size: i32, weight: i32) -> HFONT {
    let face = to_wide("Microsoft YaHei UI");
    unsafe {
        CreateFontW(
            -size,
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            DEFAULT_PITCH,
            face.as_ptr(),
        )
    }
}

fn app_lock() -> std::sync::MutexGuard<'static, AppState> {
    APP.get().unwrap().lock().unwrap()
}

unsafe fn show_error_if_needed(hwnd: HWND, result: Result<(), String>) {
    if let Err(err) = result {
        unsafe {
            message_box(hwnd, "错误", &err, MB_OK | MB_ICONERROR);
        }
    }
}

fn null_mut<T>() -> *mut T {
    std::ptr::null_mut::<T>()
}
