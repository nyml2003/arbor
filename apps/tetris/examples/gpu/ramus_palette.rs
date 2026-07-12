use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Mutex};

use nucleo_matcher::Matcher;
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use punctum_tetris::{BOARD_HEIGHT, BOARD_WIDTH, PieceKind, Rotation, TetrisCommand, TetrisState};
use ramus_core::{
    AuthorizationService, Capability, Catalog, CompileLimits, Compiler, Diagnostic, Effect,
    EffectPermit, ExecutionError, ExecutionFailure, MethodName, MethodRegistration, MethodSchema,
    NodePath, ParseDiagnosticKind, ParseFailure, ParseLimits, PlanDraft, Principal, Provider,
    ProviderError, ProviderId, ProviderRequest, Runtime, SchemaVersion, Value, parse_with_limits,
};

const PLAYER_ID: &str = "local-player";
const PROVIDER_ID: &str = "tetris";
const DEVELOPER_INVOCATION: (&str, &str) = ("/developer/tetris", "inspect");
const STATE_INVOCATION: (&str, &str) = ("/tetris/game", "state");
pub const AUTOPLAY_INVOCATION: &str = "/tetris/agent autoplay";
const COMMAND_INVOCATIONS: [(&str, &str); 6] = [
    ("/tetris/piece", "left"),
    ("/tetris/piece", "right"),
    ("/tetris/piece", "rotate"),
    ("/tetris/piece", "soft-drop"),
    ("/tetris/piece", "hard-drop"),
    ("/tetris/game", "restart"),
];

const PARSE_LIMITS: ParseLimits = ParseLimits {
    max_source_bytes: 256,
    max_calls: 1,
    max_arguments_per_call: 0,
};

const COMPILE_LIMITS: CompileLimits = CompileLimits {
    max_calls: 1,
    max_arguments_per_call: 0,
    max_total_bytes: 256,
    max_value_bytes: 0,
    max_value_nodes: 0,
    max_value_depth: 0,
};

pub type CommandQueue = Arc<Mutex<VecDeque<TetrisCommand>>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaletteIntent {
    Open,
    Close,
    InsertText(String),
    Backspace,
    Next,
    Previous,
    Execute,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteOutcome {
    Updated,
    Closed,
    Executed,
    AutoplayRequested,
    NoSelection,
    Failed,
    Ignored,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticStage {
    Selection,
    Parse,
    Seal,
    Provider,
    Runtime,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteDiagnostic {
    pub stage: DiagnosticStage,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PaletteState {
    open: bool,
    query: String,
    items: Vec<String>,
    selected_index: Option<usize>,
    diagnostic: Option<PaletteDiagnostic>,
}

impl PaletteState {
    pub const fn is_open(&self) -> bool {
        self.open
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn items(&self) -> &[String] {
        &self.items
    }

    pub const fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub const fn diagnostic(&self) -> Option<&PaletteDiagnostic> {
        self.diagnostic.as_ref()
    }

    fn replace_items(&mut self, items: Vec<String>) {
        self.items = items;
        self.selected_index = (!self.items.is_empty()).then_some(0);
    }

    fn clear_diagnostic(&mut self) {
        self.diagnostic = None;
    }
}

pub struct RamusPalette {
    authorization: AuthorizationService,
    principal: Principal,
    compiler: Compiler,
    runtime: Runtime,
    #[allow(dead_code)]
    game_state: Arc<Mutex<Value>>,
}

impl RamusPalette {
    pub fn new(command_queue: CommandQueue) -> Self {
        let provider_id = provider_id();
        let catalog = build_catalog(&provider_id);
        let game_state = Arc::new(Mutex::new(Value::Unit));
        let authorization = AuthorizationService::new();
        let principal = authorization
            .create_principal(PLAYER_ID)
            .expect("the fixed local player principal is valid and unique");
        grant_player_commands(&authorization, &principal);

        let compiler = Compiler::new(Arc::clone(&catalog));
        let mut runtime = Runtime::new(catalog, authorization.checker());
        runtime
            .bind_provider(
                provider_id,
                Arc::new(TetrisProvider {
                    command_queue,
                    game_state: Arc::clone(&game_state),
                }),
            )
            .expect("the Tetris provider is bound exactly once");

        Self {
            authorization,
            principal,
            compiler,
            runtime,
            game_state,
        }
    }

    pub fn discover_invocations(&self) -> Vec<String> {
        let session = self
            .authorization
            .session(&self.principal)
            .expect("the local player belongs to this authority");
        let mut invocations = self
            .compiler
            .discover(&session.view())
            .into_iter()
            .filter(|entry| is_command_invocation(entry.path.as_str(), entry.method.as_str()))
            .map(|entry| format!("{} {}", entry.path.as_str(), entry.method.as_str()))
            .collect::<Vec<_>>();
        invocations.sort();
        invocations
    }

    pub fn complete_invocations(&self, prefix: &str) -> Vec<String> {
        let session = self
            .authorization
            .session(&self.principal)
            .expect("the local player belongs to this authority");
        let mut invocations = self
            .compiler
            .complete(&session.view(), prefix)
            .into_iter()
            .filter(|completion| is_command_text(&completion.invocation))
            .map(|completion| completion.invocation)
            .collect::<Vec<_>>();
        invocations.sort();
        invocations
    }

    pub fn handle(&self, state: &mut PaletteState, intent: PaletteIntent) -> PaletteOutcome {
        match intent {
            PaletteIntent::Open => {
                state.open = true;
                state.query.clear();
                state.clear_diagnostic();
                self.refresh_items(state);
                PaletteOutcome::Updated
            }
            PaletteIntent::Close if state.open => {
                state.open = false;
                state.clear_diagnostic();
                PaletteOutcome::Closed
            }
            PaletteIntent::Close => PaletteOutcome::Ignored,
            PaletteIntent::InsertText(text) if state.open => {
                state.query.push_str(&text);
                state.clear_diagnostic();
                self.refresh_items(state);
                PaletteOutcome::Updated
            }
            PaletteIntent::Backspace if state.open => {
                state.query.pop();
                state.clear_diagnostic();
                self.refresh_items(state);
                PaletteOutcome::Updated
            }
            PaletteIntent::Next if state.open => {
                state.selected_index = match (state.selected_index, state.items.len()) {
                    (_, 0) => None,
                    (Some(index), len) => Some((index + 1) % len),
                    (None, _) => Some(0),
                };
                PaletteOutcome::Updated
            }
            PaletteIntent::Previous if state.open => {
                state.selected_index = match (state.selected_index, state.items.len()) {
                    (_, 0) => None,
                    (Some(0), len) | (None, len) => Some(len - 1),
                    (Some(index), _) => Some(index - 1),
                };
                PaletteOutcome::Updated
            }
            PaletteIntent::Execute if state.open => self.execute_selected(state),
            _ => PaletteOutcome::Ignored,
        }
    }

    pub fn execute_invocation(&self, invocation: &str) -> Result<(), PaletteDiagnostic> {
        self.execute_outputs(invocation).map(|_| ())
    }

    #[allow(dead_code)]
    pub fn observe_game_state(&self, state: &TetrisState) -> Result<Value, PaletteDiagnostic> {
        *self.game_state.lock().map_err(|_| PaletteDiagnostic {
            stage: DiagnosticStage::Provider,
            code: "observation-unavailable".into(),
            message: "the Tetris observation store is unavailable".into(),
        })? = game_state_value(state);
        let invocation = format!("{} {}", STATE_INVOCATION.0, STATE_INVOCATION.1);
        let mut outputs = self.execute_outputs(&invocation)?;
        Ok(outputs
            .pop()
            .expect("a single read call produces exactly one output"))
    }

    fn execute_outputs(&self, invocation: &str) -> Result<Vec<Value>, PaletteDiagnostic> {
        let document = parse_with_limits(invocation, PARSE_LIMITS).map_err(parse_diagnostic)?;
        let plan = {
            let session = self
                .authorization
                .session(&self.principal)
                .expect("the local player belongs to this authority");
            self.compiler
                .seal_with_limits(&session.view(), PlanDraft::from(document), COMPILE_LIMITS)
                .map_err(seal_diagnostic)?
        };
        let report = self.runtime.execute(plan).map_err(execution_diagnostic)?;
        Ok(report.outputs)
    }

    fn authorized_invocations(&self) -> Vec<String> {
        let discovered = self
            .discover_invocations()
            .into_iter()
            .collect::<BTreeSet<_>>();
        self.complete_invocations("")
            .into_iter()
            .filter(|invocation| discovered.contains(invocation))
            .collect()
    }

    fn human_invocations(&self) -> Vec<String> {
        let mut invocations = self.authorized_invocations();
        invocations.push(AUTOPLAY_INVOCATION.into());
        invocations.sort();
        invocations
    }

    fn refresh_items(&self, state: &mut PaletteState) {
        let pattern = Pattern::new(
            &state.query,
            CaseMatching::Smart,
            Normalization::Smart,
            AtomKind::Fuzzy,
        );
        let mut matcher = Matcher::default();
        let mut matches = pattern.match_list(self.human_invocations(), &mut matcher);
        matches.sort_by(|(left, left_score), (right, right_score)| {
            right_score.cmp(left_score).then_with(|| left.cmp(right))
        });
        state.replace_items(
            matches
                .into_iter()
                .map(|(invocation, _)| invocation)
                .collect(),
        );
    }

    fn execute_selected(&self, state: &mut PaletteState) -> PaletteOutcome {
        let Some(invocation) = state
            .selected_index
            .and_then(|index| state.items.get(index))
            .cloned()
        else {
            state.diagnostic = Some(PaletteDiagnostic {
                stage: DiagnosticStage::Selection,
                code: "no-selection".into(),
                message: "no command is selected".into(),
            });
            return PaletteOutcome::NoSelection;
        };

        if invocation == AUTOPLAY_INVOCATION {
            state.open = false;
            state.clear_diagnostic();
            return PaletteOutcome::AutoplayRequested;
        }

        match self.execute_invocation(&invocation) {
            Ok(()) => {
                state.open = false;
                state.clear_diagnostic();
                PaletteOutcome::Executed
            }
            Err(diagnostic) => {
                state.diagnostic = Some(diagnostic);
                PaletteOutcome::Failed
            }
        }
    }
}

struct TetrisProvider {
    command_queue: CommandQueue,
    game_state: Arc<Mutex<Value>>,
}

impl Provider for TetrisProvider {
    fn execute(
        &self,
        permit: EffectPermit,
        request: &ProviderRequest,
    ) -> Result<Value, ProviderError> {
        let state_request = is_state_request(request);
        let expected_capability = if state_request {
            Capability::Read
        } else {
            Capability::Invoke
        };
        if permit.principal().as_str() != PLAYER_ID
            || permit.capability() != expected_capability
            || permit.path() != &request.path
            || permit.method() != &request.method
        {
            return Err(rejected(
                "invalid-permit",
                "the invocation permit does not match the provider request",
            ));
        }
        if !request.arguments.is_empty() {
            return Err(rejected(
                "unexpected-arguments",
                "Tetris palette commands do not accept arguments",
            ));
        }

        if state_request {
            return self
                .game_state
                .lock()
                .map(|state| state.clone())
                .map_err(|_| {
                    rejected(
                        "observation-unavailable",
                        "the Tetris observation store is unavailable",
                    )
                });
        }

        let command = command_for_request(request).ok_or_else(|| {
            rejected(
                "unknown-command",
                "the Tetris provider does not implement this invocation",
            )
        })?;
        self.command_queue
            .lock()
            .map_err(|_| {
                rejected(
                    "command-queue-unavailable",
                    "the host command queue is unavailable",
                )
            })?
            .push_back(command);
        Ok(Value::Unit)
    }
}

fn build_catalog(provider_id: &ProviderId) -> Arc<Catalog> {
    let mut catalog = Catalog::new();
    for (path, method) in COMMAND_INVOCATIONS
        .into_iter()
        .chain([DEVELOPER_INVOCATION])
    {
        let method = MethodName::new(method).expect("fixed method names are valid");
        catalog
            .register(MethodRegistration {
                provider_id: provider_id.clone(),
                path: NodePath::parse(path).expect("fixed node paths are valid"),
                schema: MethodSchema::new(method, vec![])
                    .expect("parameter-free method schemas are valid"),
                schema_version: SchemaVersion::new(1).expect("schema version is non-zero"),
                effect: Effect::Invoke,
            })
            .expect("fixed catalog entries are unique");
    }
    catalog
        .register(MethodRegistration {
            provider_id: provider_id.clone(),
            path: NodePath::parse(STATE_INVOCATION.0).expect("the state path is valid"),
            schema: MethodSchema::new(
                MethodName::new(STATE_INVOCATION.1).expect("the state method is valid"),
                vec![],
            )
            .expect("the state method schema is valid"),
            schema_version: SchemaVersion::new(1).expect("schema version is non-zero"),
            effect: Effect::Read,
        })
        .expect("the state method is unique");
    Arc::new(catalog)
}

fn grant_player_commands(authorization: &AuthorizationService, principal: &Principal) {
    for (path, method) in COMMAND_INVOCATIONS {
        let path = NodePath::parse(path).expect("fixed node paths are valid");
        let method = MethodName::new(method).expect("fixed method names are valid");
        for capability in [
            Capability::Discover,
            Capability::Complete,
            Capability::Invoke,
        ] {
            authorization
                .grant(principal, path.clone(), Some(method.clone()), capability)
                .expect("the local player belongs to this authority");
        }
    }
    let path = NodePath::parse(STATE_INVOCATION.0).expect("the state path is valid");
    let method = MethodName::new(STATE_INVOCATION.1).expect("the state method is valid");
    for capability in [Capability::Discover, Capability::Read] {
        authorization
            .grant(principal, path.clone(), Some(method.clone()), capability)
            .expect("the local player belongs to this authority");
    }
}

fn provider_id() -> ProviderId {
    ProviderId::new(PROVIDER_ID).expect("the fixed provider id is valid")
}

fn command_for_request(request: &ProviderRequest) -> Option<TetrisCommand> {
    match (request.path.as_str(), request.method.as_str()) {
        ("/tetris/piece", "left") => Some(TetrisCommand::MoveLeft),
        ("/tetris/piece", "right") => Some(TetrisCommand::MoveRight),
        ("/tetris/piece", "rotate") => Some(TetrisCommand::RotateClockwise),
        ("/tetris/piece", "soft-drop") => Some(TetrisCommand::SoftDrop),
        ("/tetris/piece", "hard-drop") => Some(TetrisCommand::HardDrop),
        ("/tetris/game", "restart") => Some(TetrisCommand::Restart),
        _ => None,
    }
}

fn is_command_invocation(path: &str, method: &str) -> bool {
    COMMAND_INVOCATIONS
        .iter()
        .any(|candidate| candidate.0 == path && candidate.1 == method)
}

fn is_command_text(invocation: &str) -> bool {
    COMMAND_INVOCATIONS
        .iter()
        .any(|candidate| invocation == format!("{} {}", candidate.0, candidate.1))
}

fn is_state_request(request: &ProviderRequest) -> bool {
    request.path.as_str() == STATE_INVOCATION.0 && request.method.as_str() == STATE_INVOCATION.1
}

#[allow(dead_code)]
fn game_state_value(state: &TetrisState) -> Value {
    let active = state.active_piece().map_or(Value::Unit, |piece| {
        Value::Record(BTreeMap::from([
            ("col".into(), Value::Integer(i64::from(piece.col()))),
            (
                "kind".into(),
                Value::String(piece_name(piece.kind()).into()),
            ),
            (
                "rotation".into(),
                Value::String(rotation_name(piece.rotation()).into()),
            ),
            ("row".into(), Value::Integer(i64::from(piece.row()))),
        ]))
    });
    let locked_board = (0..BOARD_HEIGHT)
        .map(|row| {
            let cells = (0..BOARD_WIDTH)
                .map(|col| state.locked_cell(col, row).map_or('.', piece_symbol))
                .collect::<String>();
            Value::String(cells)
        })
        .collect::<Vec<_>>();

    Value::Record(BTreeMap::from([
        ("active".into(), active),
        (
            "board_height".into(),
            Value::Integer(i64::from(BOARD_HEIGHT)),
        ),
        ("board_width".into(), Value::Integer(i64::from(BOARD_WIDTH))),
        (
            "cleared_lines".into(),
            Value::Integer(i64::from(state.cleared_lines())),
        ),
        ("game_over".into(), Value::Boolean(state.is_game_over())),
        ("locked_board".into(), Value::List(locked_board)),
    ]))
}

#[allow(dead_code)]
const fn piece_name(kind: PieceKind) -> &'static str {
    match kind {
        PieceKind::I => "I",
        PieceKind::O => "O",
        PieceKind::T => "T",
        PieceKind::S => "S",
        PieceKind::Z => "Z",
        PieceKind::J => "J",
        PieceKind::L => "L",
    }
}

#[allow(dead_code)]
const fn piece_symbol(kind: PieceKind) -> char {
    match kind {
        PieceKind::I => 'I',
        PieceKind::O => 'O',
        PieceKind::T => 'T',
        PieceKind::S => 'S',
        PieceKind::Z => 'Z',
        PieceKind::J => 'J',
        PieceKind::L => 'L',
    }
}

#[allow(dead_code)]
const fn rotation_name(rotation: Rotation) -> &'static str {
    match rotation {
        Rotation::Spawn => "spawn",
        Rotation::Right => "right",
        Rotation::Reverse => "reverse",
        Rotation::Left => "left",
    }
}

fn rejected(code: &str, message: &str) -> ProviderError {
    ProviderError::Rejected {
        code: code.into(),
        message: message.into(),
    }
}

fn parse_diagnostic(failure: ParseFailure) -> PaletteDiagnostic {
    failure.diagnostics().first().map_or_else(
        || PaletteDiagnostic {
            stage: DiagnosticStage::Parse,
            code: "parse-failed".into(),
            message: failure.to_string(),
        },
        |diagnostic| PaletteDiagnostic {
            stage: DiagnosticStage::Parse,
            code: parse_diagnostic_code(&diagnostic.kind).into(),
            message: diagnostic.to_string(),
        },
    )
}

fn parse_diagnostic_code(kind: &ParseDiagnosticKind) -> &'static str {
    match kind {
        ParseDiagnosticKind::SourceTooLarge => "source-too-large",
        ParseDiagnosticKind::TooManyCalls => "too-many-calls",
        ParseDiagnosticKind::TooManyArguments => "too-many-arguments",
        ParseDiagnosticKind::EmptyInput => "empty-input",
        ParseDiagnosticKind::EmptyStatement => "empty-statement",
        ParseDiagnosticKind::ExpectedNodePath => "expected-node-path",
        ParseDiagnosticKind::InvalidNodePath { .. } => "invalid-node-path",
        ParseDiagnosticKind::ExpectedMethod => "expected-method",
        ParseDiagnosticKind::InvalidMethodName { .. } => "invalid-method-name",
        ParseDiagnosticKind::ExpectedArgument => "expected-argument",
        ParseDiagnosticKind::InvalidParameterName { .. } => "invalid-parameter-name",
        ParseDiagnosticKind::MissingArgumentValue => "missing-argument-value",
        ParseDiagnosticKind::WhitespaceAroundEquals => "whitespace-around-equals",
        ParseDiagnosticKind::MissingWhitespace => "missing-whitespace",
        ParseDiagnosticKind::UnterminatedString => "unterminated-string",
        ParseDiagnosticKind::InvalidEscape { .. } => "invalid-escape",
        ParseDiagnosticKind::IntegerOutOfRange { .. } => "integer-out-of-range",
        ParseDiagnosticKind::ForbiddenSyntax(_) => "forbidden-syntax",
        ParseDiagnosticKind::UnexpectedCharacter { .. } => "unexpected-character",
    }
}

fn seal_diagnostic(diagnostic: Diagnostic) -> PaletteDiagnostic {
    PaletteDiagnostic {
        stage: DiagnosticStage::Seal,
        code: diagnostic.code.as_str().into(),
        message: diagnostic.message,
    }
}

fn execution_diagnostic(failure: ExecutionFailure) -> PaletteDiagnostic {
    match failure.error {
        ExecutionError::Provider(ProviderError::Rejected { code, message }) => PaletteDiagnostic {
            stage: DiagnosticStage::Provider,
            code,
            message,
        },
        error => PaletteDiagnostic {
            stage: DiagnosticStage::Runtime,
            code: match error {
                ExecutionError::CatalogChanged => "catalog-changed",
                ExecutionError::SchemaChanged => "schema-changed",
                ExecutionError::AuthorizationRevoked => "authorization-revoked",
                ExecutionError::ProviderUnavailable => "provider-unavailable",
                ExecutionError::Provider(_) => unreachable!("provider failures are handled above"),
            }
            .into(),
            message: format!(
                "runtime execution failed at call {}: {error:?}",
                failure.call_index
            ),
        },
    }
}
