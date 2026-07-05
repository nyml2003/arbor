use thiserror::Error;

use crate::error::ErrorPhase;
use crate::token::{Token, TokenKind};

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{message} at {line}:{column}")]
pub struct LexerError {
    pub phase: ErrorPhase,
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl LexerError {
    fn new(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            phase: ErrorPhase::Lexer,
            message: message.into(),
            line,
            column,
        }
    }
}

pub struct Lexer {
    chars: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            chars: input.into().chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();

        while !self.is_eof() {
            self.skip_whitespace();
            if self.is_eof() {
                break;
            }

            match self.peek(0) {
                '\n' => tokens.push(self.tokenize_eol()),
                '|' => tokens.push(self.tokenize_pipe()),
                '>' => tokens.push(self.tokenize_redirect()),
                '"' if self.peek(1) == '"' && self.peek(2) == '"' => {
                    tokens.push(self.tokenize_triple_quote()?);
                }
                '"' => tokens.push(self.tokenize_double_quote()?),
                '\'' => tokens.push(self.tokenize_single_quote()?),
                _ => tokens.push(self.tokenize_word()),
            }
        }

        tokens.push(Token::new(TokenKind::Eof, "", self.line, self.column));
        Ok(tokens)
    }

    fn is_eof(&self) -> bool {
        self.position >= self.chars.len()
    }

    fn peek(&self, offset: usize) -> char {
        self.chars
            .get(self.position + offset)
            .copied()
            .unwrap_or('\0')
    }

    fn advance(&mut self) -> char {
        let ch = self.peek(0);
        self.position += 1;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(0), ' ' | '\t' | '\r') {
            self.advance();
        }
    }

    fn tokenize_word(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        let mut value = String::new();

        while !self.is_eof() {
            let ch = self.peek(0);
            if matches!(ch, ' ' | '\t' | '\r' | '\n' | '|' | '>' | '"' | '\'') {
                break;
            }
            value.push(self.advance());
        }

        Token::new(TokenKind::Word, value, start_line, start_column)
    }

    fn tokenize_double_quote(&mut self) -> Result<Token, LexerError> {
        let start_line = self.line;
        let start_column = self.column;
        let mut value = String::new();
        self.advance();

        while !self.is_eof() {
            match self.peek(0) {
                '"' => {
                    self.advance();
                    return Ok(Token::new(
                        TokenKind::StringDouble,
                        value,
                        start_line,
                        start_column,
                    ));
                }
                '\\' => {
                    self.advance();
                    value.push(self.read_escape()?);
                }
                _ => value.push(self.advance()),
            }
        }

        Err(LexerError::new(
            "Unclosed double quote string",
            self.line,
            self.column,
        ))
    }

    fn tokenize_single_quote(&mut self) -> Result<Token, LexerError> {
        let start_line = self.line;
        let start_column = self.column;
        let mut value = String::new();
        self.advance();

        while !self.is_eof() {
            if self.peek(0) == '\'' {
                self.advance();
                return Ok(Token::new(
                    TokenKind::StringSingle,
                    value,
                    start_line,
                    start_column,
                ));
            }
            value.push(self.advance());
        }

        Err(LexerError::new(
            "Unclosed single quote string",
            self.line,
            self.column,
        ))
    }

    fn tokenize_triple_quote(&mut self) -> Result<Token, LexerError> {
        let start_line = self.line;
        let start_column = self.column;
        let mut value = String::new();
        self.advance();
        self.advance();
        self.advance();

        while !self.is_eof() {
            if self.peek(0) == '"' && self.peek(1) == '"' && self.peek(2) == '"' {
                self.advance();
                self.advance();
                self.advance();
                return Ok(Token::new(
                    TokenKind::StringTriple,
                    value,
                    start_line,
                    start_column,
                ));
            }
            value.push(self.advance());
        }

        Err(LexerError::new(
            "Unclosed triple quote string",
            self.line,
            self.column,
        ))
    }

    fn read_escape(&mut self) -> Result<char, LexerError> {
        let escaped = match self.peek(0) {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '\\' => '\\',
            '"' => '"',
            '\0' => {
                return Err(LexerError::new(
                    "Invalid escape sequence: \\",
                    self.line,
                    self.column,
                ));
            }
            other => {
                return Err(LexerError::new(
                    format!("Invalid escape sequence: \\{other}"),
                    self.line,
                    self.column,
                ));
            }
        };
        self.advance();
        Ok(escaped)
    }

    fn tokenize_redirect(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        if self.peek(1) == '>' {
            self.advance();
            self.advance();
            return Token::new(TokenKind::RedirectAppend, ">>", start_line, start_column);
        }

        self.advance();
        Token::new(TokenKind::RedirectOut, ">", start_line, start_column)
    }

    fn tokenize_pipe(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        self.advance();
        Token::new(TokenKind::Pipe, "|", start_line, start_column)
    }

    fn tokenize_eol(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        self.advance();
        Token::new(TokenKind::Eol, "\n", start_line, start_column)
    }
}
