use crate::defs::{MoveDef, SpeciesDef};
use crate::ids::{MoveId, SpeciesId};
use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct DataPack {
    pub id: String,
    species: Box<[SpeciesDef]>,
    moves: Box<[MoveDef]>,
}

#[derive(Clone, Debug, Deserialize)]
struct DataPackDocument {
    id: String,
    species: Vec<SpeciesDef>,
    moves: Vec<MoveDef>,
}

impl DataPack {
    pub fn new(id: impl Into<String>, species: Vec<SpeciesDef>, moves: Vec<MoveDef>) -> Self {
        Self {
            id: id.into(),
            species: species.into_boxed_slice(),
            moves: moves.into_boxed_slice(),
        }
    }

    pub fn from_json_str(input: &str) -> serde_json::Result<Self> {
        let doc: DataPackDocument = serde_json::from_str(input)?;
        Ok(Self::new(doc.id, doc.species, doc.moves))
    }

    pub fn species(&self, id: SpeciesId) -> &SpeciesDef {
        &self.species[id.0]
    }

    pub fn move_def(&self, id: MoveId) -> &MoveDef {
        &self.moves[id.0]
    }
}
