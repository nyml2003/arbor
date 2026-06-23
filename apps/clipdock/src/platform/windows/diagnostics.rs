use windows::core::PCWSTR;
use windows::Win32::System::Diagnostics::Debug::OutputDebugStringW;

pub fn report_error(message: &str) {
    let message = format!("{message}\n");
    let wide = message
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();

    // SAFETY: wide is a valid null-terminated UTF-16 buffer for the duration of the call.
    unsafe {
        OutputDebugStringW(PCWSTR(wide.as_ptr()));
    }
}
