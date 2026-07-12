//! Pure use-case boundary for the overworld.

#![forbid(unsafe_code)]

pub use world_domain::{
    Direction, Position, Tile, WorldCommand, WorldError, WorldEvent, WorldOutcome,
};
use world_domain::{TileMap, World};

pub const DEMO_MAP_WIDTH: u16 = 16;
pub const DEMO_MAP_HEIGHT: u16 = 10;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldObservation {
    width: u16,
    height: u16,
    tiles: Vec<Tile>,
    player: Position,
    facing: Direction,
}

impl WorldObservation {
    pub const fn width(&self) -> u16 {
        self.width
    }

    pub const fn height(&self) -> u16 {
        self.height
    }

    pub fn tile(&self, position: Position) -> Option<Tile> {
        if position.x() >= self.width || position.y() >= self.height {
            return None;
        }
        Some(
            self.tiles
                [usize::from(position.y()) * usize::from(self.width) + usize::from(position.x())],
        )
    }

    pub const fn player(&self) -> Position {
        self.player
    }

    pub const fn facing(&self) -> Direction {
        self.facing
    }
}

pub struct WorldApplication {
    world: World,
}

impl WorldApplication {
    pub const fn new(world: World) -> Self {
        Self { world }
    }

    pub fn demo() -> Result<Self, WorldError> {
        let mut tiles = vec![Tile::Ground; usize::from(DEMO_MAP_WIDTH * DEMO_MAP_HEIGHT)];
        for y in 0..DEMO_MAP_HEIGHT {
            for x in 0..DEMO_MAP_WIDTH {
                let border =
                    x == 0 || y == 0 || x + 1 == DEMO_MAP_WIDTH || y + 1 == DEMO_MAP_HEIGHT;
                let grass = (6..=10).contains(&x) && (2..=7).contains(&y);
                let rocks = matches!((x, y), (3, 3) | (4, 3) | (12, 5) | (12, 6));
                let tile = if border || rocks {
                    Tile::Wall
                } else if grass {
                    Tile::Grass
                } else {
                    Tile::Ground
                };
                tiles[usize::from(y * DEMO_MAP_WIDTH + x)] = tile;
            }
        }
        let map = TileMap::new(DEMO_MAP_WIDTH, DEMO_MAP_HEIGHT, tiles)?;
        let world = World::new(map, Position::new(3, 6), Direction::Down)?;
        Ok(Self::new(world))
    }

    pub fn observe(&self) -> WorldObservation {
        WorldObservation {
            width: self.world.map().width(),
            height: self.world.map().height(),
            tiles: self.world.map().tiles().to_vec(),
            player: self.world.player(),
            facing: self.world.facing(),
        }
    }

    pub fn submit(&mut self, command: WorldCommand) -> WorldOutcome {
        self.world.submit(command)
    }
}

#[cfg(test)]
mod tests {
    use super::{Direction, Position, Tile, WorldApplication, WorldCommand};

    #[test]
    fn demo_map_exposes_a_walkable_spawn_and_nearby_grass() {
        let mut application = WorldApplication::demo().unwrap();
        let opening = application.observe();

        assert_eq!(opening.player(), Position::new(3, 6));
        assert_eq!(opening.tile(opening.player()), Some(Tile::Ground));
        assert!(
            !application
                .submit(WorldCommand::Move(Direction::Right))
                .starts_battle()
        );
        assert!(
            !application
                .submit(WorldCommand::Move(Direction::Right))
                .starts_battle()
        );
        assert!(
            application
                .submit(WorldCommand::Move(Direction::Right))
                .starts_battle()
        );
    }
}
