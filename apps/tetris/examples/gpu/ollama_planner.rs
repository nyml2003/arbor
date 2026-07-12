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

use serde::Deserialize;
use serde_json::{Value, json};

const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:11434/api/chat";
const DEFAULT_MODEL: &str = "qwen3:14b";
const MAX_RESPONSE_BYTES: u64 = 64 * 1024;
const MAX_INVOCATION_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestId(u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannerJob {
    pub id: RequestId,
    pub prompt: String,
    pub allowed_invocations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannerCompletion {
    pub id: RequestId,
    pub result: Result<String, PlannerError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlannerError {
    EmptyPrompt,
    Busy,
    WorkerStopped,
    HttpStatus(u16),
    Timeout,
    Unavailable,
    InvalidResponse,
    EmptyInvocation,
    InvocationTooLarge,
}

impl PlannerError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::EmptyPrompt => "empty-prompt",
            Self::Busy => "busy",
            Self::WorkerStopped => "worker-stopped",
            Self::HttpStatus(_) => "http-status",
            Self::Timeout => "timeout",
            Self::Unavailable => "unavailable",
            Self::InvalidResponse => "invalid-response",
            Self::EmptyInvocation => "empty-invocation",
            Self::InvocationTooLarge => "invocation-too-large",
        }
    }
}

impl fmt::Display for PlannerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPrompt => formatter.write_str("enter a natural-language command first"),
            Self::Busy => formatter.write_str("an Ollama request is already running"),
            Self::WorkerStopped => formatter.write_str("the Ollama worker has stopped"),
            Self::HttpStatus(status) => write!(formatter, "Ollama returned HTTP {status}"),
            Self::Timeout => formatter.write_str("the Ollama request timed out"),
            Self::Unavailable => formatter.write_str("Ollama is unavailable"),
            Self::InvalidResponse => formatter.write_str("Ollama returned an invalid response"),
            Self::EmptyInvocation => formatter.write_str("Ollama returned an empty invocation"),
            Self::InvocationTooLarge => {
                formatter.write_str("Ollama returned an invocation larger than 256 bytes")
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
    Failed(&'a str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompletionDisposition {
    Ignored,
    Candidate(String),
    Failed,
}

#[derive(Debug, Default)]
pub struct PlannerSession {
    next_id: u64,
    active: Option<ActiveRequest>,
    failure: Option<PlannerFailure>,
}

impl PlannerSession {
    pub fn begin(
        &mut self,
        prompt: &str,
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
        self.failure = None;
        Ok(PlannerJob {
            id,
            prompt: prompt.to_owned(),
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
            Ok(candidate) => CompletionDisposition::Candidate(candidate),
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
        self.failure = None;
    }

    pub fn clear_failure(&mut self) {
        self.failure = None;
    }

    pub fn record_failure(&mut self, code: &str, message: &str) {
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
        } else {
            PlannerView::Idle
        }
    }
}

pub trait PlannerTransport: Send + 'static {
    fn plan(&self, job: &PlannerJob) -> Result<String, PlannerError>;
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
            .name("tetris-ollama-planner".into())
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OllamaConfig {
    pub endpoint: String,
    pub model: String,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
}

impl OllamaConfig {
    pub fn from_env() -> Self {
        Self {
            endpoint: env::var("PUNCTUM_OLLAMA_URL").unwrap_or_else(|_| DEFAULT_ENDPOINT.into()),
            model: env::var("PUNCTUM_OLLAMA_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.into()),
            connect_timeout: Duration::from_secs(3),
            request_timeout: Duration::from_secs(120),
        }
    }
}

pub struct OllamaTransport {
    config: OllamaConfig,
    agent: ureq::Agent,
}

impl OllamaTransport {
    pub fn new(config: OllamaConfig) -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_connect(Some(config.connect_timeout))
            .timeout_global(Some(config.request_timeout))
            .build()
            .into();
        Self { config, agent }
    }
}

impl PlannerTransport for OllamaTransport {
    fn plan(&self, job: &PlannerJob) -> Result<String, PlannerError> {
        let request = build_chat_request(&self.config.model, job);
        let mut response = self
            .agent
            .post(&self.config.endpoint)
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

fn build_chat_request(model: &str, job: &PlannerJob) -> Value {
    let allowed = job
        .allowed_invocations
        .iter()
        .map(|invocation| format!("- {invocation}"))
        .collect::<Vec<_>>()
        .join("\n");
    let schema = json!({
        "type": "object",
        "properties": {
            "invocation": {
                "type": "string",
                "enum": job.allowed_invocations
            }
        },
        "required": ["invocation"],
        "additionalProperties": false
    });
    let system = format!(
        "Choose exactly one authorized Tetris invocation for the user's request. \
Return JSON matching this schema: {schema}. The invocation must be copied exactly from this list:\n{allowed}"
    );

    json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": job.prompt }
        ],
        "stream": false,
        "think": false,
        "format": schema,
        "options": {
            "temperature": 0,
            "num_predict": 64
        }
    })
}

#[derive(Deserialize)]
struct ChatResponse {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: String,
}

#[derive(Deserialize)]
struct PlannerOutput {
    invocation: String,
}

fn parse_chat_response(body: &str) -> Result<String, PlannerError> {
    let response: ChatResponse =
        serde_json::from_str(body).map_err(|_| PlannerError::InvalidResponse)?;
    let output: PlannerOutput = serde_json::from_str(&response.message.content)
        .map_err(|_| PlannerError::InvalidResponse)?;
    let invocation = output.invocation.trim();
    if invocation.is_empty() {
        return Err(PlannerError::EmptyInvocation);
    }
    if invocation.len() > MAX_INVOCATION_BYTES {
        return Err(PlannerError::InvocationTooLarge);
    }
    Ok(invocation.to_owned())
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

    use super::{
        CompletionDisposition, OllamaConfig, OllamaTransport, PlannerCompletion, PlannerError,
        PlannerJob, PlannerSession, PlannerTransport, PlannerView, PlannerWorker, RequestId,
        build_chat_request, parse_chat_response,
    };

    struct BlockingTransport {
        started: Sender<()>,
        release: Receiver<()>,
    }

    impl PlannerTransport for BlockingTransport {
        fn plan(&self, _job: &PlannerJob) -> Result<String, PlannerError> {
            self.started.send(()).unwrap();
            self.release.recv().unwrap();
            Ok("/tetris/piece rotate".into())
        }
    }

    fn job(id: u64) -> PlannerJob {
        PlannerJob {
            id: RequestId(id),
            prompt: "rotate the piece".into(),
            allowed_invocations: vec!["/tetris/piece rotate".into()],
        }
    }

    #[test]
    fn session_tracks_pending_detached_and_duplicate_completions() {
        let mut session = PlannerSession::default();
        let request = session
            .begin(" rotate the piece ", vec!["/tetris/piece rotate".into()])
            .unwrap();
        assert_eq!(request.prompt, "rotate the piece");
        assert_eq!(session.view(), PlannerView::Pending);
        assert_eq!(session.begin("again", Vec::new()), Err(PlannerError::Busy));

        session.detach();
        assert_eq!(session.view(), PlannerView::Idle);
        let completion = PlannerCompletion {
            id: request.id,
            result: Ok("/tetris/piece rotate".into()),
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
        let request = session.begin("rotate", Vec::new()).unwrap();

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
            Ok("/tetris/piece rotate".into())
        );
    }

    #[test]
    fn chat_request_is_non_streaming_and_constrained_to_authorized_invocations() {
        let request = build_chat_request("model", &job(1));

        assert_eq!(request["model"], "model");
        assert_eq!(request["stream"], false);
        assert_eq!(request["think"], false);
        assert_eq!(request["format"]["required"][0], "invocation");
        assert_eq!(
            request["format"]["properties"]["invocation"]["enum"][0],
            "/tetris/piece rotate"
        );
        assert!(
            request["messages"][0]["content"]
                .as_str()
                .unwrap()
                .contains("/tetris/piece rotate")
        );
    }

    #[test]
    fn response_requires_nested_structured_json_and_enforces_limits() {
        let valid = r#"{"message":{"content":"{\"invocation\":\"/tetris/piece rotate\"}"}}"#;
        assert_eq!(
            parse_chat_response(valid),
            Ok("/tetris/piece rotate".into())
        );
        assert_eq!(
            parse_chat_response(r#"{"message":{"content":"not json"}}"#),
            Err(PlannerError::InvalidResponse)
        );
        assert_eq!(
            parse_chat_response(r#"{"message":{"content":"{\"invocation\":\"\"}"}}"#),
            Err(PlannerError::EmptyInvocation)
        );

        let oversized = "x".repeat(257);
        let body = serde_json::json!({
            "message": {
                "content": serde_json::json!({ "invocation": oversized }).to_string()
            }
        });
        assert_eq!(
            parse_chat_response(&body.to_string()),
            Err(PlannerError::InvocationTooLarge)
        );
    }

    #[test]
    fn empty_prompt_is_rejected_before_a_request_id_is_allocated() {
        let mut session = PlannerSession::default();

        assert_eq!(
            session.begin("   ", Vec::new()),
            Err(PlannerError::EmptyPrompt)
        );
        assert_eq!(session.view(), PlannerView::Idle);
    }

    #[test]
    #[ignore = "requires a local Ollama server and configured model"]
    fn local_ollama_returns_an_authorized_structured_invocation() {
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
            prompt: "Rotate the current Tetris piece clockwise.".into(),
            allowed_invocations: allowed_invocations.clone(),
        };
        let transport = OllamaTransport::new(OllamaConfig::from_env());

        let invocation = transport.plan(&job).expect("local Ollama planner response");

        assert_eq!(invocation, "/tetris/piece rotate");
        assert!(allowed_invocations.contains(&invocation));
    }
}
