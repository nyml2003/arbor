use thiserror::Error;

pub type PlatformResult<T> = Result<T, PlatformError>;

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("app error: {0}")]
    App(#[from] crate::app::AppError),
    #[error("windows api error: {context}: {source}")]
    Windows {
        context: &'static str,
        #[source]
        source: windows::core::Error,
    },
    #[error("rendering error: {0}")]
    Rendering(#[from] arbor_ui_windows::RenderError),
    #[error("input injection sent {sent} of {expected} events")]
    PartialInput { sent: u32, expected: usize },
    #[error("window state is unavailable")]
    MissingWindowState,
}

pub trait WindowsResultExt<T> {
    fn context(self, context: &'static str) -> PlatformResult<T>;
}

impl<T> WindowsResultExt<T> for windows::core::Result<T> {
    fn context(self, context: &'static str) -> PlatformResult<T> {
        self.map_err(|source| PlatformError::Windows { context, source })
    }
}
