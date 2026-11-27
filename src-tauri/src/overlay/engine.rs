//! Win32 overlay engine.
//!
//! A dedicated thread owns a native layered window:
//!
//! * `WS_POPUP` + `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST |
//!   WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE` → transparent, click-through, always
//!   on top, never activated, absent from the taskbar and Alt+Tab.
//! * The crosshair is rendered once into a 32-bit premultiplied DIB and pushed
//!   with `UpdateLayeredWindow`. Nothing is redrawn until something changes:
//!   the thread sleeps in `GetMessageW` (0 % CPU at rest).
//! * The window is created once and only moved/resized afterwards.
//! * `WM_DISPLAYCHANGE` / `WM_DPICHANGED` → monitors are re-read and the
//!   window re-centred (debounced 300 ms).
//! * `SetWinEventHook(EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT)` →
//!   notified when the foreground window changes, without loading anything
//!   into other processes. Used for "fullscreen only" visibility and to detect
//!   exclusive fullscreen (`SHQueryUserNotificationState`).
//!
//! Other threads talk to it through a channel; every command waits for the
//! overlay thread to reply with the *actual* window state.

use std::cell::{Cell, RefCell};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{
    GetLastError, COLORREF, ERROR_CLASS_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, AC_SRC_ALPHA, AC_SRC_OVER,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::HiDpi::{SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClassNameW, GetDesktopWindow,
    GetForegroundWindow, GetMessageW, GetShellWindow, GetWindowLongPtrW, GetWindowRect, IsWindowVisible,
    KillTimer, PostMessageW, PostQuitMessage, RegisterClassExW, SetTimer, SetWindowPos, ShowWindow,
    TranslateMessage, UpdateLayeredWindow, EVENT_SYSTEM_FOREGROUND, GWL_STYLE, HTTRANSPARENT, HWND_TOPMOST,
    MA_NOACTIVATE, MSG, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_SHOWWINDOW, SW_HIDE,
    ULW_ALPHA, WINEVENT_OUTOFCONTEXT, WM_APP, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_MOUSEACTIVATE, WM_NCHITTEST,
    WM_TIMER, WNDCLASSEXW, WS_CAPTION, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP,
};

use super::controller::{covers_monitor, should_show, OverlayEvent, OverlayOptions, OverlayStatus};
use crate::crosshair::render::{render, to_bgra_premultiplied};
use crate::crosshair::CrosshairSpec;
use crate::displays::{self, DisplayInfo};
use crate::error::{AppError, AppResult};
use crate::settings::VisibilityMode;

const WM_APP_WAKE: u32 = WM_APP + 1;
const WM_APP_REEVALUATE: u32 = WM_APP + 2;
const TIMER_TOPMOST: usize = 1;
const TIMER_FOREGROUND: usize = 2;
const TIMER_DISPLAY: usize = 3;
const FOREGROUND_POLL_MS: u32 = 1000;
const DISPLAY_DEBOUNCE_MS: u32 = 300;
const CLASS_NAME: PCWSTR = w!("CrosshairStudioOverlay");
const REPLY_TIMEOUT: Duration = Duration::from_secs(3);

pub type EventSink = Box<dyn Fn(OverlayEvent) + Send + 'static>;

enum Command {
    SetSpec(Box<CrosshairSpec>),
    SetVisible(bool),
    SetOptions(OverlayOptions),
    Refresh,
    Shutdown,
}

struct Request {
    cmd: Command,
    reply: SyncSender<OverlayStatus>,
}

/// Handle used by the rest of the application. Cheap to share (`Send + Sync`).
pub struct OverlayEngine {
    tx: Sender<Request>,
    hwnd: isize,
    status: Arc<Mutex<OverlayStatus>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl OverlayEngine {
    /// Creates the overlay thread and its window, then applies the initial state.
    pub fn start(spec: CrosshairSpec, options: OverlayOptions, visible: bool, sink: EventSink) -> AppResult<Self> {
        let (tx, rx) = mpsc::channel::<Request>();
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<isize, String>>(1);
        let status = Arc::new(Mutex::new(OverlayStatus::default()));
        let shared = status.clone();
        let thread = std::thread::Builder::new()
            .name("crosshair-overlay".into())
            .spawn(move || thread_main(rx, spec, options, visible, sink, shared, ready_tx))?;
        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(hwnd)) => Ok(Self { tx, hwnd, status, thread: Mutex::new(Some(thread)) }),
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(AppError::Overlay(e))
            }
            Err(_) => Err(AppError::Overlay("The overlay engine did not start in time.".into())),
        }
    }

    fn send(&self, cmd: Command) -> AppResult<OverlayStatus> {
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        self.tx
            .send(Request { cmd, reply: reply_tx })
            .map_err(|_| AppError::Overlay("The overlay engine has stopped.".into()))?;
        unsafe { PostMessageW(Some(HWND(self.hwnd as *mut _)), WM_APP_WAKE, WPARAM(0), LPARAM(0)) }
            .map_err(|e| AppError::Overlay(format!("The overlay window cannot be reached: {e}")))?;
        reply_rx
            .recv_timeout(REPLY_TIMEOUT)
            .map_err(|_| AppError::Overlay("The overlay engine did not respond.".into()))
    }

    pub fn set_spec(&self, spec: CrosshairSpec) -> AppResult<OverlayStatus> {
        self.send(Command::SetSpec(Box::new(spec)))
    }

    pub fn set_visible(&self, visible: bool) -> AppResult<OverlayStatus> {
        self.send(Command::SetVisible(visible))
    }

    pub fn set_options(&self, options: OverlayOptions) -> AppResult<OverlayStatus> {
        self.send(Command::SetOptions(options))
    }

    /// Re-reads the monitors and redraws.
    pub fn refresh(&self) -> AppResult<OverlayStatus> {
        self.send(Command::Refresh)
    }

    /// Last status confirmed by the overlay thread.
    pub fn status(&self) -> OverlayStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Destroys the window and stops the thread. Idempotent.
    pub fn shutdown(&self) {
        let handle = self.thread.lock().ok().and_then(|mut t| t.take());
        if let Some(handle) = handle {
            let _ = self.send(Command::Shutdown);
            let _ = handle.join();
        }
    }
}

impl Drop for OverlayEngine {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// ─── overlay thread ─────────────────────────────────────────────────────────

thread_local! {
    static ENGINE: RefCell<Option<Engine>> = const { RefCell::new(None) };
    static OVERLAY_HWND: Cell<isize> = const { Cell::new(0) };
}

/// Off-screen 32-bit DIB selected into a memory DC.
struct Surface {
    dc: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u8,
    size: i32,
}

impl Surface {
    fn new(size: i32) -> Result<Self, String> {
        unsafe {
            let dc = CreateCompatibleDC(None);
            if dc.is_invalid() {
                return Err("CreateCompatibleDC failed".into());
            }
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: size,
                    biHeight: -size, // top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
            let bmp = match CreateDIBSection(Some(dc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0) {
                Ok(b) if !bits.is_null() => b,
                Ok(_) | Err(_) => {
                    let _ = DeleteDC(dc);
                    return Err("CreateDIBSection failed".into());
                }
            };
            let old = SelectObject(dc, bmp.into());
            Ok(Self { dc, bmp, old, bits: bits as *mut u8, size })
        }
    }

    fn write(&mut self, bgra: &[u8]) {
        let len = (self.size * self.size * 4) as usize;
        debug_assert_eq!(bgra.len(), len);
        unsafe { std::ptr::copy_nonoverlapping(bgra.as_ptr(), self.bits, len.min(bgra.len())) };
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            let _ = DeleteObject(self.bmp.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

struct Engine {
    hwnd: HWND,
    rx: Receiver<Request>,
    spec: CrosshairSpec,
    options: OverlayOptions,
    requested: bool,
    fg_fullscreen: bool,
    exclusive: bool,
    warned_for: isize,
    displays: Vec<DisplayInfo>,
    display: Option<DisplayInfo>,
    fallback: bool,
    surface: Option<Surface>,
    scale: f32,
    size: i32,
    shown: bool,
    topmost_interval: Option<u32>,
    fg_timer: bool,
    render_count: u64,
    last_render_us: u64,
    last_error: Option<String>,
    destroyed: bool,
    status: Arc<Mutex<OverlayStatus>>,
    last_published: Option<OverlayStatus>,
    sink: EventSink,
}

#[allow(clippy::too_many_arguments)]
fn thread_main(
    rx: Receiver<Request>,
    spec: CrosshairSpec,
    options: OverlayOptions,
    visible: bool,
    sink: EventSink,
    status: Arc<Mutex<OverlayStatus>>,
    ready: SyncSender<Result<isize, String>>,
) {
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let hinstance = match GetModuleHandleW(None) {
            Ok(h) => h,
            Err(e) => {
                let _ = ready.send(Err(format!("GetModuleHandleW failed: {e}")));
                return;
            }
        };
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance.into(),
            lpszClassName: CLASS_NAME,
            ..Default::default()
        };
        if RegisterClassExW(&wc) == 0 && GetLastError() != ERROR_CLASS_ALREADY_EXISTS {
            let _ = ready.send(Err(format!("RegisterClassExW failed: {:?}", GetLastError())));
            return;
        }
        let hwnd = match CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            CLASS_NAME,
            w!("Crosshair Studio Overlay"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(hinstance.into()),
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                let _ = ready.send(Err(format!("The overlay window could not be created: {e}")));
                return;
            }
        };
        OVERLAY_HWND.with(|c| c.set(hwnd.0 as isize));

        let hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(win_event_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );

        let mut engine = Engine {
            hwnd,
            rx,
            spec,
            options,
            requested: visible,
            fg_fullscreen: false,
            exclusive: false,
            warned_for: 0,
            displays: Vec::new(),
            display: None,
            fallback: false,
            surface: None,
            scale: 1.0,
            size: 0,
            shown: false,
            topmost_interval: None,
            fg_timer: false,
            render_count: 0,
            last_render_us: 0,
            last_error: None,
            destroyed: false,
            status,
            last_published: None,
            sink,
        };
        engine.reload_displays();
        engine.evaluate_foreground();
        engine.apply(true);
        ENGINE.with(|e| *e.borrow_mut() = Some(engine));
        let _ = ready.send(Ok(hwnd.0 as isize));

        let mut msg = MSG::default();
        loop {
            let r = GetMessageW(&mut msg, None, 0, 0).0;
            if r == 0 || r == -1 {
                break;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        ENGINE.with(|e| {
            if let Some(mut engine) = e.borrow_mut().take() {
                engine.teardown();
            }
        });
        if !hook.is_invalid() {
            let _ = UnhookWinEvent(hook);
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        // Belt and braces for click-through (WS_EX_TRANSPARENT already does it).
        WM_NCHITTEST => return LRESULT(HTTRANSPARENT as isize),
        WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
        WM_APP_WAKE | WM_APP_REEVALUATE | WM_TIMER | WM_DISPLAYCHANGE | WM_DPICHANGED => {
            let handled = ENGINE.with(|cell| match cell.try_borrow_mut() {
                Ok(mut guard) => match guard.as_mut() {
                    Some(engine) => {
                        engine.on_message(msg, wparam);
                        true
                    }
                    None => false,
                },
                Err(_) => {
                    // Re-entrant call (sent synchronously while the engine is
                    // busy): handle it later from the message loop.
                    if msg != WM_TIMER {
                        let _ = PostMessageW(Some(hwnd), WM_APP_REEVALUATE, WPARAM(1), LPARAM(0));
                    }
                    true
                }
            });
            if handled && msg != WM_DPICHANGED && msg != WM_DISPLAYCHANGE {
                return LRESULT(0);
            }
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

unsafe extern "system" fn win_event_proc(
    _hook: HWINEVENTHOOK,
    _event: u32,
    _hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time: u32,
) {
    ENGINE.with(|cell| match cell.try_borrow_mut() {
        Ok(mut guard) => {
            if let Some(engine) = guard.as_mut() {
                if engine.evaluate_foreground() {
                    engine.apply(false);
                }
            }
        }
        Err(_) => {
            let h = OVERLAY_HWND.with(|c| c.get());
            if h != 0 {
                let _ = PostMessageW(Some(HWND(h as *mut _)), WM_APP_REEVALUATE, WPARAM(0), LPARAM(0));
            }
        }
    });
}

/// (foreground covers the monitor, exclusive fullscreen running, foreground hwnd)
unsafe fn foreground_state(display: Option<&DisplayInfo>) -> (bool, bool, isize) {
    let exclusive = matches!(SHQueryUserNotificationState(), Ok(s) if s == QUNS_RUNNING_D3D_FULL_SCREEN);
    let fg = GetForegroundWindow();
    if fg.is_invalid() {
        return (false, exclusive, 0);
    }
    let id = fg.0 as isize;
    if fg == GetShellWindow() || fg == GetDesktopWindow() {
        return (false, exclusive, id);
    }
    let mut class = [0u16; 64];
    let n = GetClassNameW(fg, &mut class).max(0) as usize;
    let class = String::from_utf16_lossy(&class[..n]);
    if matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") {
        return (false, exclusive, id);
    }
    // A window with a title bar is a normal (possibly maximized) window.
    let style = GetWindowLongPtrW(fg, GWL_STYLE) as u32;
    if style & WS_CAPTION.0 == WS_CAPTION.0 {
        return (false, exclusive, id);
    }
    let mut rc = RECT::default();
    if GetWindowRect(fg, &mut rc).is_err() {
        return (false, exclusive, id);
    }
    let Some(d) = display else { return (false, exclusive, id) };
    let fs = covers_monitor((rc.left, rc.top, rc.right, rc.bottom), (d.x, d.y, d.x + d.width, d.y + d.height));
    (fs, exclusive, id)
}

impl Engine {
    fn on_message(&mut self, msg: u32, wparam: WPARAM) {
        match msg {
            WM_APP_WAKE => self.drain(),
            WM_APP_REEVALUATE => {
                if wparam.0 == 1 {
                    self.reload_displays();
                }
                self.evaluate_foreground();
                self.apply(wparam.0 == 1);
            }
            WM_TIMER => match wparam.0 {
                TIMER_TOPMOST => self.reassert_topmost(),
                TIMER_FOREGROUND => {
                    if self.evaluate_foreground() {
                        self.apply(false);
                    }
                }
                TIMER_DISPLAY => {
                    unsafe {
                        let _ = KillTimer(Some(self.hwnd), TIMER_DISPLAY);
                    }
                    self.reload_displays();
                    self.apply(true);
                    (self.sink)(OverlayEvent::DisplaysChanged);
                }
                _ => {}
            },
            WM_DISPLAYCHANGE | WM_DPICHANGED => unsafe {
                // Monitors may not be fully updated yet: debounce.
                SetTimer(Some(self.hwnd), TIMER_DISPLAY, DISPLAY_DEBOUNCE_MS, None);
            },
            _ => {}
        }
    }

    /// Processes every pending request, coalescing the work: during a slider
    /// drag only the latest spec is rendered.
    fn drain(&mut self) {
        let mut replies = Vec::new();
        let mut rerender = false;
        let mut reload = false;
        let mut shutdown = false;
        while let Ok(req) = self.rx.try_recv() {
            match req.cmd {
                Command::SetSpec(spec) => {
                    self.spec = *spec;
                    rerender = true;
                }
                Command::SetVisible(v) => self.requested = v,
                Command::SetOptions(o) => {
                    rerender |= o.dpi_scaling != self.options.dpi_scaling || o.monitor != self.options.monitor;
                    reload = true;
                    self.options = o;
                }
                Command::Refresh => {
                    reload = true;
                    rerender = true;
                }
                Command::Shutdown => shutdown = true,
            }
            replies.push(req.reply);
        }
        if shutdown {
            self.teardown();
            unsafe { PostQuitMessage(0) };
        } else if !replies.is_empty() {
            if reload {
                self.reload_displays();
            }
            self.evaluate_foreground();
            self.apply(rerender);
        }
        let status = self.snapshot();
        for r in replies {
            let _ = r.send(status.clone());
        }
    }

    fn reload_displays(&mut self) {
        self.displays = displays::list();
    }

    /// Re-reads the foreground window. Returns true when the result changed.
    fn evaluate_foreground(&mut self) -> bool {
        let current = displays::resolve(&self.displays, self.options.monitor.as_deref()).map(|(d, _)| d.clone());
        let (fullscreen, exclusive, fg) = unsafe { foreground_state(current.as_ref()) };
        let changed = fullscreen != self.fg_fullscreen || exclusive != self.exclusive;
        self.fg_fullscreen = fullscreen;
        self.exclusive = exclusive;
        if exclusive && self.requested && fg != self.warned_for {
            self.warned_for = fg;
            (self.sink)(OverlayEvent::Warning {
                code: "exclusive_fullscreen",
                message: "A program is running in exclusive fullscreen. Windows does not allow any overlay above it: switch the game to borderless or windowed mode.".into(),
            });
        }
        if !exclusive {
            self.warned_for = 0;
        }
        changed
    }

    fn apply(&mut self, rerender: bool) {
        if self.destroyed {
            return;
        }
        let Some((display, fallback)) =
            displays::resolve(&self.displays, self.options.monitor.as_deref()).map(|(d, f)| (d.clone(), f))
        else {
            self.last_error = Some("No monitor detected.".into());
            self.hide();
            self.publish();
            return;
        };
        self.fallback = fallback;

        let scale = if self.options.dpi_scaling { display.scale } else { 1.0 };
        if rerender || self.surface.is_none() || (scale - self.scale).abs() > 1e-3 {
            let started = Instant::now();
            let pixmap = render(&self.spec, scale);
            let size = pixmap.width() as i32;
            if self.surface.as_ref().map(|s| s.size) != Some(size) {
                self.surface = None; // free the old DIB before creating the new one
                match Surface::new(size) {
                    Ok(s) => self.surface = Some(s),
                    Err(e) => self.last_error = Some(e),
                }
            }
            if let Some(surface) = self.surface.as_mut() {
                surface.write(&to_bgra_premultiplied(&pixmap));
                self.size = size;
            }
            self.scale = scale;
            self.render_count += 1;
            self.last_render_us = started.elapsed().as_micros() as u64;
        }

        let (x, y) = displays::centered_origin(&display, self.size, self.options.offset_x, self.options.offset_y);
        self.display = Some(display);

        if let Some(surface) = self.surface.as_ref() {
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let result = unsafe {
                UpdateLayeredWindow(
                    self.hwnd,
                    None,
                    Some(&POINT { x, y }),
                    Some(&SIZE { cx: self.size, cy: self.size }),
                    Some(surface.dc),
                    Some(&POINT { x: 0, y: 0 }),
                    COLORREF(0),
                    Some(&blend),
                    ULW_ALPHA,
                )
            };
            self.last_error = result.err().map(|e| format!("UpdateLayeredWindow failed: {e}"));
        }

        let show = should_show(self.requested, self.options.visibility, self.fg_fullscreen) && self.surface.is_some();
        if show {
            unsafe {
                let _ = SetWindowPos(
                    self.hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
            }
            self.shown = true;
        } else {
            self.hide();
        }
        self.update_timers();
        self.publish();
    }

    fn hide(&mut self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
        self.shown = false;
    }

    fn reassert_topmost(&self) {
        if self.shown {
            unsafe {
                let _ = SetWindowPos(
                    self.hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
                );
            }
        }
    }

    fn update_timers(&mut self) {
        let wanted_top = (self.shown && self.options.keep_on_top).then_some(self.options.topmost_interval_ms);
        if wanted_top != self.topmost_interval {
            unsafe {
                match wanted_top {
                    Some(ms) => {
                        SetTimer(Some(self.hwnd), TIMER_TOPMOST, ms, None);
                    }
                    None => {
                        let _ = KillTimer(Some(self.hwnd), TIMER_TOPMOST);
                    }
                }
            }
            self.topmost_interval = wanted_top;
        }
        let wanted_fg = self.requested && self.options.visibility == VisibilityMode::FullscreenOnly;
        if wanted_fg != self.fg_timer {
            unsafe {
                if wanted_fg {
                    SetTimer(Some(self.hwnd), TIMER_FOREGROUND, FOREGROUND_POLL_MS, None);
                } else {
                    let _ = KillTimer(Some(self.hwnd), TIMER_FOREGROUND);
                }
            }
            self.fg_timer = wanted_fg;
        }
    }

    fn snapshot(&self) -> OverlayStatus {
        let (visible, rect) = if self.destroyed {
            (false, RECT::default())
        } else {
            unsafe {
                let mut rc = RECT::default();
                let _ = GetWindowRect(self.hwnd, &mut rc);
                (IsWindowVisible(self.hwnd).as_bool(), rc)
            }
        };
        OverlayStatus {
            running: !self.destroyed,
            visible,
            requested: self.requested,
            auto_hidden: self.requested && !should_show(self.requested, self.options.visibility, self.fg_fullscreen),
            monitor_id: self.display.as_ref().map(|d| d.id.clone()),
            monitor_name: self.display.as_ref().map(|d| d.name.clone()),
            monitor_fallback: self.fallback,
            x: rect.left,
            y: rect.top,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
            scale: self.scale,
            render_count: self.render_count,
            last_render_micros: self.last_render_us,
            exclusive_fullscreen: self.exclusive,
            last_error: self.last_error.clone(),
        }
    }

    fn publish(&mut self) {
        let s = self.snapshot();
        if let Ok(mut shared) = self.status.lock() {
            *shared = s.clone();
        }
        // Notify only meaningful changes (not every render counter tick).
        let key = |st: &OverlayStatus| OverlayStatus { render_count: 0, last_render_micros: 0, ..st.clone() };
        if self.last_published.as_ref().map(key) != Some(key(&s)) {
            (self.sink)(OverlayEvent::StatusChanged(s.clone()));
        }
        self.last_published = Some(s);
    }

    fn teardown(&mut self) {
        if self.destroyed {
            return;
        }
        unsafe {
            let _ = KillTimer(Some(self.hwnd), TIMER_TOPMOST);
            let _ = KillTimer(Some(self.hwnd), TIMER_FOREGROUND);
            let _ = KillTimer(Some(self.hwnd), TIMER_DISPLAY);
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
        self.surface = None;
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
        self.destroyed = true;
        self.shown = false;
        if let Ok(mut shared) = self.status.lock() {
            shared.running = false;
            shared.visible = false;
        }
    }
}

#[cfg(test)]
mod tests {
    //! These tests create the real Win32 overlay window on this machine.
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use windows::Win32::UI::WindowsAndMessaging::{GWL_EXSTYLE, WS_EX_LAYERED as EX_LAYERED};

    fn start(visible: bool) -> (OverlayEngine, Arc<AtomicUsize>) {
        let events = Arc::new(AtomicUsize::new(0));
        let counter = events.clone();
        let engine = OverlayEngine::start(
            CrosshairSpec::default(),
            OverlayOptions::default(),
            visible,
            Box::new(move |_| {
                counter.fetch_add(1, Ordering::SeqCst);
            }),
        )
        .expect("overlay engine starts");
        (engine, events)
    }

    #[test]
    fn window_has_overlay_styles() {
        let (engine, _) = start(false);
        let hwnd = HWND(engine.hwnd as *mut _);
        let ex = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
        for flag in [EX_LAYERED, WS_EX_TRANSPARENT, WS_EX_TOPMOST, WS_EX_TOOLWINDOW, WS_EX_NOACTIVATE] {
            assert_eq!(ex & flag.0, flag.0, "missing extended style {:#x}", flag.0);
        }
        engine.shutdown();
    }

    #[test]
    fn activation_and_deactivation_are_confirmed() {
        let (engine, events) = start(false);
        let s = engine.status();
        assert!(s.running);
        assert!(!s.visible && !s.requested);

        let s = engine.set_visible(true).unwrap();
        assert!(s.requested && s.visible, "overlay must be visible: {s:?}");
        assert!(unsafe { IsWindowVisible(HWND(engine.hwnd as *mut _)).as_bool() });

        let s = engine.set_visible(false).unwrap();
        assert!(!s.visible && !s.requested);
        assert!(events.load(Ordering::SeqCst) >= 2, "status changes are notified");

        engine.shutdown();
        assert!(!engine.status().running);
        assert!(engine.set_visible(true).is_err(), "commands fail once stopped");
    }

    #[test]
    fn window_is_centred_and_resized_without_recreation() {
        let (engine, _) = start(true);
        let before = engine.status();
        let ds = displays::list();
        let (d, _) = displays::resolve(&ds, None).unwrap();
        assert_eq!(before.x + before.width / 2, d.x + d.width / 2);
        assert_eq!(before.y + before.height / 2, d.y + d.height / 2);

        let mut big = CrosshairSpec::default();
        big.crosshair.length_h = 60.0;
        big.crosshair.length_v = 60.0;
        let after = engine.set_spec(big).unwrap();
        assert!(after.width > before.width);
        assert_eq!(after.x + after.width / 2, d.x + d.width / 2, "still centred");
        assert_eq!(after.render_count, before.render_count + 1, "rendered exactly once");

        let moved = engine
            .set_options(OverlayOptions { offset_x: 10, offset_y: -4, ..OverlayOptions::default() })
            .unwrap();
        assert_eq!(moved.x, after.x + 10);
        assert_eq!(moved.y, after.y - 4);
        assert_eq!(moved.render_count, after.render_count, "moving does not re-render");
        engine.shutdown();
    }

    #[test]
    fn missing_monitor_falls_back_to_primary() {
        let (engine, _) = start(false);
        let s = engine
            .set_options(OverlayOptions { monitor: Some(r"\\.\DISPLAY_DOES_NOT_EXIST".into()), ..OverlayOptions::default() })
            .unwrap();
        assert!(s.monitor_fallback);
        assert!(s.monitor_id.is_some());
        engine.shutdown();
    }

    #[test]
    fn fullscreen_only_mode_hides_on_the_desktop() {
        let (engine, _) = start(true);
        let s = engine
            .set_options(OverlayOptions { visibility: VisibilityMode::FullscreenOnly, ..OverlayOptions::default() })
            .unwrap();
        // The test runner's console is not a fullscreen window.
        if !s.visible {
            assert!(s.auto_hidden);
        }
        engine.shutdown();
    }
}
