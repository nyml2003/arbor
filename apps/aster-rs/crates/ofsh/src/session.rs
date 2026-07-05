use crate::commands::builtin_registry;
use crate::error::OfshError;
use crate::executor::execute;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::registry::{CommandRegistry, CommandResult};
use crate::resolver::resolve;

#[derive(Clone)]
pub struct OfshSession {
    registry: CommandRegistry,
}

impl Default for OfshSession {
    fn default() -> Self {
        Self::with_builtins()
    }
}

impl OfshSession {
    pub fn new(registry: CommandRegistry) -> Self {
        Self { registry }
    }

    pub fn with_builtins() -> Self {
        Self {
            registry: builtin_registry(),
        }
    }

    pub fn registry(&self) -> &CommandRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut CommandRegistry {
        &mut self.registry
    }

    pub fn execute(&self, line: &str) -> Result<CommandResult, OfshError> {
        if line.trim().is_empty() {
            return Ok(CommandResult::output(""));
        }

        let tokens = Lexer::new(line).tokenize()?;
        let statement = Parser::new(&tokens).parse()?;
        resolve(&statement, &self.registry)?;
        Ok(execute(&statement, &self.registry)?)
    }
}
