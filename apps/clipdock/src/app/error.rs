use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AppError {
    #[error("history item is unavailable: {0}")]
    MissingHistoryItem(String),
}
