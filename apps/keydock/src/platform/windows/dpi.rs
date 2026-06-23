use windows::Win32::UI::HiDpi::{
    SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};

pub fn set_thread_dpi_awareness() {
    // SAFETY: This process is single-threaded during startup and the context value is a Windows
    // provided constant. Failure only leaves the default DPI context, which is acceptable for v1.
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}
