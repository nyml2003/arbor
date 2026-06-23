use battle_core::BattleAction;
use battle_data::ElementType;

use crate::snapshot::{ActionKind, BattleSnapshot, ViewerProfile};
use crate::text::BattleViewText;
use crate::EventTab;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicBattleView<L: BattleViewText> {
    pub locale: L,
    pub battle_id: String,
    pub viewer: ViewerProfile,
    pub mode: String,
    pub turn: u16,
    pub request_label: String,
    pub weather_line: String,
    pub seed: u64,
    pub player: SidePanelView,
    pub opponent: SidePanelView,
    pub legal_actions: Vec<ActionView>,
    pub recent_events: Vec<String>,
    pub selected_tab: EventTab,
    pub agent_summary: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SidePanelView {
    pub side_label: String,
    pub active_name: String,
    pub species_name: String,
    pub primary_type: ElementType,
    pub secondary_type: Option<ElementType>,
    pub type_line: String,
    pub current_hp: i32,
    pub max_hp: i32,
    pub hp_line: String,
    pub status_line: String,
    pub bench_line: String,
    pub latest_summary: String,
    pub is_waiting: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionView {
    pub index: usize,
    pub hotkey: String,
    pub token: String,
    pub label: String,
    pub kind: String,
    pub element: Option<String>,
    pub power: Option<u16>,
    pub action: BattleAction,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UiEventLog {
    pub domain: Vec<String>,
    pub trace: Vec<String>,
    pub system: Vec<String>,
}

impl UiEventLog {
    pub fn selected_lines(&self, tab: EventTab, limit: usize) -> Vec<String> {
        let source = match tab {
            EventTab::Domain => &self.domain,
            EventTab::Trace => &self.trace,
            EventTab::System => &self.system,
        };

        let start = source.len().saturating_sub(limit);
        source[start..].to_vec()
    }
}

pub fn build_public_battle_view<L: BattleViewText>(
    snapshot: &BattleSnapshot,
    locale: L,
    log: &UiEventLog,
    selected_tab: EventTab,
    latest_agent_summary: &str,
) -> PublicBattleView<L> {
    PublicBattleView {
        locale,
        battle_id: snapshot.battle_id.clone(),
        viewer: snapshot.viewer,
        mode: locale.mode_human_vs_ai().to_string(),
        turn: snapshot.turn,
        request_label: locale.request_label(snapshot.request),
        weather_line: locale.weather_line(snapshot.weather),
        seed: snapshot.seed,
        player: side_panel_view(&snapshot.player, locale),
        opponent: side_panel_view(&snapshot.opponent, locale),
        legal_actions: snapshot
            .legal_actions
            .iter()
            .map(|action| action_view(action, snapshot.viewer, locale))
            .collect(),
        recent_events: log.selected_lines(selected_tab, 12),
        selected_tab,
        agent_summary: vec![
            locale.agent_name_line("simple-rule-ai"),
            latest_agent_summary.to_string(),
            locale.fallback_none().to_string(),
        ],
    }
}

fn side_panel_view<L: BattleViewText>(
    snapshot: &crate::snapshot::SideSnapshot,
    locale: L,
) -> SidePanelView {
    SidePanelView {
        side_label: locale.side_label(snapshot.side).to_string(),
        active_name: snapshot.active_name.clone(),
        species_name: snapshot.species_name.clone(),
        primary_type: snapshot.primary_type,
        secondary_type: snapshot.secondary_type,
        type_line: locale.pokemon_type_line(snapshot.primary_type, snapshot.secondary_type),
        current_hp: snapshot.current_hp,
        max_hp: snapshot.max_hp,
        hp_line: locale.hp_line(snapshot.current_hp, snapshot.max_hp),
        status_line: locale.status_line(snapshot.status),
        bench_line: locale.alive_line(snapshot.alive_count, snapshot.party_size),
        latest_summary: locale.latest_side_summary(snapshot.side),
        is_waiting: snapshot.is_waiting,
    }
}

fn action_view<L: BattleViewText>(
    descriptor: &crate::snapshot::ActionDescriptor,
    viewer: ViewerProfile,
    locale: L,
) -> ActionView {
    let (hotkey, label, kind, element) = match viewer {
        ViewerProfile::LocalPlayer(_) => {
            let label = match descriptor.kind {
                ActionKind::Move => descriptor.name.clone(),
                ActionKind::Switch => locale.action_switch(&descriptor.name),
            };
            let kind = match descriptor.kind {
                ActionKind::Move => locale.action_kind_move().to_string(),
                ActionKind::Switch => locale.action_kind_switch().to_string(),
            };
            let element = descriptor
                .element_type
                .map(|element| locale.element_type_name(element).to_string());
            ((descriptor.index + 1).to_string(), label, kind, element)
        }
        ViewerProfile::Spectator => {
            let kind = match descriptor.kind {
                ActionKind::Move => locale.action_kind_move().to_string(),
                ActionKind::Switch => locale.action_kind_switch().to_string(),
            };
            let element = descriptor
                .element_type
                .map(|element| locale.element_type_name(element).to_string());
            (String::new(), descriptor.name.clone(), kind, element)
        }
        ViewerProfile::Agent(_) | ViewerProfile::Debug => {
            let kind = match descriptor.kind {
                ActionKind::Move => "move".to_string(),
                ActionKind::Switch => "switch".to_string(),
            };
            (String::new(), descriptor.name.clone(), kind, None)
        }
    };

    ActionView {
        index: descriptor.index,
        hotkey,
        token: descriptor.token.clone(),
        label,
        kind,
        element,
        power: descriptor.power,
        action: descriptor.action,
    }
}
