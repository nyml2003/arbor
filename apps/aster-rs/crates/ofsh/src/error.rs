use thiserror::Error;

use crate::executor::ExecutionError;
use crate::lexer::LexerError;
use crate::parser::ParseError;
use crate::resolver::ResolveError;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ErrorPhase {
    Lexer,
    Parse,
    Semantic,
    Runtime,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum OfshError {
    #[error(transparent)]
    Lexer(#[from] LexerError),
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error(transparent)]
    Resolve(#[from] ResolveError),
    #[error(transparent)]
    Execute(#[from] ExecutionError),
}
