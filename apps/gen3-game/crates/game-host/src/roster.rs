use std::collections::BTreeSet;

use battle_application::{
    Accuracy, BattleStats, MAX_MOVES, Move, MoveId, Pokemon, PokemonId, PokemonType, TEAM_SIZE,
    Team, ValidationError,
};
use game_data::{CurrentDataSet, MoveId as DataMoveId, PokemonFormId, TypeId as DataTypeId};

const ROSTER_SIZE: usize = TEAM_SIZE * 2;
const DEMO_LEVEL: u8 = 50;
const LAST_EMERALD_POKEMON: u32 = 386;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RosterError {
    NotEnoughEligiblePokemon {
        required: usize,
        actual: usize,
    },
    NotEnoughEligibleMoves {
        pokemon: PokemonFormId,
        required: usize,
        actual: usize,
    },
    MissingPokemon(PokemonFormId),
    MissingMove(DataMoveId),
    MoveNotLearnable {
        pokemon: PokemonFormId,
        battle_move: DataMoveId,
    },
    MissingType(DataTypeId),
    UnsupportedType {
        id: DataTypeId,
        identifier: String,
    },
    MissingMovePower(DataMoveId),
    MissingMovePp(DataMoveId),
    InvalidBattleModel(ValidationError),
}

impl From<ValidationError> for RosterError {
    fn from(error: ValidationError) -> Self {
        Self::InvalidBattleModel(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RosterMember {
    pokemon_form_id: PokemonFormId,
    level: u8,
    move_ids: Vec<DataMoveId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EligiblePokemon {
    pokemon_form_id: PokemonFormId,
    move_ids: Vec<DataMoveId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DemoSpriteManifest {
    player: Vec<PokemonFormId>,
    opponent: Vec<PokemonFormId>,
}

impl DemoSpriteManifest {
    pub fn player(&self) -> &[PokemonFormId] {
        &self.player
    }

    pub fn opponent(&self) -> &[PokemonFormId] {
        &self.opponent
    }
}

pub fn demo_teams(data: &CurrentDataSet, seed: u64) -> Result<(Team, Team), RosterError> {
    let members = random_members(data, seed)?;
    Ok((
        build_team(data, "player", &members[..TEAM_SIZE])?,
        build_team(data, "rival", &members[TEAM_SIZE..])?,
    ))
}

pub fn sprite_manifest(
    data: &CurrentDataSet,
    seed: u64,
) -> Result<DemoSpriteManifest, RosterError> {
    let members = random_members(data, seed)?;
    Ok(DemoSpriteManifest {
        player: members[..TEAM_SIZE]
            .iter()
            .map(|member| member.pokemon_form_id)
            .collect(),
        opponent: members[TEAM_SIZE..]
            .iter()
            .map(|member| member.pokemon_form_id)
            .collect(),
    })
}

fn random_members(data: &CurrentDataSet, seed: u64) -> Result<Vec<RosterMember>, RosterError> {
    let mut seen_names = BTreeSet::new();
    let mut eligible = data
        .pokemon_iter()
        .filter_map(|pokemon| {
            if pokemon.id.0 > LAST_EMERALD_POKEMON
                || !pokemon.types.iter().all(|id| is_supported_type(data, *id))
                || !seen_names.insert(pokemon.display_name.localized.clone())
            {
                return None;
            }
            let move_ids = compatible_move_ids(data, pokemon.id);
            (move_ids.len() >= MAX_MOVES).then_some(EligiblePokemon {
                pokemon_form_id: pokemon.id,
                move_ids,
            })
        })
        .collect::<Vec<_>>();
    if eligible.len() < ROSTER_SIZE {
        return Err(RosterError::NotEnoughEligiblePokemon {
            required: ROSTER_SIZE,
            actual: eligible.len(),
        });
    }

    let mut rng = RosterRng::new(seed);
    rng.shuffle(&mut eligible);
    eligible
        .into_iter()
        .take(ROSTER_SIZE)
        .map(|mut pokemon| {
            rng.shuffle(&mut pokemon.move_ids);
            pokemon.move_ids.truncate(MAX_MOVES);
            if pokemon.move_ids.len() != MAX_MOVES {
                return Err(RosterError::NotEnoughEligibleMoves {
                    pokemon: pokemon.pokemon_form_id,
                    required: MAX_MOVES,
                    actual: pokemon.move_ids.len(),
                });
            }
            Ok(RosterMember {
                pokemon_form_id: pokemon.pokemon_form_id,
                level: DEMO_LEVEL,
                move_ids: pokemon.move_ids,
            })
        })
        .collect()
}

fn compatible_move_ids(data: &CurrentDataSet, pokemon: PokemonFormId) -> Vec<DataMoveId> {
    data.learnset(pokemon)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let battle_move = data.move_by_id(entry.move_id)?;
            battle_move.power.filter(|power| *power > 0)?;
            battle_move.pp.filter(|pp| *pp > 0)?;
            is_supported_type(data, battle_move.move_type).then_some(entry.move_id)
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn is_supported_type(data: &CurrentDataSet, id: DataTypeId) -> bool {
    data.type_by_id(id)
        .is_some_and(|record| is_supported_type_name(&record.identifier))
}

fn is_supported_type_name(identifier: &str) -> bool {
    matches!(
        identifier,
        "normal"
            | "fighting"
            | "flying"
            | "poison"
            | "ground"
            | "rock"
            | "bug"
            | "ghost"
            | "steel"
            | "fire"
            | "water"
            | "grass"
            | "electric"
            | "psychic"
            | "ice"
            | "dragon"
            | "dark"
    )
}

fn build_team(
    data: &CurrentDataSet,
    prefix: &str,
    members: &[RosterMember],
) -> Result<Team, RosterError> {
    let members = members
        .iter()
        .map(|member| build_pokemon(data, prefix, member))
        .collect::<Result<Vec<_>, _>>()?;
    Team::new(members).map_err(Into::into)
}

fn build_pokemon(
    data: &CurrentDataSet,
    prefix: &str,
    member: &RosterMember,
) -> Result<Pokemon, RosterError> {
    let record = data
        .pokemon(member.pokemon_form_id)
        .ok_or(RosterError::MissingPokemon(member.pokemon_form_id))?;
    let primary_type = battle_type(data, record.types[0])?;
    let secondary_type = record
        .types
        .get(1)
        .copied()
        .map(|id| battle_type(data, id))
        .transpose()?;
    let moves = member
        .move_ids
        .iter()
        .copied()
        .map(|id| {
            if !data.can_learn(member.pokemon_form_id, id) {
                return Err(RosterError::MoveNotLearnable {
                    pokemon: member.pokemon_form_id,
                    battle_move: id,
                });
            }
            battle_move(data, id)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let stats = record.base_stats;
    Pokemon::new(
        PokemonId::new(format!("{prefix}-form-{}", member.pokemon_form_id.0))?,
        &record.display_name.localized,
        member.level,
        primary_type,
        secondary_type,
        u32::from(stats.hp),
        u32::from(stats.hp),
        BattleStats::new(
            stats.attack,
            stats.defense,
            stats.special_attack,
            stats.special_defense,
            stats.speed,
        )?,
        moves,
    )
    .map_err(Into::into)
}

fn battle_move(data: &CurrentDataSet, id: DataMoveId) -> Result<Move, RosterError> {
    let record = data.move_by_id(id).ok_or(RosterError::MissingMove(id))?;
    let power = record.power.ok_or(RosterError::MissingMovePower(id))?;
    let pp = record.pp.ok_or(RosterError::MissingMovePp(id))?;
    let accuracy = record
        .accuracy
        .map(Accuracy::percent)
        .transpose()?
        .unwrap_or(Accuracy::AlwaysHit);
    Move::new(
        MoveId::new(format!("pokeapi-move-{}", id.0))?,
        &record.display_name.localized,
        battle_type(data, record.move_type)?,
        power,
        accuracy,
        pp,
        pp,
        record.priority,
    )
    .map_err(Into::into)
}

fn battle_type(data: &CurrentDataSet, id: DataTypeId) -> Result<PokemonType, RosterError> {
    let record = data.type_by_id(id).ok_or(RosterError::MissingType(id))?;
    match record.identifier.as_str() {
        "normal" => Ok(PokemonType::Normal),
        "fighting" => Ok(PokemonType::Fighting),
        "flying" => Ok(PokemonType::Flying),
        "poison" => Ok(PokemonType::Poison),
        "ground" => Ok(PokemonType::Ground),
        "rock" => Ok(PokemonType::Rock),
        "bug" => Ok(PokemonType::Bug),
        "ghost" => Ok(PokemonType::Ghost),
        "steel" => Ok(PokemonType::Steel),
        "fire" => Ok(PokemonType::Fire),
        "water" => Ok(PokemonType::Water),
        "grass" => Ok(PokemonType::Grass),
        "electric" => Ok(PokemonType::Electric),
        "psychic" => Ok(PokemonType::Psychic),
        "ice" => Ok(PokemonType::Ice),
        "dragon" => Ok(PokemonType::Dragon),
        "dark" => Ok(PokemonType::Dark),
        identifier => Err(RosterError::UnsupportedType {
            id,
            identifier: identifier.to_owned(),
        }),
    }
}

struct RosterRng {
    state: u64,
}

impl RosterRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    fn next(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.state = value;
        value.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn shuffle<T>(&mut self, values: &mut [T]) {
        for upper in (1..values.len()).rev() {
            let index = (self.next() % (upper as u64 + 1)) as usize;
            values.swap(upper, index);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use game_data::CurrentDataSet;

    use super::{MAX_MOVES, ROSTER_SIZE, demo_teams, random_members};

    #[test]
    fn seeded_roster_has_twelve_unique_pokemon_with_four_unique_learnset_moves() {
        let data = CurrentDataSet::embedded().unwrap();
        let members = random_members(&data, 0xA2B3_C4D5).unwrap();

        assert_eq!(members.len(), ROSTER_SIZE);
        assert_eq!(
            members
                .iter()
                .map(|member| member.pokemon_form_id)
                .collect::<BTreeSet<_>>()
                .len(),
            ROSTER_SIZE
        );
        for member in &members {
            assert_eq!(member.move_ids.len(), MAX_MOVES);
            assert_eq!(
                member
                    .move_ids
                    .iter()
                    .copied()
                    .collect::<BTreeSet<_>>()
                    .len(),
                MAX_MOVES
            );
            assert!(
                member
                    .move_ids
                    .iter()
                    .all(|move_id| data.can_learn(member.pokemon_form_id, *move_id))
            );
        }
    }

    #[test]
    fn equal_seeds_are_reproducible_and_different_seeds_change_the_roster() {
        let data = CurrentDataSet::embedded().unwrap();
        let first = random_members(&data, 7).unwrap();
        let repeated = random_members(&data, 7).unwrap();
        let different = random_members(&data, 8).unwrap();

        assert_eq!(first, repeated);
        assert_ne!(first, different);
    }

    #[test]
    fn generated_members_build_two_valid_battle_teams() {
        let data = CurrentDataSet::embedded().unwrap();
        let (player, opponent) = demo_teams(&data, 42).unwrap();
        let mut members = player.members().iter().chain(opponent.members());

        assert_eq!(player.members().len(), 6);
        assert_eq!(opponent.members().len(), 6);
        assert_eq!(
            members
                .clone()
                .map(|pokemon| pokemon.name())
                .collect::<BTreeSet<_>>()
                .len(),
            ROSTER_SIZE
        );
        assert!(members.all(|pokemon| pokemon.moves().len() == MAX_MOVES));
    }
}
