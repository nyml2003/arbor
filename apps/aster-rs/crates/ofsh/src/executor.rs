use thiserror::Error;

use crate::ast::StatementNode;
use crate::error::ErrorPhase;
use crate::registry::{CommandRegistry, CommandResult};

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ExecutionError {
    #[error("{feature} is parsed but not supported by the v1 executor")]
    UnsupportedFeature {
        phase: ErrorPhase,
        feature: &'static str,
    },
    #[error("unknown command \"{command_name}\"")]
    UnknownCommand {
        phase: ErrorPhase,
        command_name: String,
    },
}

impl ExecutionError {
    fn unsupported(feature: &'static str) -> Self {
        Self::UnsupportedFeature {
            phase: ErrorPhase::Runtime,
            feature,
        }
    }

    fn unknown(command_name: impl Into<String>) -> Self {
        Self::UnknownCommand {
            phase: ErrorPhase::Runtime,
            command_name: command_name.into(),
        }
    }
}

pub fn execute(
    statement: &StatementNode,
    registry: &CommandRegistry,
) -> Result<CommandResult, ExecutionError> {
    if statement.pipeline.commands.len() > 1 {
        return Err(ExecutionError::unsupported("pipe"));
    }
    if statement.redirection.is_some() {
        return Err(ExecutionError::unsupported("redirection"));
    }

    let command = statement
        .pipeline
        .commands
        .first()
        .expect("parser only creates non-empty pipelines");
    let args = command
        .args
        .iter()
        .map(|arg| arg.value.clone())
        .collect::<Vec<_>>();
    let registered = registry
        .get(&command.name.value)
        .ok_or_else(|| ExecutionError::unknown(command.name.value.clone()))?;

    Ok(registered.call(&args))
}
