use thiserror::Error;

use crate::ast::{
    Argument, ArgumentKind, CommandNode, PipelineNode, RedirectionNode, RedirectionOperator,
    StatementNode,
};
use crate::error::ErrorPhase;
use crate::token::{Token, TokenKind};

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{message} at {line}:{column}")]
pub struct ParseError {
    pub phase: ErrorPhase,
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl ParseError {
    fn new(message: impl Into<String>, token: &Token) -> Self {
        Self {
            phase: ErrorPhase::Parse,
            message: message.into(),
            line: token.line,
            column: token.column,
        }
    }
}

pub struct Parser<'a> {
    tokens: &'a [Token],
    position: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    pub fn parse(mut self) -> Result<StatementNode, ParseError> {
        self.parse_statement()
    }

    fn current(&self) -> &Token {
        self.tokens
            .get(self.position)
            .or_else(|| self.tokens.last())
            .expect("parser requires at least EOF token")
    }

    fn advance(&mut self) {
        if self.position + 1 < self.tokens.len() {
            self.position += 1;
        }
    }

    fn parse_statement(&mut self) -> Result<StatementNode, ParseError> {
        while self.current().kind == TokenKind::Eol {
            self.advance();
        }

        if self.current().kind == TokenKind::Eof {
            return Err(ParseError::new("Empty statement", self.current()));
        }

        let pipeline = self.parse_pipeline()?;
        let redirection = if matches!(
            self.current().kind,
            TokenKind::RedirectOut | TokenKind::RedirectAppend
        ) {
            Some(self.parse_redirection()?)
        } else {
            None
        };

        if !matches!(self.current().kind, TokenKind::Eol | TokenKind::Eof) {
            return Err(ParseError::new(
                "Expected end of line or end of file",
                self.current(),
            ));
        }

        Ok(StatementNode {
            pipeline,
            redirection,
        })
    }

    fn parse_pipeline(&mut self) -> Result<PipelineNode, ParseError> {
        let mut commands = vec![self.parse_command()?];

        while self.current().kind == TokenKind::Pipe {
            self.advance();
            commands.push(self.parse_command()?);
        }

        Ok(PipelineNode { commands })
    }

    fn parse_command(&mut self) -> Result<CommandNode, ParseError> {
        if self.current().kind != TokenKind::Word {
            return Err(ParseError::new(
                "Command name must be a plain word, not a string",
                self.current(),
            ));
        }

        let name = self.parse_argument()?;
        let mut args = Vec::new();
        while self.current().kind.is_argument() {
            args.push(self.parse_argument()?);
        }

        Ok(CommandNode { name, args })
    }

    fn parse_argument(&mut self) -> Result<Argument, ParseError> {
        let token = self.current();
        let kind = match token.kind {
            TokenKind::Word => ArgumentKind::Word,
            TokenKind::StringDouble => ArgumentKind::StringDouble,
            TokenKind::StringSingle => ArgumentKind::StringSingle,
            TokenKind::StringTriple => ArgumentKind::StringTriple,
            _ => return Err(ParseError::new("Expected argument", token)),
        };
        let argument = Argument::new(token.value.clone(), kind);
        self.advance();
        Ok(argument)
    }

    fn parse_redirection(&mut self) -> Result<RedirectionNode, ParseError> {
        let operator = match self.current().kind {
            TokenKind::RedirectOut => RedirectionOperator::Truncate,
            TokenKind::RedirectAppend => RedirectionOperator::Append,
            _ => {
                return Err(ParseError::new(
                    "Expected redirection operator",
                    self.current(),
                ))
            }
        };
        self.advance();

        if !self.current().kind.is_argument() {
            return Err(ParseError::new(
                "Expected path after redirect operator",
                self.current(),
            ));
        }

        Ok(RedirectionNode {
            operator,
            target: self.parse_argument()?,
        })
    }
}
