use std::cell::RefCell;
use std::mem::size_of;

mod com;

use windows::core::w;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetMessageW,
    GetWindowLongPtrW, GetWindowRect, IsWindow, KillTimer, LoadCursorW, PostQuitMessage, SetTimer,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateMessage, CREATESTRUCTW, GWLP_USERDATA,
    HCURSOR, HICON, HMENU, HTCAPTION, HTCLIENT, HWND_TOPMOST, IDC_ARROW, MSG, SWP_NOACTIVATE,
    SWP_SHOWWINDOW, SW_SHOWNA, WM_CANCELMODE, WM_CAPTURECHANGED, WM_CLOSE, WM_CREATE, WM_DESTROY,
    WM_DPICHANGED, WM_ERASEBKGND, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_NCCREATE,
    WM_NCHITTEST, WM_PAINT, WM_SIZE, WM_TIMER, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP,
};

use crate::app::{ChromeHit, InputCommand, KeyDockApp, Point, PointerEvent, Size};
use arbor_ui_windows::Renderer;

pub use com::ComApartment;

use super::error::{PlatformError, PlatformResult, WindowsResultExt};
use super::input;

const CLASS_NAME: windows::core::PCWSTR = w!("KeyDockWindowClass");
const WINDOW_TITLE: windows::core::PCWSTR = w!("KeyDock");
const INITIAL_WIDTH: i32 = 920;
const INITIAL_HEIGHT: i32 = 330;
const ANIMATION_TIMER_ID: usize = 1;
const ANIMATION_TIMER_MS: u32 = 16;

type RawWndProc = Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT>;

#[repr(C)]
struct RawWndClassExW {
    cb_size: u32,
    style: u32,
    wnd_proc: RawWndProc,
    class_extra: i32,
    window_extra: i32,
    instance: HINSTANCE,
    icon: HICON,
    cursor: HCURSOR,
    background_brush: *mut core::ffi::c_void,
    menu_name: windows::core::PCWSTR,
    class_name: windows::core::PCWSTR,
    small_icon: HICON,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassExW(window_class: *const RawWndClassExW) -> u16;
}

pub struct KeyDockWindow {
    hwnd: HWND,
    state: Box<WindowState>,
}

impl KeyDockWindow {
    pub fn new() -> PlatformResult<Self> {
        let instance = module_instance()?;
        register_window_class(instance)?;

        let mut state = Box::new(WindowState::new()?);
        let state_ptr = state.as_mut() as *mut WindowState;

        // SAFETY: The class was registered in this module, strings are static UTF-16, and state_ptr
        // points to a boxed WindowState that outlives the HWND. WM_NCCREATE stores the pointer.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                CLASS_NAME,
                WINDOW_TITLE,
                WS_POPUP,
                80,
                80,
                INITIAL_WIDTH,
                INITIAL_HEIGHT,
                None,
                None::<HMENU>,
                Some(instance),
                Some(state_ptr.cast()),
            )
            .context("create keydock window")?
        };

        state.attach(hwnd)?;

        // SAFETY: hwnd is live, HWND_TOPMOST is a Windows constant, and SWP_NOACTIVATE preserves the
        // intended no-focus window behavior.
        unsafe {
            SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                80,
                80,
                INITIAL_WIDTH,
                INITIAL_HEIGHT,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            )
            .context("show topmost keydock window")?;
            let _ = ShowWindow(hwnd, SW_SHOWNA);
        }

        Ok(Self { hwnd, state })
    }

    pub fn run(&mut self) -> PlatformResult<()> {
        // Touch state so the field is considered live ownership and not dead storage.
        let _ = self.state.app_snapshot_size();

        let mut message = MSG::default();
        loop {
            // SAFETY: message points to initialized writable storage and message loop is on UI thread.
            let result = unsafe { GetMessageW(&mut message, None, 0, 0) };
            let value = result.0;
            if value == -1 {
                return Err(PlatformError::MissingWindowState);
            }
            if value == 0 {
                break;
            }
            // SAFETY: MSG was populated by GetMessageW.
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        Ok(())
    }
}

impl Drop for KeyDockWindow {
    fn drop(&mut self) {
        if !self.hwnd.is_invalid() {
            // SAFETY: hwnd belongs to this wrapper while the native window still exists.
            if unsafe { IsWindow(Some(self.hwnd)) }.as_bool() {
                // SAFETY: hwnd is still a live window owned by this wrapper.
                let _ = unsafe { DestroyWindow(self.hwnd) };
            }
        }
    }
}

struct WindowState {
    app: KeyDockApp,
    renderer: RefCell<Option<Renderer>>,
    hwnd: Option<HWND>,
    animation_timer_running: bool,
}

impl WindowState {
    fn new() -> PlatformResult<Self> {
        Ok(Self {
            app: KeyDockApp::new()?,
            renderer: RefCell::new(None),
            hwnd: None,
            animation_timer_running: false,
        })
    }

    fn attach(&mut self, hwnd: HWND) -> PlatformResult<()> {
        self.hwnd = Some(hwnd);
        self.renderer.replace(Some(Renderer::new(hwnd)?));
        let size = client_size(hwnd)?;
        self.app.resize(size)?;
        Ok(())
    }

    fn app_snapshot_size(&self) -> Size {
        size_from_rect(self.app.snapshot().surface_rect)
    }

    fn handle_message(
        &mut self,
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_CREATE => LRESULT(0),
            WM_SIZE => {
                let _ = self.resize_from_client(hwnd);
                LRESULT(0)
            }
            WM_DPICHANGED => {
                let _ = self.resize_from_client(hwnd);
                LRESULT(0)
            }
            WM_MOUSEMOVE => {
                let point = point_from_lparam(lparam);
                if let Ok(update) = self.app.handle_pointer_event(PointerEvent::Move(point)) {
                    if update.needs_render {
                        let _ = self.render(hwnd);
                    }
                }
                LRESULT(0)
            }
            WM_LBUTTONDOWN => {
                let point = point_from_lparam(lparam);
                let needs_render = self
                    .app
                    .handle_pointer_event(PointerEvent::Down(point))
                    .map(|update| update.needs_render)
                    .unwrap_or(false);
                self.sync_animation_timer(hwnd);
                if needs_render {
                    let _ = self.render(hwnd);
                }
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                let point = point_from_lparam(lparam);
                let mut needs_render = false;
                if let Ok(update) = self.app.handle_pointer_event(PointerEvent::Up(point)) {
                    needs_render = update.needs_render;
                    self.execute_commands(hwnd, &update.commands);
                    if update.commands.contains(&InputCommand::CloseApp) {
                        return LRESULT(0);
                    }
                }
                self.sync_animation_timer(hwnd);
                if needs_render {
                    let _ = self.render(hwnd);
                }
                LRESULT(0)
            }
            WM_NCHITTEST => self.hit_test(hwnd, lparam),
            WM_CANCELMODE | WM_CAPTURECHANGED => {
                if let Ok(update) = self.app.handle_pointer_event(PointerEvent::Cancel) {
                    if update.needs_render {
                        let _ = self.render(hwnd);
                    }
                }
                LRESULT(0)
            }
            WM_PAINT => {
                // No GDI paint APIs are used. DefWindowProc validates the update region, then
                // Direct2D redraws the client area.
                let result = unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
                let _ = self.render(hwnd);
                result
            }
            WM_TIMER => {
                if wparam.0 == ANIMATION_TIMER_ID {
                    self.app.advance_animations(ANIMATION_TIMER_MS as f32);
                    self.sync_animation_timer(hwnd);
                    let _ = self.render(hwnd);
                    return LRESULT(0);
                }
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            WM_CLOSE => {
                // SAFETY: hwnd is this window and can be destroyed in response to WM_CLOSE.
                let _ = unsafe { DestroyWindow(hwnd) };
                LRESULT(0)
            }
            WM_DESTROY => {
                // SAFETY: Clears the user data slot for this HWND and posts quit for this UI thread.
                unsafe {
                    let _ = KillTimer(Some(hwnd), ANIMATION_TIMER_ID);
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    PostQuitMessage(0);
                }
                LRESULT(0)
            }
            _ => {
                // SAFETY: Delegating unhandled messages to DefWindowProcW is required by Win32.
                unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
            }
        }
    }

    fn resize_from_client(&mut self, hwnd: HWND) -> PlatformResult<()> {
        let size = client_size(hwnd)?;
        self.app.resize(size)?;
        if let Some(renderer) = self.renderer.borrow_mut().as_mut() {
            renderer.resize(size.width as u32, size.height as u32)?;
        }
        self.render(hwnd)?;
        Ok(())
    }

    fn render(&self, hwnd: HWND) -> PlatformResult<()> {
        let size = client_size(hwnd)?;
        let snapshot = self.app.snapshot();
        if let Some(renderer) = self.renderer.borrow_mut().as_mut() {
            renderer.draw(&snapshot, size.width as u32, size.height as u32)?;
        }
        Ok(())
    }

    fn sync_animation_timer(&mut self, hwnd: HWND) {
        if self.app.has_active_animations() {
            if !self.animation_timer_running {
                // SAFETY: hwnd is this window. A null timer proc routes WM_TIMER to this wndproc.
                let timer_id =
                    unsafe { SetTimer(Some(hwnd), ANIMATION_TIMER_ID, ANIMATION_TIMER_MS, None) };
                self.animation_timer_running = timer_id != 0;
            }
        } else if self.animation_timer_running {
            // SAFETY: Cancels the timer owned by this hwnd/id pair.
            let _ = unsafe { KillTimer(Some(hwnd), ANIMATION_TIMER_ID) };
            self.animation_timer_running = false;
        }
    }

    fn hit_test(&self, hwnd: HWND, lparam: LPARAM) -> LRESULT {
        // SAFETY: WM_NCHITTEST defaults must run first so Windows can keep any system behavior it
        // owns. KeyDock only remaps normal client hits into drag hits for its title strip.
        let default_hit = unsafe { DefWindowProcW(hwnd, WM_NCHITTEST, WPARAM(0), lparam) };
        if default_hit.0 != HTCLIENT as isize {
            return default_hit;
        }

        let Some(point) = screen_point_to_client(hwnd, point_from_lparam(lparam)) else {
            return default_hit;
        };

        match self.app.chrome_hit_test(point) {
            ChromeHit::Drag => LRESULT(HTCAPTION as isize),
            ChromeHit::Client | ChromeHit::Command(_) => default_hit,
        }
    }

    fn execute_commands(&self, hwnd: HWND, commands: &[InputCommand]) {
        for command in commands {
            match command {
                InputCommand::CloseApp => {
                    // SAFETY: CloseApp is generated by KeyDock's own UI and destroys its own HWND.
                    let _ = unsafe { DestroyWindow(hwnd) };
                }
                command => {
                    let _ = input::send(command);
                }
            }
        }
    }
}

fn register_window_class(instance: HINSTANCE) -> PlatformResult<()> {
    let cursor = load_arrow_cursor()?;
    let class = RawWndClassExW {
        cb_size: size_of::<RawWndClassExW>() as u32,
        style: 0,
        wnd_proc: Some(window_proc),
        class_extra: 0,
        window_extra: 0,
        instance,
        icon: HICON::default(),
        cursor,
        background_brush: std::ptr::null_mut(),
        menu_name: windows::core::PCWSTR::null(),
        class_name: CLASS_NAME,
        small_icon: HICON::default(),
    };
    // SAFETY: class matches the Win32 WNDCLASSEXW ABI. The background brush is null because KeyDock
    // paints with Direct2D and suppresses WM_ERASEBKGND.
    let atom = unsafe { RegisterClassExW(&class) };
    if atom == 0 {
        return Err(windows::core::Error::from_thread()).context("register keydock window class");
    }
    Ok(())
}

fn load_arrow_cursor() -> PlatformResult<HCURSOR> {
    // SAFETY: IDC_ARROW is a system cursor resource and no module handle is required.
    unsafe { LoadCursorW(None, IDC_ARROW).context("load arrow cursor") }
}

fn module_instance() -> PlatformResult<HINSTANCE> {
    // SAFETY: None requests the handle for the current process module.
    let module = unsafe { GetModuleHandleW(None).context("get module handle")? };
    Ok(HINSTANCE(module.0))
}

fn size_from_rect(rect: crate::app::Rect) -> Size {
    Size::new(rect.width, rect.height)
}

fn client_size(hwnd: HWND) -> PlatformResult<Size> {
    let mut rect = RECT::default();
    // SAFETY: rect points to writable stack storage and hwnd is a live window.
    unsafe { GetClientRect(hwnd, &mut rect).context("get client rect")? };
    Ok(Size::new(
        (rect.right - rect.left).max(1) as f32,
        (rect.bottom - rect.top).max(1) as f32,
    ))
}

fn point_from_lparam(lparam: LPARAM) -> Point {
    let value = lparam.0 as u32;
    let x = (value & 0xffff) as i16 as f32;
    let y = ((value >> 16) & 0xffff) as i16 as f32;
    Point::new(x, y)
}

fn screen_point_to_client(hwnd: HWND, point: Point) -> Option<Point> {
    let mut rect = RECT::default();
    // SAFETY: rect points to writable stack storage and hwnd is the window being hit-tested.
    if unsafe { GetWindowRect(hwnd, &mut rect) }.is_ok() {
        return Some(Point::new(
            point.x - rect.left as f32,
            point.y - rect.top as f32,
        ));
    }

    None
}

extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        // SAFETY: lparam for WM_NCCREATE is a valid CREATESTRUCTW pointer. lpCreateParams is the
        // WindowState pointer passed to CreateWindowExW and remains boxed by KeyDockWindow.
        unsafe {
            let create = lparam.0 as *const CREATESTRUCTW;
            if !create.is_null() {
                let state = (*create).lpCreateParams as *mut WindowState;
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize);
            }
        }
    }

    // SAFETY: GWLP_USERDATA contains either 0 or the WindowState pointer installed at WM_NCCREATE.
    let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState };
    if state_ptr.is_null() {
        // SAFETY: No state is available yet, so the default window procedure handles the message.
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }

    // SAFETY: state_ptr is owned by KeyDockWindow and lives at least as long as the HWND.
    unsafe { &mut *state_ptr }.handle_message(hwnd, message, wparam, lparam)
}
