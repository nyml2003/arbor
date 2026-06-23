use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error, PartialEq)]
pub enum AppError {
    #[error("keyboard layout is empty")]
    EmptyLayout,
    #[error("keyboard row {0} is empty")]
    EmptyRow(usize),
    #[error("key width must be positive for {0}")]
    InvalidKeyWidth(String),
    #[error("unknown key id: {0}")]
    UnknownKey(String),
}
