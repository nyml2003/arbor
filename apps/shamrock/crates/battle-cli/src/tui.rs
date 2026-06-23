use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
};

use battle_view::{ActionView, EventTab, PublicBattleView, SidePanelView};

use crate::i18n::Locale;
use crate::theme::{IconName, UiTheme};

/**
这个文件只负责终端界面绘制和键盘交互。

它不读取 battle-core 内部状态，也不做规则判断。
它唯一消费的输入是已经整理好的 `PublicBattleView`。
*/
/**
`Tui` 封装了 ratatui/crossterm 所需的终端状态。

把终端对象、当前 tab 和资源清理逻辑放在这里，
可以让外层 battle loop 只关心“画什么”和“读到什么输入”。
*/
pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    selected_tab: EventTab,
    show_help: bool,
    theme: UiTheme,
}

impl Tui {
    /**
    进入 TUI 模式并接管当前终端。

    这里一次完成 raw mode、alternate screen 和 terminal backend 初始化，
    让调用方只处理成功后的 UI 生命周期。
    */
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal, selected_tab: EventTab::Domain, show_help: false, theme: UiTheme::detect() })
    }

    /**
    返回当前事件区选中的 tab。

    battle loop 用它把 UI 状态回传给 view projection，这样视图切换不需要直接改 battle 数据。
    */
    pub fn selected_tab(&self) -> EventTab {
        self.selected_tab
    }

    /**
    在事件区 tab 之间循环切换。

    tab 本身是界面层状态，不应该泄漏到 battle-core。
    */
    pub fn cycle_tab(&mut self) {
        self.selected_tab = next_event_tab(self.selected_tab);
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /**
    根据当前公开视图重绘整屏。

    它不做 battle 推进，只负责把现有 view 投影到终端。
    */
    pub fn draw(&mut self, view: &PublicBattleView<Locale>) -> io::Result<()> {
        let theme = self.theme;
        let show_help = self.show_help;
        self.terminal.draw(|frame| render_root(frame, view, theme, show_help))?;
        Ok(())
    }

    /**
    等待玩家从键盘给出一个动作选择。

    `E` 和 `?` 都在这个函数内部直接处理，不会把外层 battle loop 打断。
    这样 tab 切换和帮助弹层会停留在屏幕上，而不是一闪而过。
    */
    pub fn wait_for_player_action(&mut self, view: &PublicBattleView<Locale>) -> io::Result<Option<usize>> {
        loop {
            self.draw(view)?;
            if event::poll(Duration::from_millis(250))? {
                match event::read()? {
                    Event::Key(key) => match key.code {
                        KeyCode::Char('q') => return Ok(None),
                        KeyCode::Char('e') | KeyCode::Char('E') if !self.show_help => {
                            self.cycle_tab();
                        }
                        KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::Char('H') => {
                            self.toggle_help();
                        }
                        KeyCode::Esc | KeyCode::Enter if self.show_help => {
                            self.toggle_help();
                        }
                        KeyCode::Char(ch) if ch.is_ascii_digit() && !self.show_help => {
                            if let Some(value) = ch.to_digit(10) {
                                let index = value.saturating_sub(1) as usize;
                                return Ok(Some(index));
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }

    /**
    在一整回合结算完成后暂停，等玩家确认继续。

    这样玩家能看清楚一整回合里的事件，不会因为自动推进而错过信息。
    */
    pub fn wait_for_continue(&mut self, view: &PublicBattleView<Locale>) -> io::Result<bool> {
        loop {
            self.draw(view)?;
            if event::poll(Duration::from_millis(250))? {
                match event::read()? {
                    Event::Key(key) => match key.code {
                        KeyCode::Char('q') => return Ok(false),
                        KeyCode::Char('e') | KeyCode::Char('E') if !self.show_help => {
                            self.cycle_tab();
                        }
                        KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::Char('H') => {
                            self.toggle_help();
                        }
                        KeyCode::Esc | KeyCode::Enter if self.show_help => {
                            self.toggle_help();
                        }
                        KeyCode::Enter | KeyCode::Char(' ') if !self.show_help => return Ok(true),
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }
}

/**
离开 TUI 时统一恢复终端状态。

把清理逻辑放在 `Drop` 里，可以减少异常退出后终端被留在 raw mode 的风险。
*/
impl Drop for Tui {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

/**
绘制整屏根布局。

它的职责是切分区域并调用各个子渲染函数，不负责准备数据。
*/
pub fn render_root(frame: &mut Frame<'_>, view: &PublicBattleView<Locale>, theme: UiTheme, show_help: bool) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(12),
            Constraint::Length(10),
        ])
        .split(frame.area());

    render_status_bar(frame, root[0], view, theme);

    let middle = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(root[1]);

    render_battlefield(frame, middle[0], view, theme);

    let side = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(6)])
        .split(middle[1]);

    render_action_panel(frame, side[0], view, theme);
    render_agent_panel(frame, side[1], view, theme);
    render_event_console(frame, root[2], view, theme);
    if show_help {
        render_help_overlay(frame, theme, view.locale);
    }
}

/**
绘制顶部状态栏。

这里优先放整场 battle 的全局信息，让玩家一眼知道当前模式、回合和输入状态。
*/
fn render_status_bar(frame: &mut Frame<'_>, area: ratatui::layout::Rect, view: &PublicBattleView<Locale>, theme: UiTheme) {
    let locale = view.locale;
    let request_style = if theme.supports_color() {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    };
    let text = vec![
        Line::from(vec![
            icon_span(theme, IconName::Status, Color::Cyan),
            Span::styled(format!(" {} ", locale.status_title()), theme.style(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(format!("{} {}   {} {}   ", locale.mode_title(), view.mode, locale.battle_title(), view.battle_id)),
            Span::styled(format!("{} {}   ", locale.turn_title(), view.turn), theme.style(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} {}   ", locale.request_title(), view.request_label), request_style),
            Span::raw(format!("{} {}", locale.seed_title(), view.seed)),
        ]),
        Line::from(view.weather_line.clone()),
        Line::from(vec![
            key_hint(theme, "1-9", locale.choose_key_label()),
            Span::raw("  "),
            key_hint(theme, "E", locale.events_key_label()),
            Span::raw("  "),
            key_hint(theme, "?", locale.help_key_label()),
            Span::raw("  "),
            key_hint(theme, "Enter/Space", locale.continue_key_label()),
            Span::raw("  "),
            key_hint(theme, "Q", locale.quit_key_label()),
        ]),
        Line::from(locale.tip_help_and_continue()),
    ];
    let widget = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(title_with_icon(theme, IconName::Status, locale.status_title())))
        .style(Style::default().fg(Color::White));
    frame.render_widget(widget, area);
}

/**
绘制主战场区域。

当前布局用上下两块对称展示双方状态，这样比较双方 active 会更直接。
*/
fn render_battlefield(frame: &mut Frame<'_>, area: ratatui::layout::Rect, view: &PublicBattleView<Locale>, theme: UiTheme) {
    let outer = Block::default().borders(Borders::ALL).title(title_with_icon(theme, IconName::Battlefield, view.locale.battlefield_title()));
    frame.render_widget(outer, area);
    let inner = area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 1 });
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    render_side_panel(frame, split[0], &view.opponent, view.locale, theme, IconName::Opponent, Color::LightRed);
    render_side_panel(frame, split[1], &view.player, view.locale, theme, IconName::Player, Color::LightCyan);
}

/**
绘制单边状态面板。

这里完全依赖 `SidePanelView`，而不是自己去遍历 battle 状态。
这样 widget 保持简单，数据准备都留在 view projection。
*/
fn render_side_panel(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    side: &SidePanelView,
    locale: crate::i18n::Locale,
    theme: UiTheme,
    icon: IconName,
    accent: Color,
) {
    let hp_style = hp_style(theme, side.current_hp, side.max_hp);
    let wait_style = if side.is_waiting {
        theme.style(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        theme.style(Color::DarkGray)
    };
    let lines = vec![
        Line::from(vec![
            icon_span(theme, icon, accent),
            Span::styled(format!(" {} ", side.active_name), theme.style(accent).add_modifier(Modifier::BOLD)),
            Span::raw(format!("/ {}", side.species_name)),
        ]),
        pokemon_type_line(side, locale, theme),
        Line::from(vec![
            icon_span(theme, IconName::Hp, hp_style.fg.unwrap_or(Color::White)),
            Span::styled(format!(" {}", side.hp_line), hp_style),
        ]),
        Line::from(side.status_line.clone()),
        Line::from(side.bench_line.clone()),
        Line::from(Span::styled(side.latest_summary.clone(), wait_style)),
    ];
    let widget = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(side.side_label.as_str()).border_style(theme.style(accent)))
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, area);
}

fn pokemon_type_line(side: &SidePanelView, locale: crate::i18n::Locale, theme: UiTheme) -> Line<'static> {
    let mut spans = Vec::new();
    append_type_chip(&mut spans, side.primary_type, locale, theme);
    if let Some(extra) = side.secondary_type {
        spans.push(Span::raw(" "));
        append_type_chip(&mut spans, extra, locale, theme);
    }
    Line::from(spans)
}

fn append_type_chip(
    spans: &mut Vec<Span<'static>>,
    element: battle_data::ElementType,
    locale: crate::i18n::Locale,
    theme: UiTheme,
) {
    let color = match element {
        battle_data::ElementType::Normal => Color::Gray,
        battle_data::ElementType::Electric => Color::Yellow,
        battle_data::ElementType::Fire => Color::LightRed,
        battle_data::ElementType::Water => Color::Blue,
        battle_data::ElementType::Grass => Color::Green,
    };
    let label = locale.element_type_name(element);
    spans.push(Span::styled(format!("[{label}]"), theme.style(color).add_modifier(Modifier::BOLD)));
}

/**
绘制操作面板。

动作是否合法已经在更外面的 view 构建阶段确定好了，这里只负责显示。
*/
fn render_action_panel(frame: &mut Frame<'_>, area: ratatui::layout::Rect, view: &PublicBattleView<Locale>, theme: UiTheme) {
    let locale = view.locale;
    let items: Vec<ListItem<'_>> = view
        .legal_actions
        .iter()
        .map(|action| {
            let line = Line::from(vec![
                Span::styled(format!("{:>2}", action.hotkey), theme.style(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled(format!("[{}]", action.kind), theme.style(Color::Magenta)),
                Span::raw(" "),
                Span::styled(action.token.clone(), theme.style(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::raw(action.label.clone()),
            ]);
            let meta = action_meta_line(action, locale, theme);
            ListItem::new(vec![line, meta])
        })
        .collect();

    let widget = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title_with_icon(theme, IconName::Action, view.locale.actions_title())))
        .highlight_style(theme.style(Color::Yellow).add_modifier(Modifier::BOLD));
    frame.render_widget(widget, area);
}

fn action_meta_line(action: &ActionView, locale: crate::i18n::Locale, theme: UiTheme) -> Line<'static> {
    let mut spans = Vec::new();
    if let Some(element) = &action.element {
        spans.push(Span::styled(
            format!("{} {}", locale.action_meta_type(), element),
            theme.style(Color::Blue),
        ));
    }
    if let Some(power) = action.power {
        if !spans.is_empty() {
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled(
            format!("{} {}", locale.action_meta_power(), power),
            theme.style(Color::LightGreen),
        ));
    }
    if spans.is_empty() {
        spans.push(Span::styled(action.kind.clone(), theme.style(Color::DarkGray)));
    }
    Line::from(spans)
}

/**
绘制 agent/session 面板。

这块是附加信息区，用来显示当前 AI 摘要和会话状态，不参与规则。
*/
fn render_agent_panel(frame: &mut Frame<'_>, area: ratatui::layout::Rect, view: &PublicBattleView<Locale>, theme: UiTheme) {
    let items: Vec<ListItem<'_>> = view
        .agent_summary
        .iter()
        .map(|line| ListItem::new(line.clone()))
        .collect();
    let widget = List::new(items).block(Block::default().borders(Borders::ALL).title(title_with_icon(theme, IconName::Agent, view.locale.agent_panel_title())));
    frame.render_widget(widget, area);
}

/**
绘制底部事件区。

它先渲染外框和 tab，再渲染当前 tab 对应的文本列表。
这样事件类型扩展和列表内容扩展可以继续分开处理。
*/
fn render_event_console(frame: &mut Frame<'_>, area: ratatui::layout::Rect, view: &PublicBattleView<Locale>, theme: UiTheme) {
    let outer = Block::default().borders(Borders::ALL).title(title_with_icon(theme, IconName::Events, view.locale.console_title()));
    frame.render_widget(outer, area);

    let inner = area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 1 });
    frame.render_widget(Clear, inner);

    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(inner);

    let tabs = Tabs::new(vec![
        Line::from(EventTab::Domain.title(view.locale)),
        Line::from(EventTab::Trace.title(view.locale)),
        Line::from(EventTab::System.title(view.locale)),
    ])
    .select(match view.selected_tab {
        EventTab::Domain => 0,
        EventTab::Trace => 1,
        EventTab::System => 2,
    })
    .block(Block::default().borders(Borders::BOTTOM))
    .style(theme.style(Color::DarkGray))
    .highlight_style(theme.style(Color::Yellow).add_modifier(Modifier::BOLD));
    frame.render_widget(tabs, split[0]);

    let items: Vec<ListItem<'_>> = view
        .recent_events
        .iter()
        .map(|line| ListItem::new(line.clone()))
        .collect();
    let widget = List::new(items);
    frame.render_widget(widget, split[1]);
}

fn render_help_overlay(frame: &mut Frame<'_>, theme: UiTheme, locale: crate::i18n::Locale) {
    let popup = centered_rect(frame.area(), 60, 42);
    frame.render_widget(Clear, popup);
    let lines = vec![
        Line::from(vec![icon_span(theme, IconName::Help, Color::Yellow), Span::styled(format!(" {}", locale.help_title()), theme.style(Color::Yellow).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(locale.help_choose_action()),
        Line::from(locale.help_switch_event_tab()),
        Line::from(locale.help_toggle_help()),
        Line::from(locale.help_continue()),
        Line::from(locale.help_quit()),
        Line::from(""),
        Line::from(locale.help_color_auto()),
        Line::from(locale.help_no_color()),
        Line::from(locale.help_icon_mode()),
    ];
    let widget = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title_with_icon(theme, IconName::Help, locale.help_title())).border_style(theme.style(Color::Yellow)))
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, popup);
}

/**
给事件区算下一个 tab。

把循环顺序集中到这个小函数里，测试也更容易直接验证。
*/
fn next_event_tab(current: EventTab) -> EventTab {
    match current {
        EventTab::Domain => EventTab::Trace,
        EventTab::Trace => EventTab::System,
        EventTab::System => EventTab::Domain,
    }
}

fn centered_rect(area: ratatui::layout::Rect, percent_x: u16, percent_y: u16) -> ratatui::layout::Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1]);
    horizontal[1]
}

fn title_with_icon(theme: UiTheme, icon: IconName, text: &'static str) -> Line<'static> {
    let glyph = theme.icon(icon);
    if glyph.is_empty() {
        Line::from(text)
    } else {
        Line::from(format!("{glyph} {text}"))
    }
}

fn icon_span(theme: UiTheme, icon: IconName, color: Color) -> Span<'static> {
    let glyph = theme.icon(icon);
    if glyph.is_empty() {
        Span::raw("")
    } else {
        Span::styled(format!("{glyph} "), theme.style(color))
    }
}

fn key_hint(theme: UiTheme, key: &'static str, action: &'static str) -> Span<'static> {
    Span::styled(format!("[{key}] {action}"), theme.style(Color::Yellow).add_modifier(Modifier::BOLD))
}

fn hp_style(theme: UiTheme, current_hp: i32, max_hp: i32) -> Style {
    if max_hp <= 0 {
        return Style::default();
    }

    let ratio = current_hp as f32 / max_hp as f32;
    if ratio >= 0.66 {
        theme.style(Color::Green).add_modifier(Modifier::BOLD)
    } else if ratio >= 0.33 {
        theme.style(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        theme.style(Color::LightRed).add_modifier(Modifier::BOLD)
    }
}

#[cfg(test)]
mod tests {
    use battle_core::{BattleInit, initialize_battle};
    use battle_data::{load_demo_enemy_team, load_demo_player_team, load_gen1_demo_pack};
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    use crate::i18n::Locale;
    use battle_view::{EventTab, UiEventLog, ViewerProfile, build_battle_snapshot, build_public_battle_view};

    use super::{next_event_tab, render_root};

    #[test]
    fn root_render_contains_panel_titles() {
        let data = load_gen1_demo_pack();
        let state = initialize_battle(BattleInit { player: load_demo_player_team(), opponent: load_demo_enemy_team() }, &data).unwrap();
        let actions = battle_format::legal_actions(&state, battle_core::SideId::Player);
        let snapshot = build_battle_snapshot(&state, &data, "demo", 42, &actions, ViewerProfile::LocalPlayer(battle_core::SideId::Player));
        let view = build_public_battle_view(&snapshot, Locale::EnUs, &UiEventLog::default(), EventTab::Domain, "ready");

        let backend = TestBackend::new(100, 32);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = crate::theme::UiTheme {
            color_mode: crate::theme::ColorMode::Basic,
            icon_mode: crate::theme::IconMode::Unicode,
        };
        terminal.draw(|frame| render_root(frame, &view, theme, false)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        assert_buffer_contains(&buffer, "Status");
        assert_buffer_contains(&buffer, "Actions");
        assert_buffer_contains(&buffer, "Console");
        assert_buffer_contains(&buffer, "Agent / Session");
    }

    #[test]
    fn tab_cycle_rotates_all_panels() {
        assert_eq!(next_event_tab(EventTab::Domain).title(Locale::EnUs), "Trace");
        assert_eq!(next_event_tab(EventTab::Trace).title(Locale::EnUs), "System");
        assert_eq!(next_event_tab(EventTab::System).title(Locale::EnUs), "Events");
    }

    fn assert_buffer_contains(buffer: &Buffer, needle: &str) {
        let rendered: String = buffer.content().iter().map(|cell| cell.symbol()).collect();
        assert!(rendered.contains(needle), "buffer did not contain {needle}");
    }
}
