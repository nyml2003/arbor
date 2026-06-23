use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

use super::super::error::{PlatformError, PlatformResult};

#[derive(Debug)]
pub struct ComApartment;

impl ComApartment {
    pub fn init() -> PlatformResult<Self> {
        // SAFETY: The UI thread initializes COM once during startup and uninitializes on drop.
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
        // SAFETY: Matches the successful CoInitializeEx call for this UI thread.
        unsafe {
            CoUninitialize();
        }
    }
}
