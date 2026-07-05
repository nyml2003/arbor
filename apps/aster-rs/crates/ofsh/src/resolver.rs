use thiserror::Error;

use crate::ast::StatementNode;
use crate::error::ErrorPhase;
use crate::registry::CommandRegistry;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{message}")]
pub struct ResolveError {
    pub phase: ErrorPhase,
    pub message: String,
    pub command_name: String,
}

impl ResolveError {
    fn unknown(command_name: impl Into<String>) -> Self {
        let command_name = command_name.into();
        Self {
            phase: ErrorPhase::Semantic,
            message: format!("unknown command \"{command_name}\""),
            command_name,
        }
    }
}

pub fn resolve(statement: &StatementNode, registry: &CommandRegistry) -> Result<(), ResolveError> {
    for command in &statement.pipeline.commands {
        if !registry.has(&command.name.value) {
            return Err(ResolveError::unknown(command.name.value.clone()));
        }
    }
    Ok(())
}
