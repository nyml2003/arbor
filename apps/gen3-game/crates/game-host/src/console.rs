use battle_application::Action;
use battle_ramus_adapter::{ActionInvocation, ActionQueue, AdapterDiagnostic, BattleRamusAdapter};
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConsoleIntent {
    Open(Vec<Action>),
    Close,
    InsertText(String),
    Backspace,
    Next,
    Previous,
    Execute,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsoleOutcome {
    Updated,
    Closed,
    ActionQueued,
    NoSelection,
    Failed,
    Ignored,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConsoleState {
    open: bool,
    query: String,
    available: Vec<ActionInvocation>,
    items: Vec<ActionInvocation>,
    selected_index: Option<usize>,
    diagnostic: Option<String>,
}

impl ConsoleState {
    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn items(&self) -> &[ActionInvocation] {
        &self.items
    }

    pub const fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
}

pub struct GameConsole {
    adapter: BattleRamusAdapter,
    queue: ActionQueue,
}

impl GameConsole {
    pub fn new() -> Self {
        let queue = ActionQueue::default();
        Self {
            adapter: BattleRamusAdapter::new(queue.clone()),
            queue,
        }
    }

    pub fn handle(&self, state: &mut ConsoleState, intent: ConsoleIntent) -> ConsoleOutcome {
        match intent {
            ConsoleIntent::Open(legal_actions) => {
                state.open = true;
                state.query.clear();
                state.available = self.adapter.action_invocations(&legal_actions);
                state.diagnostic = state
                    .available
                    .is_empty()
                    .then(|| "当前没有可用的战斗指令".into());
                refresh_items(state);
                ConsoleOutcome::Updated
            }
            ConsoleIntent::Close if state.open => {
                state.open = false;
                state.diagnostic = None;
                ConsoleOutcome::Closed
            }
            ConsoleIntent::Close => ConsoleOutcome::Ignored,
            ConsoleIntent::InsertText(text) if state.open => {
                state.query.push_str(&text);
                state.diagnostic = None;
                refresh_items(state);
                ConsoleOutcome::Updated
            }
            ConsoleIntent::Backspace if state.open => {
                state.query.pop();
                state.diagnostic = None;
                refresh_items(state);
                ConsoleOutcome::Updated
            }
            ConsoleIntent::Next if state.open => {
                state.selected_index = match (state.selected_index, state.items.len()) {
                    (_, 0) => None,
                    (Some(index), len) => Some((index + 1) % len),
                    (None, _) => Some(0),
                };
                ConsoleOutcome::Updated
            }
            ConsoleIntent::Previous if state.open => {
                state.selected_index = match (state.selected_index, state.items.len()) {
                    (_, 0) => None,
                    (Some(0), len) | (None, len) => Some(len - 1),
                    (Some(index), _) => Some(index - 1),
                };
                ConsoleOutcome::Updated
            }
            ConsoleIntent::Execute if state.open => self.execute_selected(state),
            _ => ConsoleOutcome::Ignored,
        }
    }

    pub fn take_queued_action(&self) -> Option<Action> {
        self.queue.lock().ok()?.pop_front()
    }

    pub fn execution_succeeded(&self, state: &mut ConsoleState) {
        state.open = false;
        state.diagnostic = None;
    }

    pub fn execution_failed(&self, state: &mut ConsoleState, message: impl Into<String>) {
        state.diagnostic = Some(message.into());
    }

    fn execute_selected(&self, state: &mut ConsoleState) -> ConsoleOutcome {
        let Some(invocation) = state
            .selected_index
            .and_then(|index| state.items.get(index))
            .map(|item| item.invocation.clone())
        else {
            state.diagnostic = Some("没有匹配的战斗指令".into());
            return ConsoleOutcome::NoSelection;
        };

        match self.adapter.execute_invocation(&invocation) {
            Ok(()) => ConsoleOutcome::ActionQueued,
            Err(diagnostic) => {
                state.diagnostic = Some(format_diagnostic(&diagnostic));
                ConsoleOutcome::Failed
            }
        }
    }
}

impl Default for GameConsole {
    fn default() -> Self {
        Self::new()
    }
}

fn refresh_items(state: &mut ConsoleState) {
    let pattern = Pattern::new(
        &state.query,
        CaseMatching::Smart,
        Normalization::Smart,
        AtomKind::Fuzzy,
    );
    let mut matcher = Matcher::default();
    let mut utf32_buffer = Vec::new();
    let mut matches = state
        .available
        .iter()
        .cloned()
        .filter_map(|item| {
            pattern
                .score(
                    Utf32Str::new(item.invocation.as_str(), &mut utf32_buffer),
                    &mut matcher,
                )
                .map(|score| (item, score))
        })
        .collect::<Vec<_>>();
    matches.sort_by(|(left, left_score), (right, right_score)| {
        right_score
            .cmp(left_score)
            .then_with(|| left.invocation.cmp(&right.invocation))
    });
    state.items = matches.into_iter().map(|(item, _)| item).collect();
    state.selected_index = (!state.items.is_empty()).then_some(0);
}

fn format_diagnostic(diagnostic: &AdapterDiagnostic) -> String {
    format!("{}: {}", diagnostic.code, diagnostic.message)
}

#[cfg(test)]
mod tests {
    use battle_application::{Action, MoveSlot, TeamSlot};

    use super::{ConsoleIntent, ConsoleOutcome, ConsoleState, GameConsole};

    fn fixture() -> (GameConsole, ConsoleState) {
        (GameConsole::new(), ConsoleState::default())
    }

    #[test]
    fn fuzzy_filtering_navigation_and_zero_results_are_stable() {
        let (console, mut state) = fixture();
        let legal = vec![
            Action::UseMove(MoveSlot::new(0).unwrap()),
            Action::Switch(TeamSlot::new(1).unwrap()),
        ];
        console.handle(&mut state, ConsoleIntent::Open(legal));
        assert_eq!(state.items().len(), 2);

        console.handle(&mut state, ConsoleIntent::InsertText("teamtwo".into()));
        assert_eq!(state.items().len(), 1);
        assert_eq!(state.selected_index(), Some(0));

        console.handle(&mut state, ConsoleIntent::InsertText("zzz".into()));
        assert!(state.items().is_empty());
        assert_eq!(state.selected_index(), None);
        assert_eq!(
            console.handle(&mut state, ConsoleIntent::Execute),
            ConsoleOutcome::NoSelection
        );
    }

    #[test]
    fn selected_execution_queues_the_exact_action_and_waits_for_host_commit() {
        let (console, mut state) = fixture();
        let action = Action::UseMove(MoveSlot::new(2).unwrap());
        console.handle(&mut state, ConsoleIntent::Open(vec![action]));

        assert_eq!(
            console.handle(&mut state, ConsoleIntent::Execute),
            ConsoleOutcome::ActionQueued
        );
        assert!(state.open);
        assert_eq!(console.take_queued_action(), Some(action));

        console.execution_succeeded(&mut state);
        assert!(!state.open);
    }

    #[test]
    fn close_and_edit_intents_are_ignored_while_closed() {
        let (console, mut state) = fixture();
        assert_eq!(
            console.handle(&mut state, ConsoleIntent::Close),
            ConsoleOutcome::Ignored
        );
        assert_eq!(
            console.handle(&mut state, ConsoleIntent::InsertText("x".into())),
            ConsoleOutcome::Ignored
        );
    }

    #[test]
    fn opening_without_legal_actions_still_opens_with_an_explanation() {
        let (console, mut state) = fixture();

        assert_eq!(
            console.handle(&mut state, ConsoleIntent::Open(Vec::new())),
            ConsoleOutcome::Updated
        );
        assert!(state.open);
        assert_eq!(state.diagnostic(), Some("当前没有可用的战斗指令"));
    }
}
