use crate::lexer::Lexer;
use crate::registry::{CommandRegistry, CommandSpec};
use crate::token::{Token, TokenKind};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CompletionKind {
    Command,
    Argument,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionItem {
    pub value: String,
    pub description: String,
    pub kind: CompletionKind,
}

impl CompletionItem {
    fn command(spec: CommandSpec) -> Self {
        Self {
            value: spec.name,
            description: spec.description,
            kind: CompletionKind::Command,
        }
    }

    fn argument(value: String) -> Self {
        Self {
            description: value.clone(),
            value,
            kind: CompletionKind::Argument,
        }
    }
}

pub struct CompletionEngine<'a> {
    registry: &'a CommandRegistry,
}

impl<'a> CompletionEngine<'a> {
    pub fn new(registry: &'a CommandRegistry) -> Self {
        Self { registry }
    }

    pub fn complete(&self, input: &str) -> Vec<CompletionItem> {
        let Ok(tokens) = Lexer::new(input).tokenize() else {
            return Vec::new();
        };
        let significant = tokens
            .iter()
            .filter(|token| !matches!(token.kind, TokenKind::Eof | TokenKind::Eol))
            .collect::<Vec<_>>();

        if significant.is_empty() {
            return self.command_completions("");
        }
        if significant.iter().any(|token| {
            matches!(
                token.kind,
                TokenKind::Pipe | TokenKind::RedirectOut | TokenKind::RedirectAppend
            )
        }) {
            return Vec::new();
        }

        let command = significant[0];
        if command.kind != TokenKind::Word {
            return Vec::new();
        }

        if significant.len() == 1 && !ends_with_spacing(input) {
            return self.command_completions(&command.value);
        }

        let Some(spec) = self.registry.get(&command.value).map(|entry| entry.spec()) else {
            return Vec::new();
        };
        let prefix = argument_prefix(&significant, input);
        spec.arg_completions
            .iter()
            .filter(|candidate| candidate.starts_with(prefix))
            .cloned()
            .map(CompletionItem::argument)
            .collect()
    }

    fn command_completions(&self, prefix: &str) -> Vec<CompletionItem> {
        self.registry
            .specs()
            .into_iter()
            .filter(|spec| spec.name.starts_with(prefix))
            .map(CompletionItem::command)
            .collect()
    }
}

fn argument_prefix<'a>(tokens: &'a [&Token], input: &str) -> &'a str {
    if ends_with_spacing(input) {
        return "";
    }
    tokens
        .last()
        .filter(|token| token.kind.is_argument())
        .map(|token| token.value.as_str())
        .unwrap_or("")
}

fn ends_with_spacing(input: &str) -> bool {
    input
        .chars()
        .last()
        .is_some_and(|ch| matches!(ch, ' ' | '\t' | '\r' | '\n'))
}
