use thiserror::Error;

pub type RenderResult<T> = Result<T, RenderError>;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("windows api error: {context}: {source}")]
    Windows {
        context: &'static str,
        #[source]
        source: windows::core::Error,
    },
}

pub(crate) trait WindowsResultExt<T> {
    fn context(self, context: &'static str) -> RenderResult<T>;
}

impl<T> WindowsResultExt<T> for windows::core::Result<T> {
    fn context(self, context: &'static str) -> RenderResult<T> {
        self.map_err(|source| RenderError::Windows { context, source })
    }
}
