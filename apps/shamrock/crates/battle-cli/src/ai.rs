use battle_core::{BattleAction, BattleState, SideId};
use battle_data::{DataPack, type_modifier};
use battle_format::legal_actions;

/**
当前 demo 的敌方 AI。

它只在合法动作里挑一个“本回合看起来最赚”的动作。
目的不是做强 AI，而是给最小可玩流程提供一个稳定对手。
*/
pub(crate) fn choose_enemy_action(state: &BattleState, data: &DataPack) -> BattleAction {
    let actions = legal_actions(state, SideId::Opponent);
    actions
        .into_iter()
        .max_by_key(|action| estimate_action_score(*action, state, SideId::Opponent, data))
        .unwrap_or(BattleAction::UseMove(0))
}

/**
给动作打一个非常粗糙的启发式分数。

这个分数只服务 demo AI，不是正式对战评估函数。
它优先考虑即时收益，所以天然偏向直接伤害和属性优势。
*/
fn estimate_action_score(
    action: BattleAction,
    state: &BattleState,
    side: SideId,
    data: &DataPack,
) -> i32 {
    match action {
        BattleAction::Switch(_) => 0,
        BattleAction::UseMove(index) => {
            let attacker = &state.teams[side.index()].party[state.teams[side.index()].active];
            let defender =
                &state.teams[side.foe().index()].party[state.teams[side.foe().index()].active];
            let move_def = data.move_def(attacker.moves[index]);
            let mut score = i32::from(move_def.power);

            let attacker_species = data.species(attacker.species);
            if move_def.element_type == attacker_species.primary_type
                || Some(move_def.element_type) == attacker_species.secondary_type
            {
                score += 20;
            }

            let defender_species = data.species(defender.species);
            score = score * i32::from(type_modifier(
                move_def.element_type,
                defender_species.primary_type,
            )) / 100;
            score + i32::from(move_def.priority) * 10
        }
    }
}
