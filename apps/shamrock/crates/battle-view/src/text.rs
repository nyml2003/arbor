use battle_core::{Request, SideId};
use battle_data::{ElementType, StatusCondition, WeatherKind};

use crate::EventTab;

pub trait BattleViewText: Copy + Eq + PartialEq {
    fn event_tab_title(self, tab: EventTab) -> &'static str;
    fn mode_human_vs_ai(self) -> &'static str;
    fn weather_line(self, weather: Option<(WeatherKind, u8)>) -> String;
    fn side_label(self, side: SideId) -> &'static str;
    fn pokemon_type_line(self, primary: ElementType, secondary: Option<ElementType>) -> String;
    fn hp_line(self, current_hp: i32, max_hp: i32) -> String;
    fn status_line(self, status: Option<StatusCondition>) -> String;
    fn alive_line(self, alive_count: usize, party_size: usize) -> String;
    fn latest_side_summary(self, side: SideId) -> String;
    fn request_label(self, request: Request) -> String;
    fn action_kind_move(self) -> &'static str;
    fn action_kind_switch(self) -> &'static str;
    fn action_switch(self, nickname: &str) -> String;
    fn element_type_name(self, element_type: ElementType) -> &'static str;
    fn agent_name_line(self, agent_name: &str) -> String;
    fn fallback_none(self) -> &'static str;
}
