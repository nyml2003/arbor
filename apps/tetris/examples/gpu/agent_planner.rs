use std::{
    env,
    error::Error,
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{SyncSender, TrySendError, sync_channel},
    },
    thread,
    time::Duration,
};

use ramus_core::Value;
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};

const DEFAULT_ENDPOINT: &str = "https://api.deepseek.com/chat/completions";
const DEFAULT_MODEL: &str = "deepseek-v4-pro";
const MAX_RESPONSE_BYTES: u64 = 64 * 1024;
const MAX_INVOCATION_BYTES: usize = 256;
const MAX_PLAN_ACTIONS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestId(u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannerJob {
    pub id: RequestId,
    pub prompt: String,
    pub context: String,
    pub allowed_invocations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannerDecision {
    pub invocations: Vec<String>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannerCompletion {
    pub id: RequestId,
    pub result: Result<PlannerDecision, PlannerError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlannerError {
    EmptyPrompt,
    Busy,
    WorkerStopped,
    MissingApiKey,
    HttpStatus(u16),
    Timeout,
    Unavailable,
    ResponseTruncated,
    EmptyResponse,
    InvalidResponse,
    EmptyInvocation,
    InvocationTooLarge,
    PlanTooLong,
}

impl PlannerError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::EmptyPrompt => "empty-prompt",
            Self::Busy => "busy",
            Self::WorkerStopped => "worker-stopped",
            Self::MissingApiKey => "missing-api-key",
            Self::HttpStatus(_) => "http-status",
            Self::Timeout => "timeout",
            Self::Unavailable => "unavailable",
            Self::ResponseTruncated => "response-truncated",
            Self::EmptyResponse => "empty-response",
            Self::InvalidResponse => "invalid-response",
            Self::EmptyInvocation => "empty-invocation",
            Self::InvocationTooLarge => "invocation-too-large",
            Self::PlanTooLong => "plan-too-long",
        }
    }
}

impl fmt::Display for PlannerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPrompt => formatter.write_str("enter a natural-language command first"),
            Self::Busy => formatter.write_str("a DeepSeek request is already running"),
            Self::WorkerStopped => formatter.write_str("the DeepSeek worker has stopped"),
            Self::MissingApiKey => {
                formatter.write_str("set PUNCTUM_DEEPSEEK_API_KEY before using the agent")
            }
            Self::HttpStatus(status) => write!(formatter, "DeepSeek returned HTTP {status}"),
            Self::Timeout => formatter.write_str("the DeepSeek request timed out"),
            Self::Unavailable => formatter.write_str("DeepSeek is unavailable"),
            Self::ResponseTruncated => {
                formatter.write_str("DeepSeek exhausted its output token budget")
            }
            Self::EmptyResponse => formatter.write_str("DeepSeek returned empty content"),
            Self::InvalidResponse => formatter.write_str("DeepSeek returned an invalid response"),
            Self::EmptyInvocation => formatter.write_str("DeepSeek returned an empty invocation"),
            Self::InvocationTooLarge => {
                formatter.write_str("DeepSeek returned an invocation larger than 256 bytes")
            }
            Self::PlanTooLong => {
                formatter.write_str("DeepSeek returned more than 16 planned actions")
            }
        }
    }
}

impl Error for PlannerError {}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PlannerFailure {
    message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ActiveRequest {
    id: RequestId,
    attached: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlannerView<'a> {
    Idle,
    Pending,
    Message(&'a str),
    Failed(&'a str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompletionDisposition {
    Ignored,
    Decision(PlannerDecision),
    Failed,
}

#[derive(Debug, Default)]
pub struct PlannerSession {
    next_id: u64,
    active: Option<ActiveRequest>,
    message: Option<String>,
    failure: Option<PlannerFailure>,
}

impl PlannerSession {
    pub const fn is_pending(&self) -> bool {
        self.active.is_some()
    }

    pub fn begin(
        &mut self,
        prompt: &str,
        context: String,
        allowed_invocations: Vec<String>,
    ) -> Result<PlannerJob, PlannerError> {
        if self.active.is_some() {
            return Err(PlannerError::Busy);
        }
        let prompt = prompt.trim();
        if prompt.is_empty() {
            return Err(PlannerError::EmptyPrompt);
        }

        self.next_id = self.next_id.checked_add(1).unwrap_or(1);
        let id = RequestId(self.next_id);
        self.active = Some(ActiveRequest { id, attached: true });
        self.message = None;
        self.failure = None;
        Ok(PlannerJob {
            id,
            prompt: prompt.to_owned(),
            context,
            allowed_invocations,
        })
    }

    pub fn submit_failed(&mut self, id: RequestId, error: PlannerError) {
        if self.active.is_some_and(|active| active.id == id) {
            let attached = self.active.take().is_some_and(|active| active.attached);
            if attached {
                self.record_error(error);
            }
        }
    }

    pub fn complete(&mut self, completion: PlannerCompletion) -> CompletionDisposition {
        let Some(active) = self.active.filter(|active| active.id == completion.id) else {
            return CompletionDisposition::Ignored;
        };
        self.active = None;
        if !active.attached {
            return CompletionDisposition::Ignored;
        }

        match completion.result {
            Ok(decision) => CompletionDisposition::Decision(decision),
            Err(error) => {
                self.record_error(error);
                CompletionDisposition::Failed
            }
        }
    }

    pub fn detach(&mut self) {
        if let Some(active) = &mut self.active {
            active.attached = false;
        }
        self.message = None;
        self.failure = None;
    }

    pub fn clear_failure(&mut self) {
        self.failure = None;
        self.message = None;
    }

    pub fn record_message(&mut self, message: String) {
        self.failure = None;
        self.message = Some(message);
    }

    pub fn record_failure(&mut self, code: &str, message: &str) {
        self.message = None;
        self.failure = Some(PlannerFailure {
            message: format!("{code}: {message}"),
        });
    }

    pub fn record_error(&mut self, error: PlannerError) {
        self.record_failure(error.code(), &error.to_string());
    }

    pub fn view(&self) -> PlannerView<'_> {
        if self.active.is_some_and(|active| active.attached) {
            PlannerView::Pending
        } else if let Some(failure) = &self.failure {
            PlannerView::Failed(&failure.message)
        } else if let Some(message) = &self.message {
            PlannerView::Message(message)
        } else {
            PlannerView::Idle
        }
    }
}

pub trait PlannerTransport: Send + 'static {
    fn plan(&self, job: &PlannerJob) -> Result<PlannerDecision, PlannerError>;
}

pub struct PlannerWorker {
    sender: SyncSender<PlannerJob>,
    busy: Arc<AtomicBool>,
}

impl PlannerWorker {
    pub fn spawn<T, F>(transport: T, notify: F) -> Result<Self, std::io::Error>
    where
        T: PlannerTransport,
        F: Fn(PlannerCompletion) + Send + 'static,
    {
        let (sender, receiver) = sync_channel::<PlannerJob>(1);
        let busy = Arc::new(AtomicBool::new(false));
        let worker_busy = Arc::clone(&busy);
        thread::Builder::new()
            .name("tetris-deepseek-planner".into())
            .spawn(move || {
                while let Ok(job) = receiver.recv() {
                    let completion = PlannerCompletion {
                        id: job.id,
                        result: transport.plan(&job),
                    };
                    worker_busy.store(false, Ordering::Release);
                    notify(completion);
                }
            })?;
        Ok(Self { sender, busy })
    }

    pub fn try_submit(&self, job: PlannerJob) -> Result<(), PlannerError> {
        if self
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(PlannerError::Busy);
        }

        match self.sender.try_send(job) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => {
                self.busy.store(false, Ordering::Release);
                Err(PlannerError::Busy)
            }
            Err(TrySendError::Disconnected(_)) => {
                self.busy.store(false, Ordering::Release);
                Err(PlannerError::WorkerStopped)
            }
        }
    }
}

#[derive(Clone)]
pub struct DeepSeekConfig {
    pub endpoint: String,
    pub model: String,
    api_key: Option<String>,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
}

impl DeepSeekConfig {
    pub fn from_env() -> Self {
        Self {
            endpoint: nonempty_env("PUNCTUM_DEEPSEEK_URL")
                .unwrap_or_else(|| DEFAULT_ENDPOINT.into()),
            model: nonempty_env("PUNCTUM_DEEPSEEK_MODEL").unwrap_or_else(|| DEFAULT_MODEL.into()),
            api_key: nonempty_env("PUNCTUM_DEEPSEEK_API_KEY"),
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(60),
        }
    }
}

fn nonempty_env(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

pub struct DeepSeekTransport {
    config: DeepSeekConfig,
    agent: ureq::Agent,
}

impl DeepSeekTransport {
    pub fn new(config: DeepSeekConfig) -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_connect(Some(config.connect_timeout))
            .timeout_global(Some(config.request_timeout))
            .build()
            .into();
        Self { config, agent }
    }
}

impl PlannerTransport for DeepSeekTransport {
    fn plan(&self, job: &PlannerJob) -> Result<PlannerDecision, PlannerError> {
        let api_key = self
            .config
            .api_key
            .as_deref()
            .ok_or(PlannerError::MissingApiKey)?;
        let request = build_chat_request(&self.config.model, job);
        let authorization = format!("Bearer {api_key}");
        let mut response = self
            .agent
            .post(&self.config.endpoint)
            .header("Authorization", authorization)
            .send_json(&request)
            .map_err(map_http_error)?;
        let body = response
            .body_mut()
            .with_config()
            .limit(MAX_RESPONSE_BYTES)
            .read_to_string()
            .map_err(map_http_error)?;
        parse_chat_response(&body)
    }
}

pub fn format_observation(value: &Value) -> String {
    value_to_json(value).to_string()
}

fn value_to_json(value: &Value) -> JsonValue {
    match value {
        Value::String(value) => JsonValue::String(value.clone()),
        Value::Integer(value) => (*value).into(),
        Value::Boolean(value) => (*value).into(),
        Value::List(values) => JsonValue::Array(values.iter().map(value_to_json).collect()),
        Value::Record(values) => JsonValue::Object(
            values
                .iter()
                .map(|(key, value)| (key.clone(), value_to_json(value)))
                .collect(),
        ),
        Value::Unit => JsonValue::Null,
    }
}

fn build_chat_request(model: &str, job: &PlannerJob) -> JsonValue {
    let allowed = job
        .allowed_invocations
        .iter()
        .map(|invocation| format!("- {invocation}"))
        .collect::<Vec<_>>()
        .join("\n");
    let system = format!(
        "Return only a JSON object with exactly two fields: invocations and message. \
Invocations must be an array of at most {MAX_PLAN_ACTIONS} authorized Tetris invocations, each \
copied exactly from this list and ordered for execution. Use an empty array when no game action \
is appropriate. Always provide a brief response in message. \
Example planned JSON: {{\"invocations\":[\"/tetris/piece rotate\",\"/tetris/piece left\",\"/tetris/piece hard-drop\"],\"message\":\"Place the piece on the left.\"}}. \
Example response-only JSON: {{\"invocations\":[],\"message\":\"Hello.\"}}.\n{allowed}"
    );
    let user = format!(
        "Current Tetris state returned by the authorized Ramus read command \
`/tetris/game state`:\n{}\n\nUser request:\n{}",
        job.context, job.prompt
    );

    json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ],
        "stream": false,
        "thinking": { "type": "disabled" },
        "response_format": { "type": "json_object" },
        "temperature": 0,
        "max_tokens": 128 * 1024
    })
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    finish_reason: Option<String>,
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct PlannerOutput {
    invocations: Vec<String>,
    message: String,
}

fn parse_chat_response(body: &str) -> Result<PlannerDecision, PlannerError> {
    let response: ChatResponse =
        serde_json::from_str(body).map_err(|_| PlannerError::InvalidResponse)?;
    let choice = response
        .choices
        .first()
        .ok_or(PlannerError::InvalidResponse)?;
    if choice.finish_reason.as_deref() == Some("length") {
        return Err(PlannerError::ResponseTruncated);
    }
    let content = choice
        .message
        .content
        .as_deref()
        .filter(|content| !content.trim().is_empty())
        .ok_or(PlannerError::EmptyResponse)?;
    let output: PlannerOutput =
        serde_json::from_str(content).map_err(|_| PlannerError::InvalidResponse)?;
    let message = output.message.trim();
    if message.is_empty() {
        return Err(PlannerError::InvalidResponse);
    }
    if output.invocations.len() > MAX_PLAN_ACTIONS {
        return Err(PlannerError::PlanTooLong);
    }
    let invocations = output
        .invocations
        .into_iter()
        .map(|invocation| {
            let invocation = invocation.trim().to_owned();
            if invocation.is_empty() {
                Err(PlannerError::EmptyInvocation)
            } else if invocation.len() > MAX_INVOCATION_BYTES {
                Err(PlannerError::InvocationTooLarge)
            } else {
                Ok(invocation)
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PlannerDecision {
        invocations,
        message: message.to_owned(),
    })
}

fn map_http_error(error: ureq::Error) -> PlannerError {
    match error {
        ureq::Error::StatusCode(status) => PlannerError::HttpStatus(status),
        ureq::Error::Timeout(_) => PlannerError::Timeout,
        ureq::Error::Json(_) => PlannerError::InvalidResponse,
        _ => PlannerError::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::{Receiver, Sender, channel};

    use ramus_core::Value;

    use super::{
        CompletionDisposition, DeepSeekConfig, DeepSeekTransport, PlannerCompletion,
        PlannerDecision, PlannerError, PlannerJob, PlannerSession, PlannerTransport, PlannerView,
        PlannerWorker, RequestId, build_chat_request, format_observation, parse_chat_response,
    };

    struct BlockingTransport {
        started: Sender<()>,
        release: Receiver<()>,
    }

    impl PlannerTransport for BlockingTransport {
        fn plan(&self, _job: &PlannerJob) -> Result<PlannerDecision, PlannerError> {
            self.started.send(()).unwrap();
            self.release.recv().unwrap();
            Ok(command_decision("/tetris/piece rotate"))
        }
    }

    fn command_decision(invocation: &str) -> PlannerDecision {
        PlannerDecision {
            invocations: vec![invocation.into()],
            message: "Executing the requested command.".into(),
        }
    }

    fn job(id: u64) -> PlannerJob {
        PlannerJob {
            id: RequestId(id),
            prompt: "rotate the piece".into(),
            context: r#"{"active":{"kind":"T"}}"#.into(),
            allowed_invocations: vec!["/tetris/piece rotate".into()],
        }
    }

    #[test]
    fn session_tracks_pending_detached_and_duplicate_completions() {
        let mut session = PlannerSession::default();
        let request = session
            .begin(
                " rotate the piece ",
                "{}".into(),
                vec!["/tetris/piece rotate".into()],
            )
            .unwrap();
        assert_eq!(request.prompt, "rotate the piece");
        assert_eq!(session.view(), PlannerView::Pending);
        assert_eq!(
            session.begin("again", "{}".into(), Vec::new()),
            Err(PlannerError::Busy)
        );

        session.detach();
        assert_eq!(session.view(), PlannerView::Idle);
        let completion = PlannerCompletion {
            id: request.id,
            result: Ok(command_decision("/tetris/piece rotate")),
        };
        assert_eq!(
            session.complete(completion.clone()),
            CompletionDisposition::Ignored
        );
        assert_eq!(session.complete(completion), CompletionDisposition::Ignored);
    }

    #[test]
    fn transport_failures_become_ui_failures_without_candidates() {
        let mut session = PlannerSession::default();
        let request = session.begin("rotate", "{}".into(), Vec::new()).unwrap();

        assert_eq!(
            session.complete(PlannerCompletion {
                id: request.id,
                result: Err(PlannerError::Timeout),
            }),
            CompletionDisposition::Failed
        );
        assert!(
            matches!(session.view(), PlannerView::Failed(message) if message.contains("timeout"))
        );
    }

    #[test]
    fn worker_is_non_blocking_and_allows_only_one_in_flight_job() {
        let (started_tx, started_rx) = channel();
        let (release_tx, release_rx) = channel();
        let (completed_tx, completed_rx) = channel();
        let worker = PlannerWorker::spawn(
            BlockingTransport {
                started: started_tx,
                release: release_rx,
            },
            move |completion| completed_tx.send(completion).unwrap(),
        )
        .unwrap();

        worker.try_submit(job(1)).unwrap();
        started_rx.recv().unwrap();
        assert_eq!(worker.try_submit(job(2)), Err(PlannerError::Busy));
        release_tx.send(()).unwrap();
        assert_eq!(
            completed_rx.recv().unwrap().result,
            Ok(command_decision("/tetris/piece rotate"))
        );
    }

    #[test]
    fn chat_request_is_non_streaming_and_constrained_to_authorized_invocations() {
        let request = build_chat_request("model", &job(1));

        assert_eq!(request["model"], "model");
        assert_eq!(request["stream"], false);
        assert_eq!(request["thinking"]["type"], "disabled");
        assert_eq!(request["response_format"]["type"], "json_object");
        assert_eq!(request["temperature"], 0);
        assert_eq!(request["max_tokens"], 128 * 1024);
        assert!(request.get("format").is_none());
        assert!(
            request["messages"][0]["content"]
                .as_str()
                .unwrap()
                .contains("/tetris/piece rotate")
        );
        assert!(
            request["messages"][1]["content"]
                .as_str()
                .unwrap()
                .contains(r#"{"active":{"kind":"T"}}"#)
        );
        assert!(
            request["messages"][1]["content"]
                .as_str()
                .unwrap()
                .contains("/tetris/game state")
        );
    }

    #[test]
    fn response_requires_nested_structured_json_and_enforces_limits() {
        let valid = r#"{"choices":[{"finish_reason":"stop","message":{"content":"{\"invocations\":[\"/tetris/piece rotate\",\"/tetris/piece left\"],\"message\":\"Rotating and moving.\"}"}}]}"#;
        assert_eq!(
            parse_chat_response(valid),
            Ok(PlannerDecision {
                invocations: vec!["/tetris/piece rotate".into(), "/tetris/piece left".into()],
                message: "Rotating and moving.".into(),
            })
        );
        let no_action = r#"{"choices":[{"finish_reason":"stop","message":{"content":"{\"invocations\":[],\"message\":\"你好！\"}"}}]}"#;
        assert_eq!(
            parse_chat_response(no_action),
            Ok(PlannerDecision {
                invocations: Vec::new(),
                message: "你好！".into(),
            })
        );
        assert_eq!(
            parse_chat_response(
                r#"{"choices":[{"finish_reason":"stop","message":{"content":"not json"}}]}"#
            ),
            Err(PlannerError::InvalidResponse)
        );
        assert_eq!(
            parse_chat_response(
                r#"{"choices":[{"finish_reason":"stop","message":{"content":"{\"invocations\":[\"\"],\"message\":\"bad\"}"}}]}"#
            ),
            Err(PlannerError::EmptyInvocation)
        );

        let oversized = "x".repeat(257);
        let body = serde_json::json!({
            "choices": [{ "finish_reason": "stop", "message": {
                "content": serde_json::json!({
                    "invocations": [oversized],
                    "message": "too large"
                }).to_string()
            }}]
        });
        assert_eq!(
            parse_chat_response(&body.to_string()),
            Err(PlannerError::InvocationTooLarge)
        );

        let too_many = serde_json::json!({
            "choices": [{ "finish_reason": "stop", "message": {
                "content": serde_json::json!({
                    "invocations": vec!["/tetris/piece left"; 17],
                    "message": "too many"
                }).to_string()
            }}]
        });
        assert_eq!(
            parse_chat_response(&too_many.to_string()),
            Err(PlannerError::PlanTooLong)
        );

        assert_eq!(
            parse_chat_response(
                r#"{"choices":[{"finish_reason":"length","message":{"content":""}}]}"#
            ),
            Err(PlannerError::ResponseTruncated)
        );
        assert_eq!(
            parse_chat_response(
                r#"{"choices":[{"finish_reason":"stop","message":{"content":""}}]}"#
            ),
            Err(PlannerError::EmptyResponse)
        );
    }

    #[test]
    fn missing_api_key_is_reported_before_network_access() {
        let transport = DeepSeekTransport::new(DeepSeekConfig {
            endpoint: "https://example.invalid/chat/completions".into(),
            model: "model".into(),
            api_key: None,
            connect_timeout: std::time::Duration::from_millis(1),
            request_timeout: std::time::Duration::from_millis(1),
        });

        assert_eq!(transport.plan(&job(1)), Err(PlannerError::MissingApiKey));
    }

    #[test]
    fn empty_prompt_is_rejected_before_a_request_id_is_allocated() {
        let mut session = PlannerSession::default();

        assert_eq!(
            session.begin("   ", "{}".into(), Vec::new()),
            Err(PlannerError::EmptyPrompt)
        );
        assert_eq!(session.view(), PlannerView::Idle);
    }

    #[test]
    #[ignore = "requires PUNCTUM_DEEPSEEK_API_KEY and remote API access"]
    fn remote_deepseek_returns_an_authorized_structured_invocation() {
        let allowed_invocations = vec![
            "/tetris/game restart".into(),
            "/tetris/piece hard-drop".into(),
            "/tetris/piece left".into(),
            "/tetris/piece right".into(),
            "/tetris/piece rotate".into(),
            "/tetris/piece soft-drop".into(),
        ];
        let job = PlannerJob {
            id: RequestId(1),
            prompt: "If the active piece rotation is spawn, rotate it; otherwise hard-drop it."
                .into(),
            context: r#"{"active":{"kind":"T","rotation":"spawn"}}"#.into(),
            allowed_invocations: allowed_invocations.clone(),
        };
        let transport = DeepSeekTransport::new(DeepSeekConfig::from_env());

        let decision = transport
            .plan(&job)
            .expect("remote DeepSeek planner response");

        assert!(!decision.invocations.is_empty());
        assert!(
            decision
                .invocations
                .iter()
                .all(|invocation| allowed_invocations.contains(invocation))
        );

        let greeting = PlannerJob {
            id: RequestId(2),
            prompt: "你好".into(),
            context: r#"{"active":{"kind":"T","rotation":"spawn"}}"#.into(),
            allowed_invocations,
        };
        let reply = transport
            .plan(&greeting)
            .expect("remote DeepSeek greeting response");
        assert!(reply.invocations.is_empty());
        assert!(!reply.message.is_empty());
    }

    #[test]
    fn ramus_observation_is_serialized_as_stable_json() {
        let observation = Value::Record(std::collections::BTreeMap::from([
            ("game_over".into(), Value::Boolean(false)),
            ("lines".into(), Value::Integer(3)),
            (
                "rows".into(),
                Value::List(vec![Value::String("....TT....".into())]),
            ),
        ]));

        assert_eq!(
            format_observation(&observation),
            r#"{"game_over":false,"lines":3,"rows":["....TT...."]}"#
        );
    }
}
