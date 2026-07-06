use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use arbor_tui::prelude::*;
use arbor_tui::testing::HeadlessApp;
use aster_application::{
    ChatRequestOptions, ChatStreamError, ChatStreamPort, StreamEvent, StreamReceiver,
};
use aster_domain::ChatMessage;

use super::log::{meta_from_state, BenchBudget, BenchProfile, BenchSummary, FrameLog, SceneTag};
use crate::runner::{before_events, before_render, update, view, AsterAction, AsterState};

const DEFAULT_COLS: u16 = 100;
const DEFAULT_ROWS: u16 = 32;
const STREAM_CHUNK_SIZE: usize = 18;

pub(crate) fn should_run_bench() -> bool {
    std::env::args().any(|arg| arg == "--bench")
}

pub(crate) fn run_from_env() -> Result<()> {
    let config = BenchConfig::from_args(std::env::args().skip(1));
    let out_path = config.output_path();
    let client = ScriptedClient::default();
    let driver = BenchmarkDriver::new(
        client,
        config.loops,
        config.profile,
        config.budget,
        config.cache_shadow,
    );
    let outcome = driver.run_all(&out_path)?;
    eprintln!("{}", outcome.report);
    eprintln!("Bench finished, log saved to {}", out_path.display());
    if !config.no_fail && !outcome.verdict.passed {
        anyhow::bail!(
            "bench budget failed: {}",
            outcome.verdict.failures.join("; ")
        );
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct BenchConfig {
    output: Option<PathBuf>,
    loops: usize,
    profile: BenchProfile,
    budget: BenchBudget,
    no_fail: bool,
    cache_shadow: bool,
}

impl BenchConfig {
    fn from_args(args: impl IntoIterator<Item = String>) -> Self {
        let mut output = None;
        let mut loops = 1usize;
        let mut profile = BenchProfile::HeadlessStrict;
        let mut budget = BenchBudget::for_profile(profile);
        let mut no_fail = false;
        let mut cache_shadow = false;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--bench" => {}
                "--bench-no-fail" => {
                    no_fail = true;
                }
                "--bench-cache-shadow" => {
                    cache_shadow = true;
                }
                "--bench-out" => {
                    if let Some(value) = args.next() {
                        output = Some(PathBuf::from(value));
                    }
                }
                "--bench-loops" => {
                    if let Some(value) = args.next() {
                        loops = value.parse().unwrap_or(1).max(1);
                    }
                }
                "--bench-profile" => {
                    if let Some(value) = args.next() {
                        if let Some(next_profile) = BenchProfile::from_name(&value) {
                            profile = next_profile;
                            budget = BenchBudget::for_profile(profile);
                        }
                    }
                }
                "--bench-max-p95-ms" => {
                    if let Some(value) = args.next() {
                        budget.max_p95_ms = value.parse().unwrap_or(budget.max_p95_ms);
                    }
                }
                "--bench-max-p99-ms" => {
                    if let Some(value) = args.next() {
                        budget.max_p99_ms = value.parse().unwrap_or(budget.max_p99_ms);
                    }
                }
                "--bench-max-c-pct" => {
                    if let Some(value) = args.next() {
                        budget.max_c_frame_pct = value.parse().unwrap_or(budget.max_c_frame_pct);
                    }
                }
                _ => {}
            }
        }
        Self {
            output,
            loops,
            profile,
            budget,
            no_fail,
            cache_shadow,
        }
    }

    fn output_path(&self) -> PathBuf {
        self.output.clone().unwrap_or_else(default_output_path)
    }
}

pub(crate) struct BenchmarkDriver {
    app: HeadlessApp<AsterState, AsterAction>,
    client: ScriptedClient,
    seq: u32,
    logs: Vec<FrameLog>,
    loops: usize,
    profile: BenchProfile,
    budget: BenchBudget,
}

impl BenchmarkDriver {
    fn new(
        client: ScriptedClient,
        loops: usize,
        profile: BenchProfile,
        budget: BenchBudget,
        cache_shadow: bool,
    ) -> Self {
        let state = AsterState::with_model(client.clone(), "deepseek-chat");
        let app = HeadlessApp::new(state, update, view, DEFAULT_COLS, DEFAULT_ROWS)
            .theme(Theme::dark())
            .cache_shadow(cache_shadow)
            .before_events(before_events)
            .before_render(before_render);
        Self {
            app,
            client,
            seq: 0,
            logs: Vec::new(),
            loops,
            profile,
            budget,
        }
    }

    pub(crate) fn run_all(mut self, out_path: &Path) -> Result<BenchOutcome> {
        for _ in 0..self.loops {
            for step in standard_chat_sequence() {
                self.run_step(step)?;
            }
        }

        let summary = BenchSummary::new(self.logs.clone(), self.profile, self.budget);
        let report = summary.report();
        let verdict = summary.verdict();
        self.write_log_file(out_path, &report)?;
        Ok(BenchOutcome { report, verdict })
    }

    fn run_step(&mut self, step: BenchStep) -> Result<()> {
        match step {
            BenchStep::IdleFrames(frames) => {
                for _ in 0..frames {
                    self.run_frame(SceneTag::Idle, [], 0)?;
                }
            }
            BenchStep::InputText(text) => {
                for ch in text.chars() {
                    self.run_frame(SceneTag::Idle, [KeyEvent::char(ch)], 0)?;
                }
            }
            BenchStep::SetDraft(scene, text) => {
                self.app.dispatch(AsterAction::DraftChanged(text));
                self.run_frame(scene, [], 0)?;
            }
            BenchStep::SubmitDraft(scene, text) => {
                self.app.dispatch(AsterAction::SubmitInput(text));
                self.run_frame(scene, [], 0)?;
            }
            BenchStep::KeyPress(scene, key) => {
                self.run_frame(scene, [key_event(key)], 0)?;
            }
            BenchStep::Scroll(scene, key, frames) => {
                for _ in 0..frames {
                    self.run_frame(scene, [key_event(key.clone())], 0)?;
                }
            }
            BenchStep::StreamText(text, interrupt_after) => {
                let chunks = chunk_text(&text, STREAM_CHUNK_SIZE);
                for (index, chunk) in chunks.into_iter().enumerate() {
                    self.client.push(StreamEvent::Token(chunk));
                    self.run_frame(SceneTag::Streaming, [], 1)?;
                    if interrupt_after.is_some_and(|limit| index + 1 >= limit) {
                        break;
                    }
                }
                if interrupt_after.is_none() {
                    self.client.push(StreamEvent::Done);
                    self.run_frame(SceneTag::Streaming, [], 0)?;
                }
            }
        }
        Ok(())
    }

    fn run_frame(
        &mut self,
        scene: SceneTag,
        events: impl IntoIterator<Item = KeyEvent>,
        new_tokens: usize,
    ) -> Result<()> {
        let stats = self.app.tick(events)?;
        let line_count = self.app.state().line_count(&Theme::dark());
        let chat = self.app.state().app.chat();
        let meta = meta_from_state(
            line_count,
            new_tokens,
            chat.state(),
            chat.messages().len(),
            &stats,
        );
        self.logs.push(FrameLog::from_stats(
            self.seq,
            scene,
            &stats,
            meta,
            self.profile,
        ));
        self.seq = self.seq.saturating_add(1);
        Ok(())
    }

    fn write_log_file(&self, out_path: &Path, report: &str) -> Result<()> {
        if let Some(parent) = out_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create bench output dir {}", parent.display())
            })?;
        }

        let mut output = String::new();
        for frame in &self.logs {
            output.push_str(&serde_json::to_string(frame)?);
            output.push('\n');
        }
        output.push('\n');
        output.push_str(report);
        fs::write(out_path, output)
            .with_context(|| format!("failed to write bench log {}", out_path.display()))
    }
}

pub(crate) struct BenchOutcome {
    report: String,
    verdict: super::log::BenchVerdict,
}

#[derive(Clone, Debug)]
enum BenchStep {
    IdleFrames(u32),
    InputText(String),
    SetDraft(SceneTag, String),
    SubmitDraft(SceneTag, String),
    KeyPress(SceneTag, Key),
    Scroll(SceneTag, Key, u32),
    StreamText(String, Option<usize>),
}

fn standard_chat_sequence() -> Vec<BenchStep> {
    vec![
        BenchStep::IdleFrames(2),
        BenchStep::InputText("Hello".to_string()),
        BenchStep::KeyPress(SceneTag::Idle, Key::Enter),
        BenchStep::StreamText(short_response(), None),
        BenchStep::KeyPress(SceneTag::Scrolling, Key::End),
        BenchStep::KeyPress(SceneTag::Scrolling, Key::Home),
        BenchStep::Scroll(SceneTag::Scrolling, Key::PageDown, 5),
        BenchStep::Scroll(SceneTag::Scrolling, Key::PageUp, 5),
        BenchStep::InputText("/th".to_string()),
        BenchStep::Scroll(SceneTag::PaletteOpen, Key::ArrowDown, 2),
        BenchStep::Scroll(SceneTag::PaletteOpen, Key::ArrowUp, 1),
        BenchStep::KeyPress(SceneTag::PaletteOpen, Key::Enter),
        BenchStep::InputText("light".to_string()),
        BenchStep::KeyPress(SceneTag::PaletteOpen, Key::Enter),
        BenchStep::SubmitDraft(SceneTag::PaletteOpen, "/theme light".to_string()),
        BenchStep::SetDraft(SceneTag::Idle, long_prompt().clone()),
        BenchStep::SubmitDraft(SceneTag::Streaming, long_prompt()),
        BenchStep::StreamText(long_response(), Some(48)),
        BenchStep::KeyPress(SceneTag::Streaming, Key::Escape),
        BenchStep::SubmitDraft(
            SceneTag::ModelSwitch,
            "/model deepseek-reasoner".to_string(),
        ),
        BenchStep::Scroll(SceneTag::Scrolling, Key::End, 1),
        BenchStep::Scroll(SceneTag::Scrolling, Key::PageUp, 12),
        BenchStep::Scroll(SceneTag::Scrolling, Key::PageDown, 12),
        BenchStep::KeyPress(SceneTag::Exit, Key::Escape),
    ]
}

#[derive(Clone, Default)]
pub(crate) struct ScriptedClient {
    active_tx: Rc<RefCell<Option<mpsc::Sender<StreamEvent>>>>,
}

impl ScriptedClient {
    fn push(&self, event: StreamEvent) {
        let Some(tx) = self.active_tx.borrow().as_ref().cloned() else {
            return;
        };
        let _ = tx.send(event);
    }
}

impl ChatStreamPort for ScriptedClient {
    fn start_stream(
        &self,
        _messages: &[ChatMessage],
        _options: &ChatRequestOptions,
    ) -> Result<StreamReceiver, ChatStreamError> {
        let (tx, rx) = mpsc::channel();
        *self.active_tx.borrow_mut() = Some(tx);
        Ok(StreamReceiver::new(rx))
    }
}

fn key_event(key: Key) -> KeyEvent {
    KeyEvent {
        key,
        modifiers: Modifiers::default(),
        kind: KeyEventKind::Press,
    }
}

fn chunk_text(text: &str, chunk_size: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if current.len() >= chunk_size {
            chunks.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn default_output_path() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    PathBuf::from(format!("perf_bench_{timestamp}.jsonl"))
}

fn short_response() -> String {
    "Hello. This is a short streaming reply used to warm the transcript and first response path."
        .to_string()
}

fn long_prompt() -> String {
    let sentence = "Explain the current design, expected bottlenecks, scroll behavior, command palette behavior, and frame pacing strategy for a terminal chat application. ";
    sentence.repeat(16)
}

fn long_response() -> String {
    let paragraph = "The benchmark keeps rendering pressure realistic by growing transcript content while streaming tokens arrive. It exercises layout, transcript wrapping, footer updates, scroll clamping, palette rebuilds, and dirty region emission. ";
    paragraph.repeat(72)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_sequence_covers_required_scenes() {
        let script = standard_chat_sequence();

        assert!(script
            .iter()
            .any(|step| matches!(step, BenchStep::StreamText(_, _))));
        assert!(script
            .iter()
            .any(|step| { matches!(step, BenchStep::Scroll(SceneTag::Scrolling, _, _)) }));
        assert!(script
            .iter()
            .any(|step| { matches!(step, BenchStep::SubmitDraft(SceneTag::ModelSwitch, _)) }));
    }

    #[test]
    fn driver_writes_jsonl_and_summary() {
        let out =
            std::env::temp_dir().join(format!("aster_bench_test_{}.jsonl", std::process::id()));
        let driver = BenchmarkDriver::new(
            ScriptedClient::default(),
            1,
            BenchProfile::HeadlessStrict,
            BenchBudget::for_profile(BenchProfile::HeadlessStrict),
            false,
        );

        let outcome = driver.run_all(&out).unwrap();
        let content = fs::read_to_string(&out).unwrap();
        let _ = fs::remove_file(&out);

        assert!(content.lines().next().unwrap().starts_with('{'));
        assert!(content.contains("\"watermark\""));
        assert!(content.contains("=== Bench Summary ==="));
        assert!(outcome.report.contains("Total frames:"));
        assert!(outcome.verdict.passed);
    }

    #[test]
    fn config_supports_profile_and_budget_overrides() {
        let config = BenchConfig::from_args([
            "--bench-profile".to_string(),
            "interactive".to_string(),
            "--bench-max-p95-ms".to_string(),
            "3.5".to_string(),
            "--bench-max-p99-ms".to_string(),
            "4.5".to_string(),
            "--bench-max-c-pct".to_string(),
            "7".to_string(),
            "--bench-no-fail".to_string(),
            "--bench-cache-shadow".to_string(),
        ]);

        assert_eq!(config.profile, BenchProfile::Interactive);
        assert_eq!(config.budget.max_p95_ms, 3.5);
        assert_eq!(config.budget.max_p99_ms, 4.5);
        assert_eq!(config.budget.max_c_frame_pct, 7);
        assert!(config.no_fail);
        assert!(config.cache_shadow);
    }
}
