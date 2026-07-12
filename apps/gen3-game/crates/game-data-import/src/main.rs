use std::{
    collections::HashMap,
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use game_data::{
    BaseStats, CurrentDataSet, DamageClass, DataSetMetadata, LocalizedName, MoveId, MoveRecord,
    PokemonFormId, PokemonRecord, SpeciesId, TypeId, TypeRecord,
};
use serde::Deserialize;

const DEFAULT_COMMIT: &str = "d638fe7791214a8d3c3282e2a3113eea7cfef288";
const SOURCE_REPOSITORY: &str = "https://github.com/PokeAPI/pokeapi";

#[derive(Debug)]
struct ImportError(String);

impl std::fmt::Display for ImportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl Error for ImportError {}

#[derive(Deserialize)]
struct LanguageRow {
    id: u16,
    identifier: String,
}
#[derive(Deserialize)]
struct StatRow {
    id: u16,
    identifier: String,
}
#[derive(Deserialize)]
struct TypeRow {
    id: u16,
    identifier: String,
}
#[derive(Deserialize)]
struct DamageClassRow {
    id: u8,
    identifier: String,
}
#[derive(Deserialize)]
struct PokemonRow {
    id: u32,
    identifier: String,
    species_id: u32,
    is_default: u8,
}
#[derive(Deserialize)]
struct PokemonStatRow {
    pokemon_id: u32,
    stat_id: u16,
    base_stat: u16,
}
#[derive(Deserialize)]
struct PokemonTypeRow {
    pokemon_id: u32,
    type_id: u16,
    slot: u8,
}
#[derive(Deserialize)]
struct SpeciesNameRow {
    pokemon_species_id: u32,
    local_language_id: u16,
    name: String,
}
#[derive(Deserialize)]
struct MoveRow {
    id: u32,
    identifier: String,
    type_id: u16,
    power: Option<u16>,
    pp: Option<u8>,
    accuracy: Option<u8>,
    priority: i8,
    damage_class_id: u8,
}
#[derive(Deserialize)]
struct MoveNameRow {
    move_id: u32,
    local_language_id: u16,
    name: String,
}
#[derive(Deserialize)]
struct TypeNameRow {
    type_id: u16,
    local_language_id: u16,
    name: String,
}

fn read_csv<T: for<'de> Deserialize<'de>>(
    source: &Path,
    name: &str,
) -> Result<Vec<T>, ImportError> {
    let path = source.join(name);
    let mut reader = csv::Reader::from_path(&path)
        .map_err(|error| ImportError(format!("cannot open {}: {error}", path.display())))?;
    reader
        .deserialize()
        .enumerate()
        .map(|(index, row)| {
            row.map_err(|error| ImportError(format!("invalid {name} row {}: {error}", index + 2)))
        })
        .collect()
}

fn localized(localized: Option<String>, english: Option<String>, fallback: &str) -> LocalizedName {
    let english = english.unwrap_or_else(|| fallback.to_owned());
    LocalizedName {
        localized: localized.unwrap_or_else(|| english.clone()),
        english,
    }
}

fn names<I>(rows: I, language_id: u16) -> HashMap<u32, (Option<String>, Option<String>)>
where
    I: IntoIterator<Item = (u32, u16, String)>,
{
    let mut result: HashMap<u32, (Option<String>, Option<String>)> = HashMap::new();
    for (id, lang, name) in rows {
        let entry = result.entry(id).or_default();
        if lang == language_id {
            entry.0 = Some(name.clone());
        }
        if lang == 9 {
            entry.1 = Some(name);
        }
    }
    result
}

fn parse_args() -> Result<(PathBuf, PathBuf, String, String), ImportError> {
    let mut source = None;
    let mut output = None;
    let mut locale = "zh-Hans".to_owned();
    let mut commit = DEFAULT_COMMIT.to_owned();
    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| ImportError(format!("missing value for {flag}")))?;
        match flag.as_str() {
            "--source" => source = Some(PathBuf::from(value)),
            "--output" => output = Some(PathBuf::from(value)),
            "--locale" => locale = value,
            "--source-commit" => commit = value,
            "--help" | "-h" => return Err(ImportError("usage: game-data-import --source DIR --output FILE [--locale zh-Hans] [--source-commit SHA]".into())),
            _ => return Err(ImportError(format!("unknown argument: {flag}"))),
        }
    }
    Ok((
        source.ok_or_else(|| ImportError("--source is required".into()))?,
        output.ok_or_else(|| ImportError("--output is required".into()))?,
        locale,
        commit,
    ))
}

fn import(source: &Path, locale: &str, commit: &str) -> Result<CurrentDataSet, ImportError> {
    if locale != "zh-Hans" {
        return Err(ImportError(format!("unsupported locale: {locale}")));
    }
    let language_id = read_csv::<LanguageRow>(source, "languages.csv")?
        .into_iter()
        .find(|row| row.identifier == "zh-hans")
        .map(|row| row.id)
        .ok_or_else(|| ImportError("zh-hans language is missing".into()))?;
    let stats = read_csv::<StatRow>(source, "stats.csv")?;
    let stat_ids: HashMap<_, _> = stats
        .into_iter()
        .map(|row| (row.identifier, row.id))
        .collect();
    let required_stats = [
        "hp",
        "attack",
        "defense",
        "special-attack",
        "special-defense",
        "speed",
    ];
    if required_stats
        .iter()
        .any(|name| !stat_ids.contains_key(*name))
    {
        return Err(ImportError("required battle stat is missing".into()));
    }

    let type_rows = read_csv::<TypeRow>(source, "types.csv")?;
    let type_ids: HashMap<_, _> = type_rows
        .iter()
        .map(|row| (row.identifier.clone(), row.id))
        .collect();
    let type_names = names(
        read_csv::<TypeNameRow>(source, "type_names.csv")?
            .into_iter()
            .map(|row| (row.type_id as u32, row.local_language_id, row.name)),
        language_id,
    );
    let types = type_rows
        .into_iter()
        .map(|row| {
            let (localized_name, english_name) = type_names
                .get(&(row.id as u32))
                .cloned()
                .unwrap_or_default();
            TypeRecord {
                id: TypeId(row.id),
                identifier: row.identifier.clone(),
                display_name: localized(localized_name, english_name, &row.identifier),
            }
        })
        .collect::<Vec<_>>();
    let damage_classes: HashMap<_, _> =
        read_csv::<DamageClassRow>(source, "move_damage_classes.csv")?
            .into_iter()
            .map(|row| (row.id, row.identifier))
            .collect();

    let species_names = names(
        read_csv::<SpeciesNameRow>(source, "pokemon_species_names.csv")?
            .into_iter()
            .map(|row| (row.pokemon_species_id, row.local_language_id, row.name)),
        language_id,
    );
    let move_names = names(
        read_csv::<MoveNameRow>(source, "move_names.csv")?
            .into_iter()
            .map(|row| (row.move_id, row.local_language_id, row.name)),
        language_id,
    );
    let mut stats_by_pokemon: HashMap<u32, [Option<u16>; 6]> = HashMap::new();
    for row in read_csv::<PokemonStatRow>(source, "pokemon_stats.csv")? {
        let slot = required_stats
            .iter()
            .position(|name| stat_ids.get(*name) == Some(&row.stat_id));
        if let Some(slot) = slot {
            let entry = stats_by_pokemon.entry(row.pokemon_id).or_default();
            if entry[slot].replace(row.base_stat).is_some() {
                return Err(ImportError(format!(
                    "pokemon {} repeats stat {}",
                    row.pokemon_id, row.stat_id
                )));
            }
        }
    }
    let mut types_by_pokemon: HashMap<u32, Vec<(u8, TypeId)>> = HashMap::new();
    for row in read_csv::<PokemonTypeRow>(source, "pokemon_types.csv")? {
        if !type_ids.values().any(|id| *id == row.type_id) {
            return Err(ImportError(format!(
                "pokemon {} references unknown type {}",
                row.pokemon_id, row.type_id
            )));
        }
        if !(1..=2).contains(&row.slot) {
            return Err(ImportError(format!(
                "pokemon {} has invalid type slot {}",
                row.pokemon_id, row.slot
            )));
        }
        let entry = types_by_pokemon.entry(row.pokemon_id).or_default();
        if entry.iter().any(|(slot, _)| *slot == row.slot) {
            return Err(ImportError(format!(
                "pokemon {} repeats type slot {}",
                row.pokemon_id, row.slot
            )));
        }
        entry.push((row.slot, TypeId(row.type_id)));
    }
    let pokemon = read_csv::<PokemonRow>(source, "pokemon.csv")?
        .into_iter()
        .map(|row| {
            let stats = stats_by_pokemon
                .remove(&row.id)
                .ok_or_else(|| ImportError(format!("pokemon {} has no stats", row.id)))?;
            let [
                Some(hp),
                Some(attack),
                Some(defense),
                Some(special_attack),
                Some(special_defense),
                Some(speed),
            ] = stats
            else {
                return Err(ImportError(format!(
                    "pokemon {} is missing a battle stat",
                    row.id
                )));
            };
            let mut types = types_by_pokemon
                .remove(&row.id)
                .ok_or_else(|| ImportError(format!("pokemon {} has no types", row.id)))?;
            types.sort_by_key(|(slot, _)| *slot);
            let (localized_name, english_name) = species_names
                .get(&row.species_id)
                .cloned()
                .unwrap_or_default();
            Ok(PokemonRecord {
                id: PokemonFormId(row.id),
                species_id: SpeciesId(row.species_id),
                identifier: row.identifier.clone(),
                is_default: row.is_default != 0,
                base_stats: BaseStats {
                    hp,
                    attack,
                    defense,
                    special_attack,
                    special_defense,
                    speed,
                },
                types: types.into_iter().map(|(_, id)| id).collect(),
                display_name: localized(localized_name, english_name, &row.identifier),
            })
        })
        .collect::<Result<Vec<_>, ImportError>>()?;

    let moves = read_csv::<MoveRow>(source, "moves.csv")?
        .into_iter()
        .map(|row| {
            let (localized_name, english_name) =
                move_names.get(&row.id).cloned().unwrap_or_default();
            let damage_class = match damage_classes.get(&row.damage_class_id).map(String::as_str) {
                Some("physical") => DamageClass::Physical,
                Some("special") => DamageClass::Special,
                Some("status") => DamageClass::Status,
                Some(other) => {
                    return Err(ImportError(format!(
                        "move {} has unknown damage class {other}",
                        row.id
                    )));
                }
                None => {
                    return Err(ImportError(format!(
                        "move {} references unknown damage class {}",
                        row.id, row.damage_class_id
                    )));
                }
            };
            if !type_ids.values().any(|id| *id == row.type_id) {
                return Err(ImportError(format!(
                    "move {} references unknown type {}",
                    row.id, row.type_id
                )));
            }
            Ok(MoveRecord {
                id: MoveId(row.id),
                identifier: row.identifier.clone(),
                display_name: localized(localized_name, english_name, &row.identifier),
                move_type: TypeId(row.type_id),
                power: row.power.filter(|value| *value != 0),
                accuracy: row.accuracy.filter(|value| *value != 0),
                pp: row.pp.filter(|value| *value != 0),
                priority: row.priority,
                damage_class,
            })
        })
        .collect::<Result<Vec<_>, ImportError>>()?;

    CurrentDataSet::new(
        DataSetMetadata {
            schema_version: "current-data-set-v1".into(),
            source_repository: SOURCE_REPOSITORY.into(),
            source_commit: commit.into(),
            generator_version: "game-data-import-0.0.0".into(),
            locale: locale.into(),
        },
        pokemon,
        moves,
        types,
    )
    .map_err(|error| ImportError(format!("imported data failed validation: {error}")))
}

fn main() -> Result<(), Box<dyn Error>> {
    let (source, output, locale, commit) = parse_args()?;
    let dataset = import(&source, &locale, &commit)?;
    let bytes = serde_json::to_vec_pretty(&dataset)?;
    CurrentDataSet::from_json(&bytes)
        .map_err(|error| ImportError(format!("generated output failed validation: {error}")))?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = output.with_extension("json.tmp");
    fs::write(&temp, bytes)?;
    if output.exists() {
        fs::remove_file(&output)?;
    }
    fs::rename(temp, output)?;
    Ok(())
}
