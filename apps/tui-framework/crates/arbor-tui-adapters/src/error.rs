// Structured error types for arbor-tui-adapters.
// Library crate → thiserror.

/// Errors from terminal I/O and backend operations.
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    /// Underlying I/O failure (stdout write, etc.).
    #[error("terminal I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Terminal size query failed — typically means stdin is not a TTY.
    #[error("failed to query terminal size — is stdin a TTY?")]
    SizeQueryFailed,

    /// Failed to enter raw mode.
    #[error("failed to enter raw mode")]
    RawModeFailed,
}
