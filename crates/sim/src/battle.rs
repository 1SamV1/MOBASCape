//! Deterministic, IO-free match spine.
//!
//! This is the first real slice of M7: a fixed 30 Hz tick, a seeded RNG,
//! integer-only positions, command input, bot-driven 5v5 combat and a stable
//! state hash. It deliberately owns no rendering, wall-clock time or OS RNG,
//! so a headless run is reproducible on every platform.
//!
//! The client does not consume this yet. `rift-tools match` runs it headless
//! and prints the hash; the next slices move the lane, economy and structures
//! into the same module and make the client a renderer over it.

use crate::grid::Grid;
use crate::path::{find_path, simplify_path};

/// Simulation rate. Every gameplay change is quantised to this tick.
pub const TICK_HZ: u32 = 30;
pub const TICK_MS: u32 = 1000 / TICK_HZ;

/// Fixed-point scale: one tile is 1024 units. Positions are integers so the
/// simulation never depends on platform floating-point behaviour.
const POS: i32 = 1024;
const HERO_RANGE: i32 = 2 * POS;
const HERO_SPEED: i32 = 112;
const ATTACK_COOLDOWN: u16 = 24;
const HERO_HP: i32 = 600;
const HERO_DAMAGE: i32 = 18;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MatchStats {
    pub attacks: u32,
    pub crits: u32,
    pub deaths: u32,
    pub commands: u32,
}

/// Input accepted at tick boundaries. Commands are conservative: they set an
/// order for one entity, and the same command at the same tick always produces
/// the same state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    MoveTo { id: u32, x: i32, z: i32 },
    Attack { id: u32, target: u32 },
    Hold { id: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    MoveTo { x: i32, z: i32 },
    Attack { target: u32 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entity {
    pub id: u32,
    pub team: u8,
    pub x: i32,
    pub z: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub damage: i32,
    pub range: i32,
    pub cooldown: u16,
    pub order: Option<Order>,
    pub target: Option<u32>,
    pub path: Vec<(i32, i32)>,
    pub path_index: usize,
    pub path_goal: Option<(i32, i32)>,
    pub path_tick: u64,
    pub alive: bool,
}

impl Entity {
    fn new(id: u32, team: u8, tile_x: i32, tile_z: i32) -> Self {
        Self {
            id,
            team,
            x: tile_x * POS + POS / 2,
            z: tile_z * POS + POS / 2,
            hp: HERO_HP,
            max_hp: HERO_HP,
            damage: HERO_DAMAGE,
            range: HERO_RANGE,
            cooldown: 0,
            order: None,
            target: None,
            path: Vec::new(),
            path_index: 0,
            path_goal: None,
            path_tick: 0,
            alive: true,
        }
    }

    pub fn tile(&self) -> (i32, i32) {
        (self.x.div_euclid(POS), self.z.div_euclid(POS))
    }

    pub fn distance_sq(&self, other: &Entity) -> i64 {
        let dx = i64::from(other.x - self.x);
        let dz = i64::from(other.z - self.z);
        dx * dx + dz * dz
    }

    pub fn hp_fraction(&self) -> f32 {
        (self.hp.max(0) as f32 / self.max_hp.max(1) as f32).clamp(0.0, 1.0)
    }
}

struct Rng {
    state: u32,
}

impl Rng {
    fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0x9E37_79B9 } else { seed },
        }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    fn next_below(&mut self, bound: u32) -> u32 {
        self.next_u32() % bound.max(1)
    }
}

/// A deterministic headless match. `step` is exactly one 30 Hz tick.
pub struct Match {
    pub tick: u64,
    pub seed: u32,
    pub entities: Vec<Entity>,
    pub winner: Option<u8>,
    pub stats: MatchStats,
    grid: Grid,
    rng: Rng,
    commands: Vec<Command>,
}

impl Match {
    /// Build a 5v5 match on a square arena with a central wall gap and four
    /// pillars, so bot pathing has to use A* rather than a straight line.
    pub fn new(seed: u32, size: usize) -> Self {
        let size = size.max(24);
        let width = size as i32;
        let height = size as i32;
        let mut grid = Grid::new(size, size);
        let mid = width / 2;
        let gap_start = height * 2 / 5;
        let gap_end = height * 3 / 5;
        grid.block_rect(mid - 1, 2, 3, gap_start - 2);
        grid.block_rect(mid - 1, gap_end, 3, height - gap_end - 2);
        for (px, pz) in [
            (width / 4, height / 4),
            (width / 4, height * 3 / 4),
            (width * 3 / 4, height / 4),
            (width * 3 / 4, height * 3 / 4),
        ] {
            grid.block_rect(px, pz, 2, 2);
        }

        let mut entities = Vec::with_capacity(10);
        for team in 0u8..2 {
            for slot in 0..5i32 {
                let x = if team == 0 { 6 } else { width - 7 };
                let z = height / 2 - 16 + slot * 8;
                entities.push(Entity::new(entities.len() as u32, team, x, z));
            }
        }

        Self {
            tick: 0,
            seed,
            entities,
            winner: None,
            stats: MatchStats::default(),
            grid,
            rng: Rng::new(seed),
            commands: Vec::new(),
        }
    }

    pub fn enqueue(&mut self, command: Command) {
        if self.winner.is_none() {
            self.commands.push(command);
        }
    }

    pub fn alive_on(&self, team: u8) -> usize {
        self.entities
            .iter()
            .filter(|entity| entity.team == team && entity.alive)
            .count()
    }

    /// Advance exactly one tick (1/30 s).
    pub fn step(&mut self) {
        self.tick += 1;
        if self.winner.is_some() {
            return;
        }
        self.apply_commands();
        for entity in &mut self.entities {
            entity.cooldown = entity.cooldown.saturating_sub(1);
        }
        for index in 0..self.entities.len() {
            if !self.entities[index].alive {
                continue;
            }
            self.act(index);
        }
        self.check_winner();
    }

    /// Run `ticks` fixed steps (or until a winner is decided).
    pub fn run(&mut self, ticks: u64) {
        for _ in 0..ticks {
            if self.winner.is_some() {
                break;
            }
            self.step();
        }
    }

    fn apply_commands(&mut self) {
        for command in std::mem::take(&mut self.commands) {
            self.stats.commands = self.stats.commands.saturating_add(1);
            match command {
                Command::MoveTo { id, x, z } => {
                    if let Some(entity) = self.entity_mut(id) {
                        entity.order = Some(Order::MoveTo { x, z });
                        entity.path.clear();
                        entity.path_goal = None;
                        entity.target = None;
                    }
                }
                Command::Attack { id, target } => {
                    if let Some(entity) = self.entity_mut(id) {
                        entity.order = Some(Order::Attack { target });
                        entity.path.clear();
                        entity.path_goal = None;
                    }
                }
                Command::Hold { id } => {
                    if let Some(entity) = self.entity_mut(id) {
                        entity.order = None;
                        entity.path.clear();
                        entity.path_goal = None;
                        entity.target = None;
                    }
                }
            }
        }
    }

    fn entity_mut(&mut self, id: u32) -> Option<&mut Entity> {
        self.entities.get_mut(id as usize)
    }

    fn act(&mut self, index: usize) {
        let order = self.entities[index].order;
        match order {
            Some(Order::MoveTo { x, z }) => {
                if self.move_to_tile(index, (x, z)) {
                    self.entities[index].order = None;
                }
            }
            Some(Order::Attack { target }) => {
                if self.target_alive(target) {
                    self.engage(index, target as usize);
                } else {
                    self.entities[index].order = None;
                    self.entities[index].target = None;
                }
            }
            None => {
                let target = self.nearest_enemy(index);
                self.entities[index].target = target.map(|t| t as u32);
                if let Some(target) = target {
                    self.engage(index, target);
                }
            }
        }
    }

    fn target_alive(&self, target: u32) -> bool {
        self.entities
            .get(target as usize)
            .is_some_and(|entity| entity.alive && entity.team != 2)
    }

    fn nearest_enemy(&self, index: usize) -> Option<usize> {
        let entity = &self.entities[index];
        let mut best: Option<(usize, i64)> = None;
        for (other_index, other) in self.entities.iter().enumerate() {
            if other_index == index || !other.alive || other.team == entity.team {
                continue;
            }
            let distance = entity.distance_sq(other);
            if best.is_none_or(|(_, best_distance)| distance < best_distance) {
                best = Some((other_index, distance));
            }
        }
        best.map(|(index, _)| index)
    }

    fn engage(&mut self, index: usize, target: usize) {
        let range = i64::from(self.entities[index].range);
        let distance = self.entities[index].distance_sq(&self.entities[target]);
        if distance <= range * range {
            if self.entities[index].cooldown == 0 {
                self.attack(index, target);
            }
        } else {
            self.chase(index, target);
        }
    }

    fn attack(&mut self, index: usize, target: usize) {
        let crit = self.rng.next_below(100) < 10;
        let variance = self.rng.next_below(7) as i32;
        let mut damage = self.entities[index].damage + variance;
        if crit {
            damage = damage * 3 / 2;
            self.stats.crits = self.stats.crits.saturating_add(1);
        }
        self.stats.attacks = self.stats.attacks.saturating_add(1);
        self.entities[index].cooldown = ATTACK_COOLDOWN;
        let victim = &mut self.entities[target];
        victim.hp -= damage;
        if victim.hp <= 0 {
            victim.hp = 0;
            victim.alive = false;
            victim.path.clear();
            self.stats.deaths = self.stats.deaths.saturating_add(1);
        }
    }

    fn chase(&mut self, index: usize, target: usize) {
        let goal = self.entities[target].tile();
        let needs_path = {
            let entity = &self.entities[index];
            entity.path.is_empty()
                || entity.path_goal != Some(goal)
                || self.tick.saturating_sub(entity.path_tick) > 18
        };
        if needs_path {
            let start = self.entities[index].tile();
            let path = find_path(&self.grid, start, goal).unwrap_or_default();
            let path = simplify_path(&self.grid, &path);
            let entity = &mut self.entities[index];
            entity.path = path;
            entity.path_index = usize::from(entity.path.len() > 1);
            entity.path_goal = Some(goal);
            entity.path_tick = self.tick;
        }
        let next = {
            let entity = &self.entities[index];
            entity.path.get(entity.path_index).copied()
        };
        if let Some(waypoint) = next {
            if self.move_to_tile(index, waypoint) {
                self.entities[index].path_index += 1;
            }
        }
    }

    /// Move one tick toward the centre of `tile`. Returns true on arrival.
    fn move_to_tile(&mut self, index: usize, tile: (i32, i32)) -> bool {
        let target_x = tile.0 * POS + POS / 2;
        let target_z = tile.1 * POS + POS / 2;
        let entity = &self.entities[index];
        let dx = i64::from(target_x - entity.x);
        let dz = i64::from(target_z - entity.z);
        let distance_sq = dx * dx + dz * dz;
        let step = i64::from(HERO_SPEED);
        if distance_sq <= step * step {
            let entity = &mut self.entities[index];
            entity.x = target_x;
            entity.z = target_z;
            return true;
        }
        let distance = isqrt(distance_sq).max(1);
        let entity = &mut self.entities[index];
        entity.x += (dx * step / distance) as i32;
        entity.z += (dz * step / distance) as i32;
        false
    }

    fn check_winner(&mut self) {
        let blue = self.alive_on(0);
        let red = self.alive_on(1);
        self.winner = match (blue, red) {
            (0, 0) => Some(2),
            (0, _) => Some(1),
            (_, 0) => Some(0),
            _ => None,
        };
    }

    /// Stable FNV-1a over every field that can affect future ticks.
    pub fn state_hash(&self) -> u64 {
        fn mix(hash: &mut u64, value: u64) {
            *hash ^= value;
            *hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
        }
        let mut hash = 0xCBF2_9CE4_8422_2325;
        mix(&mut hash, self.tick);
        mix(&mut hash, u64::from(self.seed));
        mix(&mut hash, u64::from(self.winner.unwrap_or(9)));
        mix(&mut hash, u64::from(self.stats.attacks));
        mix(&mut hash, u64::from(self.stats.crits));
        mix(&mut hash, u64::from(self.stats.deaths));
        mix(&mut hash, u64::from(self.stats.commands));
        mix(&mut hash, u64::from(self.rng.state));
        for entity in &self.entities {
            mix(&mut hash, u64::from(entity.id));
            mix(&mut hash, u64::from(entity.team));
            mix(&mut hash, entity.x as i64 as u64);
            mix(&mut hash, entity.z as i64 as u64);
            mix(&mut hash, entity.hp as i64 as u64);
            mix(&mut hash, u64::from(entity.cooldown));
            mix(&mut hash, u64::from(entity.alive));
            mix(&mut hash, u64::from(entity.target.unwrap_or(99)));
            mix(&mut hash, u64::from(entity.path_index as u32));
            for &(x, z) in &entity.path {
                mix(&mut hash, x as i64 as u64);
                mix(&mut hash, z as i64 as u64);
            }
            match entity.order {
                None => mix(&mut hash, 0),
                Some(Order::MoveTo { x, z }) => {
                    mix(&mut hash, 1);
                    mix(&mut hash, x as i64 as u64);
                    mix(&mut hash, z as i64 as u64);
                }
                Some(Order::Attack { target }) => {
                    mix(&mut hash, 2);
                    mix(&mut hash, u64::from(target));
                }
            }
        }
        hash
    }
}

/// Integer square root, used by fixed-point movement so the sim never needs a
/// floating-point `sqrt` (and therefore never drifts between platforms).
fn isqrt(value: i64) -> i64 {
    if value < 2 {
        return value;
    }
    let mut x = value;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + value / x) / 2;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_rate_is_thirty_hz() {
        assert_eq!(TICK_HZ, 30);
        assert_eq!(TICK_MS, 33);
        let mut game = Match::new(1, 48);
        game.step();
        assert_eq!(game.tick, 1);
    }

    #[test]
    fn same_seed_and_inputs_produce_the_same_hash() {
        let mut first = Match::new(0x00C0_FFEE, 64);
        let mut second = Match::new(0x00C0_FFEE, 64);
        first.run(600);
        second.run(600);
        assert_eq!(first.state_hash(), second.state_hash());
        assert_eq!(first.winner, second.winner);
        assert_eq!(first.stats, second.stats);
    }

    #[test]
    fn commands_change_the_state_deterministically() {
        let mut first = Match::new(42, 64);
        let mut second = Match::new(42, 64);
        first.enqueue(Command::MoveTo {
            id: 0,
            x: 20,
            z: 20,
        });
        second.enqueue(Command::MoveTo {
            id: 0,
            x: 20,
            z: 20,
        });
        first.run(120);
        second.run(120);
        assert_eq!(first.state_hash(), second.state_hash());
        assert!(first.stats.commands >= 1);
    }

    #[test]
    fn a_bot_match_reaches_a_winner() {
        let mut game = Match::new(7, 64);
        for _ in 0..3_000 {
            game.step();
            if game.winner.is_some() {
                break;
            }
        }
        assert!(game.winner.is_some(), "bots should resolve the match");
        assert!(game.stats.attacks > 0);
    }

    #[test]
    fn integer_distance_and_roots_are_stable() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(1), 1);
        assert_eq!(isqrt(1024 * 1024), 1024);
        assert_eq!(isqrt(2 * 1024 * 1024), 1448);
    }
}
