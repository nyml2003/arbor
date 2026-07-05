#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Word,
    StringDouble,
    StringSingle,
    StringTriple,
    Pipe,
    RedirectOut,
    RedirectAppend,
    Eol,
    Eof,
}

impl TokenKind {
    pub fn is_argument(self) -> bool {
        matches!(
            self,
            Self::Word | Self::StringDouble | Self::StringSingle | Self::StringTriple
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub value: String,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(kind: TokenKind, value: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            kind,
            value: value.into(),
            line,
            column,
        }
    }
}
