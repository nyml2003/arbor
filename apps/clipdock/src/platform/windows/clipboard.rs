use std::ffi::c_void;
use std::ptr;

use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, EmptyClipboard, GetClipboardData,
    IsClipboardFormatAvailable, OpenClipboard, RemoveClipboardFormatListener, SetClipboardData,
};
use windows::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE, GMEM_ZEROINIT,
};
use windows::Win32::System::Ole::CF_UNICODETEXT;

use super::error::{PlatformError, PlatformResult, WindowsResultExt};

pub fn add_listener(hwnd: HWND) -> PlatformResult<()> {
    // SAFETY: hwnd is a live top-level window owned by ClipDock.
    unsafe { AddClipboardFormatListener(hwnd).context("add clipboard format listener") }
}

pub fn remove_listener(hwnd: HWND) {
    // SAFETY: hwnd is the same window previously registered. Failure during teardown is not fatal.
    let _ = unsafe { RemoveClipboardFormatListener(hwnd) };
}

pub fn read_text(hwnd: HWND) -> PlatformResult<Option<String>> {
    let _clipboard = ClipboardGuard::open(hwnd)?;
    if unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT.0 as u32) }.is_err() {
        return Ok(None);
    }

    // SAFETY: Clipboard is open on this thread and the format availability was checked.
    let handle =
        unsafe { GetClipboardData(CF_UNICODETEXT.0 as u32).context("get unicode clipboard data")? };
    let global = HGLOBAL(handle.0);
    let locked = LockedGlobal::new(global)?;
    let byte_len = unsafe { GlobalSize(global) };
    if byte_len == 0 {
        return Ok(None);
    }

    let unit_len = byte_len / size_of::<u16>();
    let units = unsafe { std::slice::from_raw_parts(locked.ptr().cast::<u16>(), unit_len) };
    let end = units
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(unit_len);
    if end == 0 {
        return Ok(None);
    }

    String::from_utf16(&units[..end])
        .map(Some)
        .map_err(|_| PlatformError::InvalidClipboardText)
}

pub fn write_text(hwnd: HWND, text: &str) -> PlatformResult<()> {
    let _clipboard = ClipboardGuard::open(hwnd)?;
    // SAFETY: Clipboard is open and owned by this window for the duration of this call.
    unsafe { EmptyClipboard().context("empty clipboard")? };

    let mut units = text.encode_utf16().collect::<Vec<_>>();
    units.push(0);
    let byte_len = units.len() * size_of::<u16>();

    // SAFETY: Allocates movable, zero-initialized global memory as required by SetClipboardData.
    let mut global = OwnedGlobal::new(unsafe {
        GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, byte_len)
            .context("allocate clipboard global memory")?
    });
    {
        let locked = LockedGlobal::new(global.handle())?;
        // SAFETY: locked points to byte_len writable bytes allocated above, and units has byte_len
        // initialized bytes. Regions do not overlap.
        unsafe {
            ptr::copy_nonoverlapping(
                units.as_ptr().cast::<u8>(),
                locked.ptr().cast::<u8>(),
                byte_len,
            );
        }
    }

    let handle = HANDLE(global.handle().0);
    // SAFETY: On success, the clipboard owns the HGLOBAL and the local owner must not free it.
    unsafe {
        SetClipboardData(CF_UNICODETEXT.0 as u32, Some(handle))
            .context("set unicode clipboard data")?;
    }
    global.release();

    Ok(())
}

struct ClipboardGuard;

impl ClipboardGuard {
    fn open(hwnd: HWND) -> PlatformResult<Self> {
        // SAFETY: hwnd is a live window used as clipboard owner/open requester.
        unsafe { OpenClipboard(Some(hwnd)).context("open clipboard")? };
        Ok(Self)
    }
}

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        // SAFETY: Balances a successful OpenClipboard call on this thread.
        let _ = unsafe { CloseClipboard() };
    }
}

struct LockedGlobal {
    global: HGLOBAL,
    ptr: *mut c_void,
}

impl LockedGlobal {
    fn new(global: HGLOBAL) -> PlatformResult<Self> {
        // SAFETY: The HGLOBAL is owned by the clipboard or by this module and remains valid while
        // locked. Null indicates failure.
        let ptr = unsafe { GlobalLock(global) };
        if ptr.is_null() {
            return Err(windows::core::Error::from_thread()).context("lock global memory");
        }
        Ok(Self { global, ptr })
    }

    fn ptr(&self) -> *mut c_void {
        self.ptr
    }
}

impl Drop for LockedGlobal {
    fn drop(&mut self) {
        // SAFETY: Balances a successful GlobalLock call for this HGLOBAL.
        let _ = unsafe { GlobalUnlock(self.global) };
    }
}

struct OwnedGlobal {
    handle: Option<HGLOBAL>,
}

impl OwnedGlobal {
    fn new(handle: HGLOBAL) -> Self {
        Self {
            handle: Some(handle),
        }
    }

    fn handle(&self) -> HGLOBAL {
        self.handle.expect("owned global handle is present")
    }

    fn release(&mut self) {
        self.handle = None;
    }
}

impl Drop for OwnedGlobal {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            // SAFETY: The handle was allocated by GlobalAlloc and was not transferred to clipboard.
            let _ = unsafe { GlobalFree(Some(handle)) };
        }
    }
}
