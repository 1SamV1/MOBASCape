//! Deterministic, IO-free match spine.
//!
//! Fixed 30 Hz tick, seeded RNG, integer fixed-point positions, three lanes,
//! minion waves, towers/inhibitors/nexus, hero economy and a stable state
//! hash. No wall-clock time, OS RNG or floating point is allowed to affect
//! gameplay, so a headless run is reproducible on every platform.
//!
//! `rift-tools match` runs it without opening the cache. The client still
//! renders its own prototype match; the next slice replaces that with this
//! state.

use std::array;

use crate::grid::Grid;
use crate::path::{find_path, simplify_path};

/// Simulation rate. Every gameplay change is quantised to this tick.
pub const TICK_HZ: u32 = 30;
pub const TICK_MS: u32 = 1000 / TICK_HZ;

/// Fixed-point scale: one tile is 1024 units.
const POS: i32 = 1024;
const HERO_RANGE: i32 = 2 * POS;
const HERO_SPEED: i32 = 112;
const HERO_HP: i32 = 600;
const HERO_DAMAGE: i32 = 18;
const HERO_ATTACK_COOLDOWN: u16 = 24;
const HERO_AGGRO: i32 = 8 * POS;
const HERO_RESPAWN: u16 = 15 * TICK_HZ as u16;

const MINION_RANGE: i32 = POS;
const MINION_SPEED: i32 = 82;
const MINION_HP: i32 = 150;
const MINION_DAMAGE: i32 = 8;
const MINION_ATTACK_COOLDOWN: u16 = 30;
const MINION_AGGRO: i32 = 3 * POS;

const TOWER_HP: i32 = 1100;
const TOWER_DAMAGE: i32 = 34;
const TOWER_RANGE: i32 = 5 * POS;
const INHIBITOR_HP: i32 = 900;
const INHIBITOR_DAMAGE: i32 = 24;
const INHIBITOR_RANGE: i32 = 4 * POS;
const NEXUS_HP: i32 = 2000;
const NEXUS_DAMAGE: i32 = 28;
const NEXUS_RANGE: i32 = 5 * POS;
const STRUCTURE_COOLDOWN: u16 = 30;

const WAVE_INTERVAL: u16 = 25 * TICK_HZ as u16;
const FIRST_WAVE: u16 = 10 * TICK_HZ as u16;
const MAX_MINIONS_PER_TEAM: usize = 90;
const MAX_MATCH_TICKS: u64 = 20 * 60 * TICK_HZ as u64;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MatchStats {
    pub attacks: u32,
    pub crits: u32,
    pub deaths: u32,
    pub commands: u32,
    pub waves: u32,
}

/// Input accepted at tick boundaries.
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityKind {
    Hero,
    Minion,
    Tower,
    Inhibitor,
    Nexus,
}

impl EntityKind {
    fn is_structure(self) -> bool {
        matches!(self, Self::Tower | Self::Inhibitor | Self::Nexus)
    }

    fn is_hero(self) -> bool {
        matches!(self, Self::Hero)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entity {
    pub id: u32,
    pub kind: EntityKind,
    pub team: u8,
    pub lane: u8,
    pub x: i32,
    pub z: i32,
    pub home_x: i32,
    pub home_z: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub damage: i32,
    pub range: i32,
    pub speed: i32,
    pub attack_cooldown: u16,
    pub cooldown: u16,
    pub level: u32,
    pub xp: u32,
    pub gold: u32,
    pub respawn: u16,
    pub order: Option<Order>,
    pub target: Option<u32>,
    pub path: Vec<(i32, i32)>,
    pub path_index: usize,
    pub path_goal: Option<(i32, i32)>,
    pub path_tick: u64,
    /// Current waypoint in the entity's lane, and the direction to advance.
    pub lane_index: usize,
    pub lane_dir: i8,
    pub alive: bool,
}

impl Entity {
    #[allow(clippy::too_many_arguments)]
    fn base(
        id: u32,
        kind: EntityKind,
        team: u8,
        lane: u8,
        tile: (i32, i32),
        hp: i32,
        damage: i32,
        range: i32,
        speed: i32,
        attack_cooldown: u16,
    ) -> Self {
        Self {
            id,
            kind,
            team,
            lane,
            x: tile.0 * POS + POS / 2,
            z: tile.1 * POS + POS / 2,
            home_x: tile.0 * POS + POS / 2,
            home_z: tile.1 * POS + POS / 2,
            hp,
            max_hp: hp,
            damage,
            range,
            speed,
            attack_cooldown,
            cooldown: 0,
            level: 1,
            xp: 0,
            gold: 0,
            respawn: 0,
            order: None,
            target: None,
            path: Vec::new(),
            path_index: 0,
            path_goal: None,
            path_tick: 0,
            lane_index: 0,
            lane_dir: if team == 0 { 1 } else { -1 },
            alive: true,
        }
    }

    fn hero(id: u32, team: u8, lane: u8, tile: (i32, i32), lane_index: usize) -> Self {
        let mut entity = Self::base(
            id,
            EntityKind::Hero,
            team,
            lane,
            tile,
            HERO_HP,
            HERO_DAMAGE,
            HERO_RANGE,
            HERO_SPEED,
            HERO_ATTACK_COOLDOWN,
        );
        entity.lane_index = lane_index;
        entity
    }

    fn minion(id: u32, team: u8, lane: u8, tile: (i32, i32), lane_index: usize) -> Self {
        let mut entity = Self::base(
            id,
            EntityKind::Minion,
            team,
            lane,
            tile,
            MINION_HP,
            MINION_DAMAGE,
            MINION_RANGE,
            MINION_SPEED,
            MINION_ATTACK_COOLDOWN,
        );
        entity.lane_index = lane_index;
        entity
    }

    fn structure(
        id: u32,
        kind: EntityKind,
        team: u8,
        tile: (i32, i32),
        hp: i32,
        damage: i32,
        range: i32,
    ) -> Self {
        Self::base(
            id,
            kind,
            team,
            0,
            tile,
            hp,
            damage,
            range,
            0,
            STRUCTURE_COOLDOWN,
        )
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
    /// State hash after the most recent step, for per-tick verification.
    pub last_tick_hash: u64,
    pub lanes: [Vec<(i32, i32)>; 3],
    wave_timer: u16,
    wave_serial: u32,
    grid: Grid,
    rng: Rng,
    commands: Vec<Command>,
}

impl Match {
    /// Build a 5v5 match on a three-lane arena with jungle blocks. Coordinates
    /// are authored for a 64x64 arena and scaled to `size`.
    pub fn new(seed: u32, size: usize) -> Self {
        let size = size.max(32) as i32;
        let scale = |x: i32, z: i32| (x * size / 64, z * size / 64);
        let mut grid = Grid::new(size as usize, size as usize);
        // Jungle blocks between the lanes. The three lane lines stay clear.
        for (x, z, w, h) in [
            (22, 17, 4, 9),
            (38, 17, 4, 9),
            (22, 38, 4, 9),
            (38, 38, 4, 9),
            (28, 20, 8, 3),
            (28, 41, 8, 3),
        ] {
            let (sx, sz) = scale(x, z);
            let (sw, sh) = scale(w, h);
            grid.block_rect(sx, sz, sw.max(1), sh.max(1));
        }

        let lanes = array::from_fn(|lane| {
            let z = [8, 32, 56][lane];
            vec![
                scale(8, z),
                scale(18, z),
                scale(32, z),
                scale(46, z),
                scale(56, z),
            ]
        });

        let mut entities = Vec::new();
        for team in 0u8..2 {
            let lane_order = [0u8, 1, 2, 1, 2];
            for (slot, &lane) in lane_order.iter().enumerate() {
                let tile = scale(6, 28 + slot as i32 * 2);
                let lane_index = if team == 0 {
                    0
                } else {
                    lanes[lane as usize].len() - 1
                };
                let id = entities.len() as u32;
                entities.push(Entity::hero(id, team, lane, tile, lane_index));
            }
        }

        // Structures: the lane order is top, mid, bottom.
        for team in 0u8..2 {
            let (nexus_x, tower_x, inhibitor_x, zs) = if team == 0 {
                (7, 20, 13, [10, 32, 54])
            } else {
                (56, 43, 50, [10, 32, 54])
            };
            let id = entities.len() as u32;
            entities.push(Entity::structure(
                id,
                EntityKind::Nexus,
                team,
                scale(nexus_x, 32),
                NEXUS_HP,
                NEXUS_DAMAGE,
                NEXUS_RANGE,
            ));
            for (lane, z) in zs.into_iter().enumerate() {
                let id = entities.len() as u32;
                entities.push(Entity::structure(
                    id,
                    EntityKind::Tower,
                    team,
                    scale(tower_x, z),
                    TOWER_HP,
                    TOWER_DAMAGE,
                    TOWER_RANGE,
                ));
                let id = entities.len() as u32;
                entities.push(Entity::structure(
                    id,
                    EntityKind::Inhibitor,
                    team,
                    scale(inhibitor_x, z),
                    INHIBITOR_HP,
                    INHIBITOR_DAMAGE,
                    INHIBITOR_RANGE,
                ));
                let _ = lane;
            }
        }

        let mut game = Self {
            tick: 0,
            seed,
            entities,
            winner: None,
            stats: MatchStats::default(),
            last_tick_hash: 0,
            lanes,
            wave_timer: FIRST_WAVE,
            wave_serial: 0,
            grid,
            rng: Rng::new(seed),
            commands: Vec::new(),
        };
        game.last_tick_hash = game.state_hash();
        game
    }

    pub fn enqueue(&mut self, command: Command) {
        if self.winner.is_none() {
            self.commands.push(command);
        }
    }

    pub fn alive_on(&self, team: u8) -> usize {
        self.entities
            .iter()
            .filter(|entity| entity.kind.is_hero() && entity.team == team && entity.alive)
            .count()
    }

    pub fn nexus(&self, team: u8) -> Option<&Entity> {
        self.entities
            .iter()
            .find(|entity| entity.kind == EntityKind::Nexus && entity.team == team)
    }

    pub fn nexus_vulnerable(&self, team: u8) -> bool {
        self.entities
            .iter()
            .filter(|entity| {
                entity.kind == EntityKind::Inhibitor && entity.team != team && !entity.alive
            })
            .count()
            > 0
    }

    /// Advance exactly one tick (1/30 s).
    pub fn step(&mut self) {
        self.tick += 1;
        if self.winner.is_some() {
            self.last_tick_hash = self.state_hash();
            return;
        }
        self.apply_commands();
        self.tick_respawns();
        for entity in &mut self.entities {
            entity.cooldown = entity.cooldown.saturating_sub(1);
        }
        self.spawn_waves();
        for index in 0..self.entities.len() {
            if !self.entities[index].alive {
                continue;
            }
            self.act(index);
        }
        self.check_winner();
        self.last_tick_hash = self.state_hash();
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

    fn tick_respawns(&mut self) {
        for index in 0..self.entities.len() {
            if self.entities[index].kind.is_hero() && !self.entities[index].alive {
                self.entities[index].respawn = self.entities[index].respawn.saturating_sub(1);
                if self.entities[index].respawn > 0 {
                    continue;
                }
                let team = self.entities[index].team;
                let lane = self.entities[index].lane as usize;
                let lane_index = if team == 0 {
                    0
                } else {
                    self.lanes.get(lane).map_or(0, |points| points.len() - 1)
                };
                let entity = &mut self.entities[index];
                entity.alive = true;
                entity.hp = entity.max_hp;
                entity.x = entity.home_x;
                entity.z = entity.home_z;
                entity.path.clear();
                entity.path_goal = None;
                entity.target = None;
                entity.order = None;
                entity.lane_index = lane_index;
                entity.cooldown = 0;
            }
        }
    }

    fn spawn_waves(&mut self) {
        self.wave_timer = self.wave_timer.saturating_sub(1);
        if self.wave_timer > 0 {
            return;
        }
        self.wave_timer = WAVE_INTERVAL;
        self.wave_serial = self.wave_serial.wrapping_add(1);
        self.stats.waves = self.stats.waves.saturating_add(1);
        for team in 0u8..2 {
            let count = self
                .entities
                .iter()
                .filter(|entity| entity.kind == EntityKind::Minion && entity.team == team)
                .count();
            if count >= MAX_MINIONS_PER_TEAM {
                continue;
            }
            for lane in 0u8..3 {
                let lane_points = self.lanes[lane as usize].clone();
                let (start, lane_index) = if team == 0 {
                    (lane_points[0], 1)
                } else {
                    (lane_points[lane_points.len() - 1], lane_points.len() - 2)
                };
                for slot in -1i32..=1 {
                    let tile = (start.0, start.1 + slot);
                    let id = self.entities.len() as u32;
                    self.entities
                        .push(Entity::minion(id, team, lane, tile, lane_index));
                }
            }
        }
    }

    fn act(&mut self, index: usize) {
        let (kind, order) = (self.entities[index].kind, self.entities[index].order);
        if kind.is_structure() {
            self.act_structure(index);
            return;
        }
        match order {
            Some(Order::MoveTo { x, z }) => {
                if self.move_to_tile(index, (x, z)) {
                    self.entities[index].order = None;
                }
            }
            Some(Order::Attack { target }) => {
                if self.can_attack_target(index, target as usize) {
                    self.engage(index, target as usize);
                } else {
                    self.entities[index].order = None;
                    self.entities[index].target = None;
                }
            }
            None => {
                let aggro = if kind.is_hero() {
                    HERO_AGGRO
                } else {
                    MINION_AGGRO
                };
                let target = self.nearest_targetable_enemy(index, aggro);
                self.entities[index].target = target.map(|t| t as u32);
                if let Some(target) = target {
                    self.engage(index, target);
                } else {
                    self.advance_lane(index);
                }
            }
        }
    }

    fn act_structure(&mut self, index: usize) {
        let range = self.entities[index].range;
        if let Some(target) = self.nearest_targetable_enemy(index, range) {
            self.entities[index].target = Some(target as u32);
            if self.entities[index].cooldown == 0 {
                self.attack(index, target);
            }
        } else {
            self.entities[index].target = None;
        }
    }

    fn can_attack_target(&self, index: usize, target: usize) -> bool {
        let Some(victim) = self.entities.get(target) else {
            return false;
        };
        if !victim.alive || victim.team == self.entities[index].team {
            return false;
        }
        if victim.kind == EntityKind::Nexus && !self.nexus_vulnerable(victim.team) {
            return false;
        }
        true
    }

    /// Nearest living enemy in range, excluding shielded nexuses.
    fn nearest_targetable_enemy(&self, index: usize, range: i32) -> Option<usize> {
        let entity = &self.entities[index];
        let range_sq = i64::from(range) * i64::from(range);
        let mut best: Option<(usize, i64)> = None;
        for (other_index, other) in self.entities.iter().enumerate() {
            if other_index == index || !other.alive || other.team == entity.team {
                continue;
            }
            if other.kind == EntityKind::Nexus && !self.nexus_vulnerable(other.team) {
                continue;
            }
            let distance = entity.distance_sq(other);
            if distance > range_sq {
                continue;
            }
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
        } else if self.entities[index].kind.is_hero()
            || self.entities[index].kind == EntityKind::Minion
        {
            self.chase(index, target);
        }
    }

    fn attack(&mut self, index: usize, target: usize) {
        let attacker_team = self.entities[index].team;
        let attacker_is_hero = self.entities[index].kind.is_hero();
        let target_kind = self.entities[target].kind;
        if target_kind == EntityKind::Nexus && !self.nexus_vulnerable(self.entities[target].team) {
            return;
        }
        let crit = self.rng.next_below(100) < 10;
        let variance = self.rng.next_below(7) as i32;
        let mut damage = self.entities[index].damage + variance;
        if crit {
            damage = damage * 3 / 2;
            self.stats.crits = self.stats.crits.saturating_add(1);
        }
        if target_kind.is_structure() {
            // Minions and heroes need to be able to crack buildings inside a
            // reasonable headless match.
            damage *= 2;
        }
        let cooldown = self.entities[index].attack_cooldown;
        self.entities[index].cooldown = cooldown;
        self.stats.attacks = self.stats.attacks.saturating_add(1);
        self.entities[target].hp -= damage;
        if self.entities[target].hp > 0 {
            return;
        }
        self.entities[target].hp = 0;
        self.entities[target].alive = false;
        self.entities[target].path.clear();
        self.entities[target].path_goal = None;
        self.stats.deaths = self.stats.deaths.saturating_add(1);
        if target_kind.is_hero() {
            self.entities[target].respawn = HERO_RESPAWN;
        }
        if attacker_is_hero {
            self.grant_reward(index, target_kind);
        }
        if target_kind == EntityKind::Nexus {
            self.winner = Some(attacker_team);
        }
    }

    fn grant_reward(&mut self, index: usize, victim: EntityKind) {
        let (gold, xp) = match victim {
            EntityKind::Minion => (18u32, 20u32),
            EntityKind::Tower => (150, 100),
            EntityKind::Inhibitor => (200, 150),
            EntityKind::Hero => (300, 200),
            EntityKind::Nexus => (500, 0),
        };
        let entity = &mut self.entities[index];
        entity.gold = entity.gold.saturating_add(gold);
        entity.xp = entity.xp.saturating_add(xp);
        while entity.level < 12 && entity.xp >= xp_for_level(entity.level) {
            entity.xp -= xp_for_level(entity.level);
            entity.level += 1;
            entity.max_hp += 60;
            entity.hp = (entity.hp + 60).min(entity.max_hp);
            entity.damage += 3;
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

    fn advance_lane(&mut self, index: usize) {
        let (lane, lane_index, dir) = {
            let entity = &self.entities[index];
            (entity.lane as usize, entity.lane_index, entity.lane_dir)
        };
        let Some(lane_points) = self.lanes.get(lane) else {
            return;
        };
        let at_end = if dir > 0 {
            lane_index + 1 >= lane_points.len()
        } else {
            lane_index == 0
        };
        if at_end {
            let team = self.entities[index].team;
            let enemy_nexus = self.nexus(1 - team).map(|nexus| nexus.tile());
            if let Some(tile) = enemy_nexus {
                self.move_to_tile(index, tile);
            }
            return;
        }
        let lane_len = lane_points.len();
        let Some(waypoint) = lane_points.get(lane_index).copied() else {
            return;
        };
        if self.move_to_tile(index, waypoint) {
            let next = lane_index as i32 + i32::from(dir);
            let entity = &mut self.entities[index];
            if next >= 0 && (next as usize) < lane_len {
                entity.lane_index = next as usize;
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
        let step = i64::from(entity.speed).max(1);
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
        if self.winner.is_some() {
            return;
        }
        let blue = self.nexus(0).map(|entity| entity.hp).unwrap_or(0);
        let red = self.nexus(1).map(|entity| entity.hp).unwrap_or(0);
        if blue <= 0 && red <= 0 {
            self.winner = Some(2);
        } else if blue <= 0 {
            self.winner = Some(1);
        } else if red <= 0 {
            self.winner = Some(0);
        } else if self.tick >= MAX_MATCH_TICKS {
            // Timeout: compare nexus health with integer cross-multiplication
            // so the result never depends on floating point.
            let blue = self.nexus(0).map(|entity| (entity.hp, entity.max_hp));
            let red = self.nexus(1).map(|entity| (entity.hp, entity.max_hp));
            if let (Some((blue_hp, blue_max)), Some((red_hp, red_max))) = (blue, red) {
                let blue_ratio = i64::from(blue_hp) * i64::from(red_max.max(1));
                let red_ratio = i64::from(red_hp) * i64::from(blue_max.max(1));
                self.winner = Some(if blue_ratio > red_ratio {
                    0
                } else if red_ratio > blue_ratio {
                    1
                } else {
                    2
                });
            }
        }
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
        mix(&mut hash, u64::from(self.stats.waves));
        mix(&mut hash, u64::from(self.rng.state));
        mix(&mut hash, u64::from(self.wave_timer));
        mix(&mut hash, u64::from(self.wave_serial));
        for entity in &self.entities {
            mix(&mut hash, u64::from(entity.id));
            mix(&mut hash, u64::from(entity.team));
            mix(&mut hash, entity.kind as u64);
            mix(&mut hash, u64::from(entity.lane));
            mix(&mut hash, entity.x as i64 as u64);
            mix(&mut hash, entity.z as i64 as u64);
            mix(&mut hash, entity.hp as i64 as u64);
            mix(&mut hash, entity.max_hp as i64 as u64);
            mix(&mut hash, entity.damage as i64 as u64);
            mix(&mut hash, u64::from(entity.cooldown));
            mix(&mut hash, u64::from(entity.level));
            mix(&mut hash, u64::from(entity.xp));
            mix(&mut hash, u64::from(entity.gold));
            mix(&mut hash, u64::from(entity.respawn));
            mix(&mut hash, u64::from(entity.alive));
            mix(&mut hash, u64::from(entity.target.unwrap_or(99)));
            mix(&mut hash, u64::from(entity.lane_index as u32));
            mix(&mut hash, entity.lane_dir as i64 as u64);
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

fn xp_for_level(level: u32) -> u32 {
    120 + level.saturating_mul(80)
}

/// Integer square root, used by fixed-point movement.
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
        let mut game = Match::new(1, 64);
        game.step();
        assert_eq!(game.tick, 1);
        assert_ne!(game.last_tick_hash, 0);
    }

    #[test]
    fn same_seed_and_inputs_produce_the_same_hash() {
        let mut first = Match::new(0x00C0_FFEE, 64);
        let mut second = Match::new(0x00C0_FFEE, 64);
        for _ in 0..600 {
            first.step();
            second.step();
            assert_eq!(first.last_tick_hash, second.last_tick_hash);
        }
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
        assert_eq!(first.stats.commands, 1);
    }

    #[test]
    fn waves_spawn_on_the_fixed_interval() {
        let mut game = Match::new(3, 64);
        game.run(FIRST_WAVE as u64 + 1);
        assert_eq!(game.stats.waves, 1);
        let minions = game
            .entities
            .iter()
            .filter(|entity| entity.kind == EntityKind::Minion)
            .count();
        assert_eq!(minions, 18);
    }

    #[test]
    fn a_bot_match_can_end_on_a_nexus() {
        let mut game = Match::new(7, 64);
        game.run(MAX_MATCH_TICKS);
        let winner = game.winner.expect("bots should resolve the match");
        assert!(winner < 2, "seed 7 should produce a decisive nexus kill");
        assert!(game.tick < MAX_MATCH_TICKS, "the match should not time out");
        assert_eq!(game.nexus(1 - winner).unwrap().hp, 0);
        assert!(game.stats.attacks > 0);
        assert!(game.stats.waves > 0);
    }

    #[test]
    fn nexus_is_shielded_until_a_lane_inhibitor_falls() {
        let game = Match::new(5, 64);
        assert!(!game.nexus_vulnerable(0));
        let mut game = game;
        let inhibitor = game
            .entities
            .iter_mut()
            .find(|entity| entity.kind == EntityKind::Inhibitor && entity.team == 1)
            .expect("red inhibitor");
        inhibitor.alive = false;
        assert!(game.nexus_vulnerable(0));
    }

    #[test]
    fn integer_distance_and_roots_are_stable() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(1), 1);
        assert_eq!(isqrt(1024 * 1024), 1024);
        assert_eq!(isqrt(2 * 1024 * 1024), 1448);
    }
}
