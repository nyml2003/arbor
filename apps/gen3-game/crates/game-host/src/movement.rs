use std::time::{Duration, Instant};

use game_ui::WorldAnimation;
use punctum_gpu::{PixelOffset, PixelSize};
use world_application::Direction;

pub const WORLD_TICK_INTERVAL: Duration = Duration::from_millis(16);
pub const RUN_STOP_DURATION: Duration = Duration::from_millis(90);
const SETTLE_DURATION: Duration = Duration::from_millis(50);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gait {
    Walk,
    Run,
}

impl Gait {
    const fn duration(self) -> Duration {
        match self {
            Self::Walk => Duration::from_millis(240),
            Self::Run => Duration::from_millis(150),
        }
    }

    const fn frame_interval(self) -> Duration {
        match self {
            Self::Walk => Duration::from_millis(60),
            Self::Run => Duration::from_millis(40),
        }
    }

    pub const fn animation(self) -> WorldAnimation {
        match self {
            Self::Walk => WorldAnimation::Walk,
            Self::Run => WorldAnimation::Run,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldMotion {
    direction: Direction,
    gait: Gait,
    started_at: Instant,
    settling: Option<Settling>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Settling {
    started_at: Instant,
    remaining_at_start: Duration,
}

impl WorldMotion {
    pub const fn new(direction: Direction, gait: Gait, started_at: Instant) -> Self {
        Self {
            direction,
            gait,
            started_at,
            settling: None,
        }
    }

    pub const fn direction(self) -> Direction {
        self.direction
    }

    pub const fn gait(self) -> Gait {
        self.gait
    }

    pub fn is_complete(self, now: Instant) -> bool {
        self.remaining(now).is_zero()
    }

    pub fn settle(&mut self, now: Instant) {
        if self.settling.is_none() {
            self.settling = Some(Settling {
                started_at: now,
                remaining_at_start: self.remaining(now),
            });
        }
    }

    pub fn sprite_frame(self, now: Instant) -> usize {
        let elapsed = now.saturating_duration_since(self.started_at);
        (elapsed.as_millis() / self.gait.frame_interval().as_millis()) as usize
    }

    pub fn pixel_offset(self, now: Instant, cell_size: PixelSize) -> PixelOffset {
        let duration = self.gait.duration();
        let remaining = self.remaining(now);
        let horizontal = remaining_pixels(cell_size.width.saturating_mul(2), remaining, duration);
        let vertical = remaining_pixels(cell_size.height.saturating_mul(2), remaining, duration);
        match self.direction {
            Direction::Up => PixelOffset::new(0, vertical),
            Direction::Down => PixelOffset::new(0, -vertical),
            Direction::Left => PixelOffset::new(horizontal, 0),
            Direction::Right => PixelOffset::new(-horizontal, 0),
        }
    }

    fn remaining(self, now: Instant) -> Duration {
        if let Some(settling) = self.settling {
            let elapsed = now
                .saturating_duration_since(settling.started_at)
                .min(SETTLE_DURATION);
            let settle_remaining = SETTLE_DURATION - elapsed;
            let nanos = settling.remaining_at_start.as_nanos() * settle_remaining.as_nanos()
                / SETTLE_DURATION.as_nanos();
            return Duration::from_nanos(nanos as u64);
        }
        let duration = self.gait.duration();
        duration - now.saturating_duration_since(self.started_at).min(duration)
    }
}

fn remaining_pixels(extent: u32, remaining: Duration, duration: Duration) -> i32 {
    let pixels = u128::from(extent) * remaining.as_nanos() / duration.as_nanos();
    pixels.min(i32::MAX as u128) as i32
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PressedDirections {
    pressed_at: [Option<u64>; 4],
    sequence: u64,
}

impl PressedDirections {
    pub fn press(&mut self, direction: Direction) {
        let index = direction_index(direction);
        if self.pressed_at[index].is_none() {
            self.sequence = self.sequence.wrapping_add(1);
            self.pressed_at[index] = Some(self.sequence);
        }
    }

    pub fn release(&mut self, direction: Direction) {
        self.pressed_at[direction_index(direction)] = None;
    }

    pub fn clear(&mut self) {
        self.pressed_at = [None; 4];
    }

    pub fn active(&self) -> Option<Direction> {
        self.pressed_at
            .iter()
            .enumerate()
            .filter_map(|(index, sequence)| sequence.map(|sequence| (index, sequence)))
            .max_by_key(|(_, sequence)| *sequence)
            .map(|(index, _)| direction_from_index(index))
    }
}

const fn direction_index(direction: Direction) -> usize {
    match direction {
        Direction::Up => 0,
        Direction::Down => 1,
        Direction::Left => 2,
        Direction::Right => 3,
    }
}

const fn direction_from_index(index: usize) -> Direction {
    match index {
        0 => Direction::Up,
        1 => Direction::Down,
        2 => Direction::Left,
        _ => Direction::Right,
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use punctum_gpu::{PixelOffset, PixelSize};
    use world_application::Direction;

    use super::{Gait, PressedDirections, WorldMotion};

    #[test]
    fn most_recent_held_direction_wins_and_release_restores_the_previous_one() {
        let mut pressed = PressedDirections::default();
        pressed.press(Direction::Left);
        pressed.press(Direction::Up);
        assert_eq!(pressed.active(), Some(Direction::Up));

        pressed.release(Direction::Up);
        assert_eq!(pressed.active(), Some(Direction::Left));
        pressed.clear();
        assert_eq!(pressed.active(), None);
    }

    #[test]
    fn movement_uses_integer_offsets_from_the_previous_tile_to_the_target() {
        let started = Instant::now();
        let motion = WorldMotion::new(Direction::Right, Gait::Walk, started);
        let cell = PixelSize::new(30, 30);

        assert_eq!(motion.pixel_offset(started, cell), PixelOffset::new(-60, 0));
        assert_eq!(
            motion.pixel_offset(started + Duration::from_millis(120), cell),
            PixelOffset::new(-30, 0)
        );
        assert_eq!(
            motion.pixel_offset(started + Duration::from_millis(240), cell),
            PixelOffset::new(0, 0)
        );
        assert!(!motion.is_complete(started + Duration::from_millis(239)));
        assert!(motion.is_complete(started + Duration::from_millis(240)));
    }

    #[test]
    fn walk_and_run_advance_frames_at_their_own_fixed_rates() {
        let started = Instant::now();
        let walk = WorldMotion::new(Direction::Down, Gait::Walk, started);
        let run = WorldMotion::new(Direction::Down, Gait::Run, started);

        assert_eq!(walk.sprite_frame(started + Duration::from_millis(120)), 2);
        assert_eq!(run.sprite_frame(started + Duration::from_millis(80)), 2);
    }

    #[test]
    fn key_release_settles_the_remaining_distance_within_fifty_milliseconds() {
        let started = Instant::now();
        let mut motion = WorldMotion::new(Direction::Right, Gait::Walk, started);
        let released = started + Duration::from_millis(60);
        motion.settle(released);

        assert_eq!(
            motion.pixel_offset(released, PixelSize::new(30, 30)),
            PixelOffset::new(-45, 0)
        );
        assert!(!motion.is_complete(released + Duration::from_millis(49)));
        assert!(motion.is_complete(released + Duration::from_millis(50)));
        assert_eq!(
            motion.pixel_offset(released + Duration::from_millis(50), PixelSize::new(30, 30)),
            PixelOffset::new(0, 0)
        );
    }
}
