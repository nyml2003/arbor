//! Rust-native command shell core inspired by ObolosFS ofsh.
//!
//! This crate is UI-agnostic and filesystem-agnostic. It owns command parsing,
//! resolution, execution dispatch, and completion primitives. VFS, pipes, and
//! redirection are represented in the AST but intentionally unsupported by the
//! v1 executor.

pub mod ast;
pub mod commands;
pub mod completion;
pub mod error;
pub mod executor;
pub mod lexer;
pub mod parser;
pub mod registry;
pub mod resolver;
pub mod session;
pub mod token;

pub use ast::{
    Argument, ArgumentKind, CommandNode, PipelineNode, RedirectionNode, RedirectionOperator,
    StatementNode,
};
pub use commands::builtin_registry;
pub use completion::{CompletionEngine, CompletionItem, CompletionKind};
pub use error::{ErrorPhase, OfshError};
pub use executor::{execute, ExecutionError};
pub use lexer::{Lexer, LexerError};
pub use parser::{ParseError, Parser};
pub use registry::{CommandRegistry, CommandResult, CommandSpec, RegisteredCommand};
pub use resolver::{resolve, ResolveError};
pub use session::OfshSession;
pub use token::{Token, TokenKind};
