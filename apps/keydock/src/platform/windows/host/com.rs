use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

use crate::platform::windows::error::{PlatformError, PlatformResult};

pub struct ComApartment;

impl ComApartment {
    pub fn init() -> PlatformResult<Self> {
        // SAFETY: Called once on the UI thread before COM-backed Direct2D/DirectWrite resources are
        // created. The reserved pointer is None as required by CoInitializeEx.
        let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        if result.is_err() {
            return Err(PlatformError::Windows {
                context: "initialize COM apartment",
                source: windows::core::Error::from_hresult(result),
            });
        }
        Ok(Self)
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        // SAFETY: Balanced with a successful CoInitializeEx on the same UI thread.
        unsafe {
            CoUninitialize();
        }
    }
}
