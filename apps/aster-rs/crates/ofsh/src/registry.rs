use std::collections::BTreeMap;
use std::sync::Arc;

pub type CommandHandler = Arc<dyn Fn(&[String]) -> CommandResult + Send + Sync>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandSpec {
    pub name: String,
    pub description: String,
    pub arg_completions: Vec<String>,
}

impl CommandSpec {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            arg_completions: Vec::new(),
        }
    }

    pub fn with_arg_completions(
        mut self,
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.arg_completions = values.into_iter().map(Into::into).collect();
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandResult {
    pub output: String,
    pub exit: bool,
}

impl CommandResult {
    pub fn output(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            exit: false,
        }
    }

    pub fn exit() -> Self {
        Self {
            output: String::new(),
            exit: true,
        }
    }
}

#[derive(Clone)]
pub struct RegisteredCommand {
    spec: CommandSpec,
    handler: CommandHandler,
}

impl RegisteredCommand {
    pub fn spec(&self) -> &CommandSpec {
        &self.spec
    }

    pub fn call(&self, args: &[String]) -> CommandResult {
        (self.handler)(args)
    }
}

#[derive(Clone, Default)]
pub struct CommandRegistry {
    commands: BTreeMap<String, RegisteredCommand>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        spec: CommandSpec,
        handler: impl Fn(&[String]) -> CommandResult + Send + Sync + 'static,
    ) -> Option<CommandSpec> {
        let name = spec.name.clone();
        let replaced = self.commands.insert(
            name,
            RegisteredCommand {
                spec,
                handler: Arc::new(handler),
            },
        );
        replaced.map(|command| command.spec)
    }

    pub fn unregister(&mut self, name: &str) -> Option<CommandSpec> {
        self.commands.remove(name).map(|command| command.spec)
    }

    pub fn has(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    pub fn get(&self, name: &str) -> Option<&RegisteredCommand> {
        self.commands.get(name)
    }

    pub fn list(&self) -> Vec<String> {
        self.commands.keys().cloned().collect()
    }

    pub fn specs(&self) -> Vec<CommandSpec> {
        self.commands
            .values()
            .map(|command| command.spec.clone())
            .collect()
    }
}
