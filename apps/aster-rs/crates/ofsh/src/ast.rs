#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ArgumentKind {
    Word,
    StringDouble,
    StringSingle,
    StringTriple,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Argument {
    pub value: String,
    pub kind: ArgumentKind,
}

impl Argument {
    pub fn new(value: impl Into<String>, kind: ArgumentKind) -> Self {
        Self {
            value: value.into(),
            kind,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandNode {
    pub name: Argument,
    pub args: Vec<Argument>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PipelineNode {
    pub commands: Vec<CommandNode>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RedirectionOperator {
    Truncate,
    Append,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedirectionNode {
    pub operator: RedirectionOperator,
    pub target: Argument,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatementNode {
    pub pipeline: PipelineNode,
    pub redirection: Option<RedirectionNode>,
}
