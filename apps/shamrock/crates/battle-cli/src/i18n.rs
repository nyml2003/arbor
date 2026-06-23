use battle_core::SideId;
use battle_data::{StatId, StatusCondition};
use battle_data::{ElementType, WeatherKind};
use battle_view::{BattleViewText, EventTab};

/**
`Locale` 表示 CLI/TUI 外壳当前使用的界面语言。

现在只先支持中文和英文。
这里的 i18n 只负责 battle-cli 这层的界面文案，不负责数据包里的物种名和招式名。
*/
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Locale {
    ZhCn,
    EnUs,
}

impl Locale {
    /**
    根据环境变量推断当前界面语言。

    优先读取 `SHAMROCK_LANG`，这样用户可以明确覆盖。
    如果环境里没有可靠线索，就默认中文，因为当前项目的文档和主要使用语境都偏中文。
    */
    pub fn detect() -> Self {
        for key in ["SHAMROCK_LANG", "LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
            if let Some(value) = std::env::var_os(key) {
                if let Some(locale) = Self::from_tag(&value.to_string_lossy()) {
                    return locale;
                }
            }
        }

        Self::ZhCn
    }

    /**
    从一个语言标签里解析出当前支持的语言。

    这里只识别当前项目真正支持的两类语言。
    没识别出来时返回 `None`，交给上层走默认策略。
    */
    pub fn from_tag(tag: &str) -> Option<Self> {
        let lower = tag.trim().to_ascii_lowercase();
        if lower.starts_with("zh") || lower.contains("chinese") {
            Some(Self::ZhCn)
        } else if lower.starts_with("en") || lower.contains("english") {
            Some(Self::EnUs)
        } else {
            None
        }
    }

    pub fn event_tab_title(self, key: EventTab) -> &'static str {
        match (self, key) {
            (Self::ZhCn, EventTab::Domain) => "事件",
            (Self::ZhCn, EventTab::Trace) => "调试",
            (Self::ZhCn, EventTab::System) => "系统",
            (Self::EnUs, EventTab::Domain) => "Events",
            (Self::EnUs, EventTab::Trace) => "Trace",
            (Self::EnUs, EventTab::System) => "System",
        }
    }

    pub fn side_label(self, side: SideId) -> &'static str {
        match (self, side) {
            (Self::ZhCn, SideId::Player) => "玩家",
            (Self::ZhCn, SideId::Opponent) => "对手",
            (Self::EnUs, SideId::Player) => "Player",
            (Self::EnUs, SideId::Opponent) => "Opponent",
        }
    }

    pub fn weather_name(self, weather: WeatherKind) -> &'static str {
        match (self, weather) {
            (Self::ZhCn, WeatherKind::Sunny) => "晴天",
            (Self::ZhCn, WeatherKind::Rainy) => "下雨",
            (Self::EnUs, WeatherKind::Sunny) => "Sun",
            (Self::EnUs, WeatherKind::Rainy) => "Rain",
        }
    }

    pub fn mode_human_vs_ai(self) -> &'static str {
        match self {
            Self::ZhCn => "人类 vs AI",
            Self::EnUs => "Human vs AI",
        }
    }

    pub fn status_title(self) -> &'static str {
        match self {
            Self::ZhCn => "状态",
            Self::EnUs => "Status",
        }
    }

    pub fn battlefield_title(self) -> &'static str {
        match self {
            Self::ZhCn => "战场",
            Self::EnUs => "Battlefield",
        }
    }

    pub fn actions_title(self) -> &'static str {
        match self {
            Self::ZhCn => "操作",
            Self::EnUs => "Actions",
        }
    }

    pub fn agent_panel_title(self) -> &'static str {
        match self {
            Self::ZhCn => "AI / 会话",
            Self::EnUs => "Agent / Session",
        }
    }

    pub fn console_title(self) -> &'static str {
        match self {
            Self::ZhCn => "控制台",
            Self::EnUs => "Console",
        }
    }

    pub fn help_title(self) -> &'static str {
        match self {
            Self::ZhCn => "帮助",
            Self::EnUs => "Help",
        }
    }

    pub fn request_title(self) -> &'static str {
        match self {
            Self::ZhCn => "请求",
            Self::EnUs => "Request",
        }
    }

    pub fn turn_title(self) -> &'static str {
        match self {
            Self::ZhCn => "回合",
            Self::EnUs => "Turn",
        }
    }

    pub fn mode_title(self) -> &'static str {
        match self {
            Self::ZhCn => "模式",
            Self::EnUs => "Mode",
        }
    }

    pub fn battle_title(self) -> &'static str {
        match self {
            Self::ZhCn => "对局",
            Self::EnUs => "Battle",
        }
    }

    pub fn seed_title(self) -> &'static str {
        match self {
            Self::ZhCn => "种子",
            Self::EnUs => "Seed",
        }
    }

    pub fn choose_key_label(self) -> &'static str {
        match self {
            Self::ZhCn => "选择",
            Self::EnUs => "Choose",
        }
    }

    pub fn events_key_label(self) -> &'static str {
        match self {
            Self::ZhCn => "切换事件",
            Self::EnUs => "Events",
        }
    }

    pub fn help_key_label(self) -> &'static str {
        match self {
            Self::ZhCn => "帮助",
            Self::EnUs => "Help",
        }
    }

    pub fn continue_key_label(self) -> &'static str {
        match self {
            Self::ZhCn => "继续",
            Self::EnUs => "Continue",
        }
    }

    pub fn quit_key_label(self) -> &'static str {
        match self {
            Self::ZhCn => "退出",
            Self::EnUs => "Quit",
        }
    }

    pub fn tip_help_and_continue(self) -> &'static str {
        match self {
            Self::ZhCn => "提示：按 ? 查看帮助。每次一回合结算完后，按 Enter 或 Space 继续。",
            Self::EnUs => "Tip: press ? for help. After each resolved turn, press Enter or Space to continue.",
        }
    }

    pub fn continue_prompt(self) -> &'static str {
        match self {
            Self::ZhCn => "这一回合已经结算完。按 Enter 或 Space 继续。",
            Self::EnUs => "Turn resolved. Press Enter or Space to continue.",
        }
    }

    pub fn leave_battle_prompt(self) -> &'static str {
        match self {
            Self::ZhCn => "按 Enter 或 Space 离开战斗界面。",
            Self::EnUs => "Press Enter or Space to leave the battle screen.",
        }
    }

    pub fn help_choose_action(self) -> &'static str {
        match self {
            Self::ZhCn => "1-9  选择操作",
            Self::EnUs => "1-9  Choose action",
        }
    }

    pub fn help_switch_event_tab(self) -> &'static str {
        match self {
            Self::ZhCn => "E    切换事件分页",
            Self::EnUs => "E    Switch event tab",
        }
    }

    pub fn help_toggle_help(self) -> &'static str {
        match self {
            Self::ZhCn => "?    打开或关闭帮助",
            Self::EnUs => "?    Toggle this help",
        }
    }

    pub fn help_quit(self) -> &'static str {
        match self {
            Self::ZhCn => "Q    退出战斗界面",
            Self::EnUs => "Q    Quit battle screen",
        }
    }

    pub fn help_continue(self) -> &'static str {
        match self {
            Self::ZhCn => "Enter/Space  继续下一步",
            Self::EnUs => "Enter/Space  Continue",
        }
    }

    pub fn help_color_auto(self) -> &'static str {
        match self {
            Self::ZhCn => "颜色会自动检测。",
            Self::EnUs => "Color is auto-detected.",
        }
    }

    pub fn help_no_color(self) -> &'static str {
        match self {
            Self::ZhCn => "设置 NO_COLOR=1 可以关闭颜色。",
            Self::EnUs => "Set NO_COLOR=1 to disable color.",
        }
    }

    pub fn help_icon_mode(self) -> &'static str {
        match self {
            Self::ZhCn => "设置 SHAMROCK_TUI_ICONS=ascii|unicode|nerd 可以控制图标风格。",
            Self::EnUs => "Set SHAMROCK_TUI_ICONS=ascii|unicode|nerd to control icon style.",
        }
    }

    pub fn waiting_for_player_input(self) -> &'static str {
        match self {
            Self::ZhCn => "等待玩家输入",
            Self::EnUs => "Waiting for player input",
        }
    }

    pub fn tui_ready(self) -> &'static str {
        match self {
            Self::ZhCn => "Shamrock TUI 已就绪",
            Self::EnUs => "Shamrock TUI ready",
        }
    }

    pub fn hotkey_summary(self) -> &'static str {
        match self {
            Self::ZhCn => "快捷键：1-9 选择，E 切换事件分页，? 帮助，Q 退出",
            Self::EnUs => "Hotkeys: 1-9 choose, E switch event tab, ? help, Q quit",
        }
    }

    pub fn invalid_action_index(self, index: usize) -> String {
        match self {
            Self::ZhCn => format!("非法操作编号 {}", index + 1),
            Self::EnUs => format!("Invalid action index {}", index + 1),
        }
    }

    pub fn player_command_accepted(self) -> &'static str {
        match self {
            Self::ZhCn => "已接收玩家操作",
            Self::EnUs => "Player command accepted",
        }
    }

    pub fn ai_chose(self, action: &str) -> String {
        match self {
            Self::ZhCn => format!("AI 选择了 {action}"),
            Self::EnUs => format!("AI chose {action}"),
        }
    }

    pub fn finished_line(self, winner: SideId) -> String {
        match self {
            Self::ZhCn => format!("对战结束 - {}获胜", self.side_label(winner)),
            Self::EnUs => format!("Finished - {} wins", self.side_label(winner)),
        }
    }

    pub fn battle_finished_message(self, winner: SideId) -> String {
        match self {
            Self::ZhCn => format!("\n对战结束。胜者：{}", self.side_label(winner)),
            Self::EnUs => format!("\nBattle finished. Winner: {}", self.side_label(winner)),
        }
    }

    pub fn battle_aborted_message(self) -> &'static str {
        match self {
            Self::ZhCn => "\n对战已中止。",
            Self::EnUs => "\nBattle aborted.",
        }
    }

    pub fn tty_fallback(self) -> &'static str {
        match self {
            Self::ZhCn => "当前没有可用的 TTY，退回纯文本 CLI 模式。",
            Self::EnUs => "TTY not available, falling back to plain CLI mode.",
        }
    }

    pub fn plain_mode_selected(self) -> &'static str {
        match self {
            Self::ZhCn => "使用纯文本 CLI 模式。",
            Self::EnUs => "Using plain CLI mode.",
        }
    }

    pub fn turn_header(self, turn: u16) -> String {
        match self {
            Self::ZhCn => format!("\n第 {turn} 回合"),
            Self::EnUs => format!("\nTurn {turn}"),
        }
    }

    pub fn active_status_line(self, side: SideId, nickname: &str, species: &str, current_hp: i32, max_hp: i32) -> String {
        match self {
            Self::ZhCn => format!("{}当前上场：{}（{}） HP {}/{}", self.side_label(side), nickname, species, current_hp, max_hp),
            Self::EnUs => format!("{} active: {} ({}) HP {}/{}", self.side_label(side), nickname, species, current_hp, max_hp),
        }
    }

    pub fn status_name(self, status: StatusCondition) -> &'static str {
        match (self, status) {
            (Self::ZhCn, StatusCondition::Paralyzed) => "麻痹",
            (Self::ZhCn, StatusCondition::Poisoned) => "中毒",
            (Self::EnUs, StatusCondition::Paralyzed) => "PAR",
            (Self::EnUs, StatusCondition::Poisoned) => "PSN",
        }
    }

    pub fn status_line(self, status: Option<StatusCondition>) -> String {
        match (self, status) {
            (Self::ZhCn, Some(status)) => format!("状态 {}", self.status_name(status)),
            (Self::ZhCn, None) => "状态 正常".to_string(),
            (Self::EnUs, Some(status)) => format!("Status {}", self.status_name(status)),
            (Self::EnUs, None) => "Status OK".to_string(),
        }
    }

    pub fn weather_line(self, weather: Option<(WeatherKind, u8)>) -> String {
        match (self, weather) {
            (Self::ZhCn, Some((kind, turns))) => format!("天气 {}（剩余 {} 回合）", self.weather_name(kind), turns),
            (Self::ZhCn, None) => "天气 无".to_string(),
            (Self::EnUs, Some((kind, turns))) => format!("Weather {} ({turns} turns left)", self.weather_name(kind)),
            (Self::EnUs, None) => "Weather None".to_string(),
        }
    }

    pub fn side_type_status_line(self, type_line: &str, status_line: &str) -> String {
        match self {
            Self::ZhCn => format!("{type_line}   {status_line}"),
            Self::EnUs => format!("{type_line}   {status_line}"),
        }
    }

    pub fn pokemon_type_line(self, primary: ElementType, secondary: Option<ElementType>) -> String {
        match secondary {
            Some(extra) => match self {
                Self::ZhCn => format!("属性 {} / {}", self.element_type_name(primary), self.element_type_name(extra)),
                Self::EnUs => format!("Type {} / {}", self.element_type_name(primary), self.element_type_name(extra)),
            },
            None => match self {
                Self::ZhCn => format!("属性 {}", self.element_type_name(primary)),
                Self::EnUs => format!("Type {}", self.element_type_name(primary)),
            },
        }
    }

    pub fn choose_action_prompt(self) -> &'static str {
        match self {
            Self::ZhCn => "选择操作：",
            Self::EnUs => "Choose action: ",
        }
    }

    pub fn invalid_choice(self) -> &'static str {
        match self {
            Self::ZhCn => "输入无效，请输入菜单编号。",
            Self::EnUs => "Invalid choice. Enter the menu number.",
        }
    }

    pub fn plain_commands_hint(self) -> &'static str {
        match self {
            Self::ZhCn => "输入数字选择操作，输入 history 查看完整日志，输入 help 查看命令。",
            Self::EnUs => "Enter a number to choose, 'history' to view full log, or 'help' to view commands.",
        }
    }

    pub fn history_title(self) -> &'static str {
        match self {
            Self::ZhCn => "对战历史",
            Self::EnUs => "Battle History",
        }
    }

    pub fn recent_log_title(self) -> &'static str {
        match self {
            Self::ZhCn => "最近日志",
            Self::EnUs => "Recent Log",
        }
    }

    pub fn history_empty(self) -> &'static str {
        match self {
            Self::ZhCn => "当前还没有历史日志。",
            Self::EnUs => "No history yet.",
        }
    }

    pub fn help_commands_plain(self) -> Vec<&'static str> {
        match self {
            Self::ZhCn => vec![
                "数字：选择对应操作",
                "history：查看完整历史日志",
                "help：查看这组命令",
            ],
            Self::EnUs => vec![
                "number: choose the matching action",
                "history: show the full battle log",
                "help: show these commands",
            ],
        }
    }

    pub fn action_use(self, move_name: &str) -> String {
        match self {
            Self::ZhCn => format!("使用 {move_name}"),
            Self::EnUs => format!("Use {move_name}"),
        }
    }

    pub fn action_switch(self, nickname: &str) -> String {
        match self {
            Self::ZhCn => format!("换到 {nickname}"),
            Self::EnUs => format!("Switch to {nickname}"),
        }
    }

    pub fn resolving_turn(self, turn: u16) -> String {
        match self {
            Self::ZhCn => format!("\n开始结算第 {turn} 回合"),
            Self::EnUs => format!("\nResolving turn {turn}"),
        }
    }

    pub fn used_move(self, side: SideId, move_name: &str) -> String {
        match self {
            Self::ZhCn => format!("{}使用了 {}", self.side_label(side), move_name),
            Self::EnUs => format!("{} used {}", self.side_label(side), move_name),
        }
    }

    pub fn missed_move(self, side: SideId, move_name: &str) -> String {
        match self {
            Self::ZhCn => format!("{}的 {} 没打中", self.side_label(side), move_name),
            Self::EnUs => format!("{} missed with {}", self.side_label(side), move_name),
        }
    }

    pub fn damage_taken(self, target: SideId, amount: u16, remaining_hp: u16) -> String {
        match self {
            Self::ZhCn => format!("{}受到了 {} 点伤害，当前 HP {}", self.side_label(target), amount, remaining_hp),
            Self::EnUs => format!("{} took {} damage and is now at {} HP", self.side_label(target), amount, remaining_hp),
        }
    }

    pub fn active_fainted(self, side: SideId) -> String {
        match self {
            Self::ZhCn => format!("{}的当前宝可梦倒下了", self.side_label(side)),
            Self::EnUs => format!("{} active pokemon fainted", self.side_label(side)),
        }
    }

    pub fn healed(self, side: SideId, amount: u16, remaining_hp: u16) -> String {
        match self {
            Self::ZhCn => format!("{}恢复了 {} 点 HP，当前 HP {}", self.side_label(side), amount, remaining_hp),
            Self::EnUs => format!("{} healed {} HP and is now at {} HP", self.side_label(side), amount, remaining_hp),
        }
    }

    pub fn status_applied(self, side: SideId, status: StatusCondition) -> String {
        match self {
            Self::ZhCn => format!("{}陷入了{}", self.side_label(side), self.status_name(status)),
            Self::EnUs => format!("{} is now {}", self.side_label(side), self.status_name(status)),
        }
    }

    pub fn stat_name(self, stat: StatId) -> &'static str {
        match (self, stat) {
            (Self::ZhCn, StatId::Attack) => "攻击",
            (Self::ZhCn, StatId::Defense) => "防御",
            (Self::ZhCn, StatId::Speed) => "速度",
            (Self::EnUs, StatId::Attack) => "Attack",
            (Self::EnUs, StatId::Defense) => "Defense",
            (Self::EnUs, StatId::Speed) => "Speed",
        }
    }

    pub fn stat_stage_changed(self, side: SideId, stat: StatId, new_stage: i8) -> String {
        match self {
            Self::ZhCn => format!("{}的{}阶段变为 {}", self.side_label(side), self.stat_name(stat), new_stage),
            Self::EnUs => format!("{} {} stage is now {}", self.side_label(side), self.stat_name(stat), new_stage),
        }
    }

    pub fn action_blocked_by_status(self, side: SideId, status: StatusCondition) -> String {
        match self {
            Self::ZhCn => format!("{}因{}无法行动", self.side_label(side), self.status_name(status)),
            Self::EnUs => format!("{} is unable to move because of {}", self.side_label(side), self.status_name(status)),
        }
    }

    pub fn residual_damage(self, side: SideId, status: StatusCondition, amount: u16, remaining_hp: u16) -> String {
        match self {
            Self::ZhCn => format!("{}因{}受到 {} 点伤害，当前 HP {}", self.side_label(side), self.status_name(status), amount, remaining_hp),
            Self::EnUs => format!("{} took {} damage from {} and is now at {} HP", self.side_label(side), amount, self.status_name(status), remaining_hp),
        }
    }

    pub fn sent_out(self, side: SideId, nickname: &str) -> String {
        match self {
            Self::ZhCn => format!("{}派出了 {}", self.side_label(side), nickname),
            Self::EnUs => format!("{} sent out {}", self.side_label(side), nickname),
        }
    }

    pub fn battle_wins(self, winner: SideId) -> String {
        match self {
            Self::ZhCn => format!("{}赢下了对战", self.side_label(winner)),
            Self::EnUs => format!("{} wins the battle", self.side_label(winner)),
        }
    }

    pub fn weather_started(self, weather: WeatherKind, remaining_turns: u8) -> String {
        match self {
            Self::ZhCn => format!("天气变为{}，持续 {} 回合", self.weather_name(weather), remaining_turns),
            Self::EnUs => format!("Weather changed to {} for {} turns", self.weather_name(weather), remaining_turns),
        }
    }

    pub fn weather_ended(self, weather: WeatherKind) -> String {
        match self {
            Self::ZhCn => format!("{}结束了", self.weather_name(weather)),
            Self::EnUs => format!("{} ended", self.weather_name(weather)),
        }
    }

    pub fn forced_switch(self, side: SideId) -> String {
        match self {
            Self::ZhCn => format!("{}被强制换下", self.side_label(side)),
            Self::EnUs => format!("{} was forced to switch out", self.side_label(side)),
        }
    }

    pub fn trace_choice_accepted(self, side: SideId) -> String {
        match self {
            Self::ZhCn => format!("TRACE {}的输入已接受", self.side_label(side)),
            Self::EnUs => format!("TRACE {:?} choice accepted", side),
        }
    }

    pub fn trace_turn_resolved(self, turn: u16) -> String {
        match self {
            Self::ZhCn => format!("TRACE 第 {turn} 回合已结算"),
            Self::EnUs => format!("TRACE turn {turn} resolved"),
        }
    }

    pub fn trace_move_order(self, first: SideId, second: SideId) -> String {
        match self {
            Self::ZhCn => format!("TRACE 行动顺序 {} -> {}", self.side_label(first), self.side_label(second)),
            Self::EnUs => format!("TRACE {:?} -> {:?}", first, second),
        }
    }

    pub fn trace_accuracy(self, side: SideId, roll: u8, needed: u8) -> String {
        match self {
            Self::ZhCn => format!("TRACE {}命中判定 {} / {}", self.side_label(side), roll, needed),
            Self::EnUs => format!("TRACE {:?} accuracy {} / {}", side, roll, needed),
        }
    }

    pub fn trace_status_roll(self, side: SideId, status: StatusCondition, roll: u8, needed: u8) -> String {
        match self {
            Self::ZhCn => format!("TRACE {}{}判定 {} / {}", self.side_label(side), self.status_name(status), roll, needed),
            Self::EnUs => format!("TRACE {:?} {} roll {} / {}", side, self.status_name(status), roll, needed),
        }
    }

    pub fn trace_damage(self, side: SideId, move_name: &str, damage: u16) -> String {
        match self {
            Self::ZhCn => format!("TRACE {}的 {} 伤害 {}", self.side_label(side), move_name, damage),
            Self::EnUs => format!("TRACE {:?} {} damage {}", side, move_name, damage),
        }
    }

    pub fn trace_weather_damage(self, weather: WeatherKind, move_name: &str) -> String {
        match self {
            Self::ZhCn => format!("TRACE {}影响了 {} 的伤害", self.weather_name(weather), move_name),
            Self::EnUs => format!("TRACE {} affected {} damage", self.weather_name(weather), move_name),
        }
    }

    pub fn trace_action_skipped(self, side: SideId) -> String {
        match self {
            Self::ZhCn => format!("TRACE {}的行动被跳过", self.side_label(side)),
            Self::EnUs => format!("TRACE {:?} action skipped", side),
        }
    }

    pub fn metric_line(self, turn: u16, domain_events: usize, trace_events: usize) -> String {
        match self {
            Self::ZhCn => format!("指标 第 {turn} 回合 domain {domain_events} trace {trace_events}"),
            Self::EnUs => format!("METRIC turn {turn} domain {domain_events} trace {trace_events}"),
        }
    }

    pub fn replay_saved(self, path: &str) -> String {
        match self {
            Self::ZhCn => format!("回放已保存到 {path}"),
            Self::EnUs => format!("Replay saved to {path}"),
        }
    }

    pub fn checkpoint_battle_finished(self) -> &'static str {
        match self {
            Self::ZhCn => "对战结束",
            Self::EnUs => "battle finished",
        }
    }

    pub fn checkpoint_turn_resolved(self, turn: u16) -> String {
        match self {
            Self::ZhCn => format!("第 {turn} 回合结算完成"),
            Self::EnUs => format!("turn {turn} resolved"),
        }
    }

    pub fn opponent_chooses(self, action: &str) -> String {
        match self {
            Self::ZhCn => format!("对手选择：{action}"),
            Self::EnUs => format!("Opponent chooses: {action}"),
        }
    }

    pub fn hp_line(self, current_hp: i32, max_hp: i32) -> String {
        match self {
            Self::ZhCn | Self::EnUs => format!("HP {current_hp}/{max_hp}"),
        }
    }

    pub fn alive_line(self, alive_count: usize, total_count: usize) -> String {
        match self {
            Self::ZhCn => format!("存活 {alive_count}/{total_count}"),
            Self::EnUs => format!("Alive {alive_count}/{total_count}"),
        }
    }

    pub fn latest_side_summary(self, side: SideId) -> String {
        match (self, side) {
            (Self::ZhCn, SideId::Player) => "等待指令".to_string(),
            (Self::ZhCn, SideId::Opponent) => "AI 控制".to_string(),
            (Self::EnUs, SideId::Player) => "Awaiting command".to_string(),
            (Self::EnUs, SideId::Opponent) => "AI controlled".to_string(),
        }
    }

    pub fn request_label(self, request: battle_core::Request) -> String {
        match request {
            battle_core::Request::ChooseAction { side } => match self {
                Self::ZhCn => format!("轮到{}操作", self.side_label(side)),
                Self::EnUs => format!("{} to act", self.side_label(side)),
            },
            battle_core::Request::Finished { winner } => self.finished_line(winner),
        }
    }

    pub fn action_kind_move(self) -> &'static str {
        match self {
            Self::ZhCn => "招式",
            Self::EnUs => "MOVE",
        }
    }

    pub fn action_kind_switch(self) -> &'static str {
        match self {
            Self::ZhCn => "换人",
            Self::EnUs => "SWITCH",
        }
    }

    pub fn action_meta_type(self) -> &'static str {
        match self {
            Self::ZhCn => "属性",
            Self::EnUs => "Type",
        }
    }

    pub fn action_meta_power(self) -> &'static str {
        match self {
            Self::ZhCn => "威力",
            Self::EnUs => "Power",
        }
    }

    pub fn element_type_name(self, element_type: ElementType) -> &'static str {
        match (self, element_type) {
            (Self::ZhCn, ElementType::Normal) => "一般",
            (Self::ZhCn, ElementType::Electric) => "电",
            (Self::ZhCn, ElementType::Fire) => "火",
            (Self::ZhCn, ElementType::Water) => "水",
            (Self::ZhCn, ElementType::Grass) => "草",
            (Self::EnUs, ElementType::Normal) => "Normal",
            (Self::EnUs, ElementType::Electric) => "Electric",
            (Self::EnUs, ElementType::Fire) => "Fire",
            (Self::EnUs, ElementType::Water) => "Water",
            (Self::EnUs, ElementType::Grass) => "Grass",
        }
    }

    pub fn agent_name_line(self, name: &str) -> String {
        match self {
            Self::ZhCn => format!("AI：{name}"),
            Self::EnUs => format!("Agent: {name}"),
        }
    }

    pub fn fallback_none(self) -> &'static str {
        match self {
            Self::ZhCn => "回退：无",
            Self::EnUs => "Fallback: none",
        }
    }
}

impl BattleViewText for Locale {
    fn event_tab_title(self, tab: EventTab) -> &'static str { self.event_tab_title(tab) }
    fn mode_human_vs_ai(self) -> &'static str { self.mode_human_vs_ai() }
    fn weather_line(self, weather: Option<(WeatherKind, u8)>) -> String { self.weather_line(weather) }
    fn side_label(self, side: SideId) -> &'static str { self.side_label(side) }
    fn pokemon_type_line(self, primary: ElementType, secondary: Option<ElementType>) -> String {
        self.pokemon_type_line(primary, secondary)
    }
    fn hp_line(self, current_hp: i32, max_hp: i32) -> String { self.hp_line(current_hp, max_hp) }
    fn status_line(self, status: Option<StatusCondition>) -> String { self.status_line(status) }
    fn alive_line(self, alive_count: usize, party_size: usize) -> String { self.alive_line(alive_count, party_size) }
    fn latest_side_summary(self, side: SideId) -> String { self.latest_side_summary(side) }
    fn request_label(self, request: battle_core::Request) -> String { self.request_label(request) }
    fn action_kind_move(self) -> &'static str { self.action_kind_move() }
    fn action_kind_switch(self) -> &'static str { self.action_kind_switch() }
    fn action_switch(self, nickname: &str) -> String { self.action_switch(nickname) }
    fn element_type_name(self, element_type: ElementType) -> &'static str { self.element_type_name(element_type) }
    fn agent_name_line(self, agent_name: &str) -> String { self.agent_name_line(agent_name) }
    fn fallback_none(self) -> &'static str { self.fallback_none() }
}

#[cfg(test)]
mod tests {
    use super::Locale;

    #[test]
    fn detects_explicit_language_tag() {
        assert_eq!(Locale::from_tag("zh-CN"), Some(Locale::ZhCn));
        assert_eq!(Locale::from_tag("en_US.UTF-8"), Some(Locale::EnUs));
    }
}
