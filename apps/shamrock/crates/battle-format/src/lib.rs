use battle_core::{BattleAction, BattleState, CombatPokemon, Request, SideId, requested_side};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormatPhase {
    ChooseAction,
    Frozen,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormatContext {
    pub phase: FormatPhase,
}

impl FormatContext {
    pub fn singles_default() -> Self {
        Self {
            phase: FormatPhase::ChooseAction,
        }
    }
}

/**
这个 crate 负责“当前局面允许做什么”，不负责“做了之后会怎样结算”。

它的作用是把输入合法性从 `battle-core` 的推进逻辑里拆出来。
这样 CLI、AI 和测试都能先拿到一份统一的合法动作列表，再决定如何选招。
*/
/**
列出当前这一边可以提交的全部合法动作。

这里的设计重点是给外层一个稳定动作集合，而不是直接推进 battle。
如果当前根本还没轮到这边行动，就返回空列表，避免外层自己猜时机。
*/
pub fn legal_actions(state: &BattleState, side: SideId) -> Vec<BattleAction> {
    legal_actions_with_context(state, side, FormatContext::singles_default())
}

pub fn legal_actions_with_context(
    state: &BattleState,
    side: SideId,
    context: FormatContext,
) -> Vec<BattleAction> {
    if context.phase != FormatPhase::ChooseAction {
        return Vec::new();
    }

    if requested_side(state) != (Request::ChooseAction { side }) {
        return Vec::new();
    }

    let team = &state.teams[side.index()];
    let active = &team.party[team.active];
    let mut actions = move_actions(active);

    for (index, pokemon) in team.party.iter().enumerate() {
        if index != team.active && !pokemon.is_fainted() {
            actions.push(BattleAction::Switch(index));
        }
    }

    actions
}

/**
用 `legal_actions` 做一次简单包含判断。

这个函数本身没有额外规则，只是给调用方一个更直白的布尔接口。
这样测试、CLI 和未来的 agent 代码在只想问“这个动作行不行”时，不用手动写 contains。
*/
pub fn is_legal_action(state: &BattleState, side: SideId, action: BattleAction) -> bool {
    is_legal_action_with_context(state, side, action, FormatContext::singles_default())
}

pub fn is_legal_action_with_context(
    state: &BattleState,
    side: SideId,
    action: BattleAction,
    context: FormatContext,
) -> bool {
    legal_actions_with_context(state, side, context).contains(&action)
}

/**
根据当前 active 的招式槽位生成出招动作列表。

现在每个宝可梦固定有 4 个槽位，所以这里直接把槽位索引映射成 `UseMove`。
更复杂的 PP、禁用和锁招规则，后面仍然可以继续收敛到这个动作生成阶段。
*/
fn move_actions(active: &CombatPokemon) -> Vec<BattleAction> {
    active
        .moves
        .iter()
        .enumerate()
        .map(|(index, _)| BattleAction::UseMove(index))
        .collect()
}

#[cfg(test)]
mod tests {
    use battle_core::{BattleInit, RngState, initialize_battle, step};
    use battle_data::{load_demo_enemy_team, load_demo_player_team, load_gen1_demo_pack};

    use super::{FormatContext, FormatPhase, is_legal_action, legal_actions, legal_actions_with_context};

    #[test]
    fn player_starts_with_move_and_switch_choices() {
        let data = load_gen1_demo_pack();
        let state = initialize_battle(BattleInit { player: load_demo_player_team(), opponent: load_demo_enemy_team() }, &data).unwrap();
        let actions = legal_actions(&state, battle_core::SideId::Player);

        assert!(actions.contains(&battle_core::BattleAction::UseMove(0)));
        assert!(actions.contains(&battle_core::BattleAction::Switch(1)));
    }

    #[test]
    fn side_without_request_has_no_legal_actions() {
        let data = load_gen1_demo_pack();
        let state = initialize_battle(BattleInit { player: load_demo_player_team(), opponent: load_demo_enemy_team() }, &data).unwrap();
        let state = step(state, battle_core::SideId::Player, battle_core::BattleAction::UseMove(0), RngState::seeded(11), &data)
            .unwrap()
            .state;

        assert!(legal_actions(&state, battle_core::SideId::Player).is_empty());
    }

    #[test]
    fn legality_check_reuses_legal_action_list() {
        let data = load_gen1_demo_pack();
        let state = initialize_battle(BattleInit { player: load_demo_player_team(), opponent: load_demo_enemy_team() }, &data).unwrap();

        assert!(is_legal_action(&state, battle_core::SideId::Player, battle_core::BattleAction::Switch(1)));
        assert!(!is_legal_action(&state, battle_core::SideId::Player, battle_core::BattleAction::Switch(0)));
    }

    #[test]
    fn non_choose_phase_has_no_legal_actions() {
        let data = load_gen1_demo_pack();
        let state = initialize_battle(BattleInit { player: load_demo_player_team(), opponent: load_demo_enemy_team() }, &data).unwrap();

        let actions = legal_actions_with_context(
            &state,
            battle_core::SideId::Player,
            FormatContext { phase: FormatPhase::Frozen },
        );

        assert!(actions.is_empty());
    }
}
