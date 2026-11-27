use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use glam::{Mat3, Vec2, Vec3};
use rift_cache::{
    Cache, Locations, Model, NpcDefinition, ObjectDefinition, OverlayDefinition, Terrain,
    UnderlayDefinition,
};
use rift_sim::{Grid, find_path, simplify_path};
use serde::Deserialize;

mod props;

/// Cache scenery is authored for a first-person player, but the MOBA camera
/// frames a hero that is roughly two tiles tall. Natural props therefore need
/// a stylistic lift so trees and ruins tower over the heroes instead of
/// reading as garden ornaments.
const NATURAL_SCENERY_SCALE: f32 = 1.5;
const POCKET_SCENERY_SCALE: f32 = 1.35;
const TREE_LINE_SCALE_MIN: f32 = 1.35;
const TREE_LINE_SCALE_MAX: f32 = 2.05;
const RUIN_SCALE_MIN: f32 = 1.35;
const RUIN_SCALE_MAX: f32 = 1.85;

/// Small cache objects scattered over the grass: flowers, ferns, mushrooms,
/// loose rocks, twigs, roots and stumps. 117HD's ground is dense with these,
/// and they do most of the work in making a flat texture field feel natural.
const DECOR_LIMIT: usize = 2600;
const DECOR_LANE_CLEAR: f32 = 2.6 * 128.0;
const DECOR_STRUCTURE_CLEAR: f32 = 2.2 * 128.0;
/// Cache decorations that are a single untextured plane or a placeholder cube
/// look wrong once the flat-colour fallback is applied.
const DECOR_MODEL_BLOCKLIST: &[u32] = &[3951, 4730, 32530];

/// River crossings: the terrain is levelled into a ford and a cache-textured
/// deck is laid over it, aligned with the lane.
const BRIDGE_HALF_LENGTH: f32 = 4.5 * 128.0;
const BRIDGE_HALF_WIDTH: f32 = 2.6 * 128.0;
const BRIDGE_DECK_LIFT: f32 = 0.5;

/// Convert an sRGB channel to linear light. The surface is an sRGB target, so
/// shader output must be linear or the palette is double-gamma corrected.
pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Byte-at-a-time sRGB conversion for the per-frame dynamic mesh. A powf per
/// colour channel per vertex was a measurable part of the mesh build.
fn srgb_u8_to_linear(value: u8) -> f32 {
    static LUT: std::sync::OnceLock<[f32; 256]> = std::sync::OnceLock::new();
    LUT.get_or_init(|| std::array::from_fn(|i| srgb_to_linear(i as f32 / 255.0)))[value as usize]
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SceneVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
    pub uv: [f32; 2],
    pub ao: f32,
    pub material: u32,
    pub texture_id: u32,
}

impl SceneVertex {
    pub fn new(position: Vec3, normal: Vec3, color: [u8; 3]) -> Self {
        Self {
            position: position.to_array(),
            normal: normal.normalize_or_zero().to_array(),
            color: [
                srgb_u8_to_linear(color[0]),
                srgb_u8_to_linear(color[1]),
                srgb_u8_to_linear(color[2]),
                1.0,
            ],
            uv: [0.0, 0.0],
            ao: 1.0,
            material: 0,
            texture_id: 0,
        }
    }

    pub fn unlit(position: Vec3, color: [u8; 3]) -> Self {
        let mut vertex = Self::new(position, Vec3::Y, color);
        vertex.material = 1;
        vertex
    }

    /// Lit, world-space triplanar stone. The procedural castle boxes carry no
    /// authored UVs, so a planar cache sprite would smear on their vertical
    /// faces; the shader projects the stone sprite on all three axes instead.
    pub fn stone(position: Vec3, normal: Vec3, color: [u8; 3]) -> Self {
        let mut vertex = Self::new(position, normal, color);
        vertex.material = 8;
        vertex
    }

    pub fn blob(position: Vec3, alpha: f32, uv: [f32; 2]) -> Self {
        Self {
            position: position.to_array(),
            normal: Vec3::Y.to_array(),
            color: [0.0, 0.0, 0.0, alpha],
            uv,
            ao: 1.0,
            material: 2,
            texture_id: 0,
        }
    }
}

#[derive(Default)]
pub struct MeshData {
    pub vertices: Vec<SceneVertex>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn push_triangle(&mut self, a: SceneVertex, b: SceneVertex, c: SceneVertex) {
        let base = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&[a, b, c]);
        self.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }
}

/// Height grid covering the whole loaded block, used for camera picking and
/// keeping units on the ground.
#[derive(Clone)]
pub struct HeightGrid {
    origin_tile_x: i32,
    origin_tile_y: i32,
    /// World units between samples: 128 for the per-tile grid, smaller
    /// for the subdivided display terrain.
    step: f32,
    size: usize,
    heights: Vec<f32>,
}

impl HeightGrid {
    pub fn height_at(&self, world_x: f32, world_z: f32) -> f32 {
        let gx = (world_x - self.origin_tile_x as f32 * 128.0) / self.step;
        let gy = (world_z - self.origin_tile_y as f32 * 128.0) / self.step;
        if gx < 0.0 || gy < 0.0 || gx >= (self.size - 1) as f32 || gy >= (self.size - 1) as f32 {
            return 0.0;
        }
        let x0 = gx.floor() as usize;
        let y0 = gy.floor() as usize;
        let fx = gx - x0 as f32;
        let fy = gy - y0 as f32;
        let h00 = self.heights[y0 * self.size + x0];
        let h10 = self.heights[y0 * self.size + x0 + 1];
        let h01 = self.heights[(y0 + 1) * self.size + x0];
        let h11 = self.heights[(y0 + 1) * self.size + x0 + 1];
        let a = h00 + (h10 - h00) * fx;
        let b = h01 + (h11 - h01) * fx;
        a + (b - a) * fy
    }

    pub fn contains(&self, world_x: f32, world_z: f32) -> bool {
        let gx = (world_x - self.origin_tile_x as f32 * 128.0) / self.step;
        let gy = (world_z - self.origin_tile_y as f32 * 128.0) / self.step;
        gx >= 0.0 && gy >= 0.0 && gx < (self.size - 1) as f32 && gy < (self.size - 1) as f32
    }
}

#[derive(Clone)]
pub struct WorldCollision {
    grid: Grid,
    sight_grid: Grid,
    /// Structures and castle walls: blocked tiles that no Web Nest opens.
    solid: Grid,
    origin_tile_x: i32,
    origin_tile_y: i32,
}

impl WorldCollision {
    pub fn new(grid: Grid, sight_grid: Grid, origin_tile_x: i32, origin_tile_y: i32) -> Self {
        let solid = Grid::new(grid.width() as usize, grid.height() as usize);
        Self {
            grid,
            sight_grid,
            solid,
            origin_tile_x,
            origin_tile_y,
        }
    }

    /// Mark the tiles that stay shut even inside a Web Nest (towers, the
    /// nexus, castle walls). The grid must match the collision grid's size.
    pub fn with_solid(mut self, solid: Grid) -> Self {
        debug_assert_eq!(
            (solid.width(), solid.height()),
            (self.grid.width(), self.grid.height())
        );
        self.solid = solid;
        self
    }

    pub fn width(&self) -> i32 {
        self.grid.width()
    }

    pub fn height(&self) -> i32 {
        self.grid.height()
    }

    pub fn origin_tile(&self) -> (i32, i32) {
        (self.origin_tile_x, self.origin_tile_y)
    }

    pub fn tile_of(&self, world_x: f32, world_z: f32) -> (i32, i32) {
        (
            (world_x / 128.0).floor() as i32 - self.origin_tile_x,
            (world_z / 128.0).floor() as i32 - self.origin_tile_y,
        )
    }

    pub fn world_of_tile(&self, tx: i32, ty: i32, height: &HeightGrid) -> Vec3 {
        let x = (self.origin_tile_x + tx) as f32 * 128.0 + 64.0;
        let z = (self.origin_tile_y + ty) as f32 * 128.0 + 64.0;
        Vec3::new(x, height.height_at(x, z), z)
    }

    pub fn is_walkable_world(&self, world_x: f32, world_z: f32) -> bool {
        let (tx, ty) = self.tile_of(world_x, world_z);
        self.grid.is_walkable(tx, ty)
    }

    /// Trees, walls, and structures block sight; water does not.
    pub fn blocks_sight_tile(&self, tx: i32, ty: i32) -> bool {
        !self.sight_grid.is_walkable(tx, ty)
    }

    /// The tiles a walker standing at `from` can reach on foot. `from` is
    /// snapped to the nearest walkable tile first, as a path search would.
    pub fn walk_region(&self, from: Vec3) -> WalkRegion {
        let (tx, ty) = self.tile_of(from.x, from.z);
        let start = self.grid.nearest_walkable(tx, ty, 20).unwrap_or((tx, ty));
        WalkRegion {
            seen: self.grid.reachable_from(start),
            width: self.grid.width(),
            height: self.grid.height(),
            origin_tile_x: self.origin_tile_x,
            origin_tile_y: self.origin_tile_y,
        }
    }

    pub fn nearest_walkable_world(&self, world_x: f32, world_z: f32) -> Option<(i32, i32)> {
        let (tx, ty) = self.tile_of(world_x, world_z);
        self.grid.nearest_walkable(tx, ty, 12)
    }

    pub fn path_world(&self, start: Vec3, goal: Vec3, height: &HeightGrid) -> Vec<Vec3> {
        let start_tile = self.tile_of(start.x, start.z);
        let goal_tile = self.tile_of(goal.x, goal.z);
        let Some(path) = find_path(&self.grid, start_tile, goal_tile) else {
            return Vec::new();
        };
        let simplified = simplify_path(&self.grid, &path);
        simplified
            .into_iter()
            .skip(1)
            .map(|(tx, ty)| self.world_of_tile(tx, ty, height))
            .collect()
    }

    /// A copy in which every tile whose middle lies inside one of `discs`
    /// (centre, radius in world units) is open ground, unless it is solid
    /// (a structure or castle wall). Web Nest lets its caster walk through
    /// the trees, rocks and cliffs inside the web; sight still stops at them.
    pub fn opened(&self, discs: &[(Vec3, f32)]) -> WorldCollision {
        let mut grid = self.grid.clone();
        let (ox, oz) = (
            self.origin_tile_x as f32 * 128.0,
            self.origin_tile_y as f32 * 128.0,
        );
        for &(centre, radius) in discs {
            let (cx, cz) = (
                (centre.x - ox).round() as i32,
                (centre.z - oz).round() as i32,
            );
            let radius = radius.max(0.0) as i32;
            let (lo_x, lo_y) = grid.cell_at(cx - radius, cz - radius);
            let (hi_x, hi_y) = grid.cell_at(cx + radius, cz + radius);
            let reach = radius as i64 * radius as i64;
            for ty in lo_y..=hi_y {
                for tx in lo_x..=hi_x {
                    let (mx, my) = grid.centre_of(tx, ty);
                    let (dx, dy) = ((mx - cx) as i64, (my - cz) as i64);
                    if dx * dx + dy * dy <= reach && !self.solid.is_blocked(tx, ty) {
                        grid.set_blocked(tx, ty, false);
                    }
                }
            }
        }
        WorldCollision {
            grid,
            sight_grid: self.sight_grid.clone(),
            solid: self.solid.clone(),
            origin_tile_x: self.origin_tile_x,
            origin_tile_y: self.origin_tile_y,
        }
    }

    /// Move with axis-separated collision sliding. Used by hold-to-steer.
    pub fn slide_move(&self, pos: Vec3, delta: Vec3, height: &HeightGrid) -> Vec3 {
        let mut next = pos + delta;
        if self.is_walkable_world(next.x, next.z) {
            next.y = height.height_at(next.x, next.z);
            return next;
        }
        next = pos + Vec3::new(delta.x, 0.0, 0.0);
        if self.is_walkable_world(next.x, next.z) {
            next.y = height.height_at(next.x, next.z);
            return next;
        }
        next = pos + Vec3::new(0.0, 0.0, delta.z);
        if self.is_walkable_world(next.x, next.z) {
            next.y = height.height_at(next.x, next.z);
            return next;
        }
        pos
    }
}

/// Tiles reachable on foot from one spot ([`WorldCollision::walk_region`]),
/// so a leap can refuse to land somewhere its hero could never walk out of.
pub struct WalkRegion {
    seen: Vec<bool>,
    width: i32,
    height: i32,
    origin_tile_x: i32,
    origin_tile_y: i32,
}

impl WalkRegion {
    /// Whether a world point stands on a tile of the region.
    pub fn contains(&self, world_x: f32, world_z: f32) -> bool {
        let tx = (world_x / 128.0).floor() as i32 - self.origin_tile_x;
        let ty = (world_z / 128.0).floor() as i32 - self.origin_tile_y;
        tx >= 0
            && ty >= 0
            && tx < self.width
            && ty < self.height
            && self.seen[(ty * self.width + tx) as usize]
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StructureKind {
    Tower,
    Inhibitor,
    Nexus,
    /// A lane watchtower. It acquires a target inside its short aggro radius
    /// and then keeps firing until the target dies or leaves the longer
    /// locked range.
    Watchtower,
}

#[derive(Debug, Clone)]
pub struct StructureSpawn {
    pub team: u8,
    pub kind: StructureKind,
    pub lane_index: Option<usize>,
    pub tile: [f32; 2],
    pub model_id: u32,
    pub pos: Vec3,
    pub health: f32,
    pub range: f32,
    pub damage: f32,
    pub interval: f32,
    pub scale: f32,
    pub aggro: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct StructureJson {
    team: u8,
    kind: StructureKind,
    tile: [f32; 2],
    model: u32,
    health: f32,
    range: f32,
    damage: f32,
    interval: f32,
    scale: f32,
    /// Range at which a watchtower first acquires a target. Once it has a
    /// target it keeps firing from `range`, which is the larger lock range.
    #[serde(default)]
    aggro: f32,
}

/// A biome pocket: a rectangle of a real OSRS region transplanted into the
/// map so each side of the rift has its own scenery and ground palette.
#[derive(Debug, Clone, Deserialize)]
struct BiomeJson {
    id: String,
    #[serde(default)]
    side: String,
    #[serde(default)]
    label: String,
    /// OSRS region (region_x, region_y) the pocket is cut from.
    region: [u32; 2],
    /// Source rectangle inside the region: x, y, width, height (tiles).
    #[serde(default)]
    rect: Option<[u32; 4]>,
    /// Map tile where the source rectangle is placed.
    origin: [f32; 2],
    /// Ground tint multiplier applied to the pocket terrain.
    #[serde(default)]
    tint: Option<[f32; 3]>,
    /// Grass layer: 0 classic, 1 lush, 2 dry, 3 steppe, 4 dire, 5 straw.
    #[serde(default)]
    grass: Option<u8>,
    /// Scenery budget for the pocket.
    #[serde(default = "default_budget")]
    budget: usize,
}

fn default_budget() -> usize {
    90
}

fn default_grass_variant(id: &str) -> u8 {
    match id {
        "radiant_human_city" | "radiant_elven_lands" | "radiant_gnome_stronghold" => 1,
        "dire_dark_magic" | "dire_vampire" => 4,
        "dire_ogre" => 3,
        "dire_goblin" => 2,
        _ => 0,
    }
}

/// The river that splits the map between the radiant and dire halves.
#[derive(Debug, Clone, Deserialize)]
struct RiverJson {
    points: Vec<[f32; 2]>,
    /// Half width of the river in tiles.
    #[serde(default = "default_river_width")]
    width: f32,
    /// How far the bed is cut below the banks, in world units.
    #[serde(default = "default_river_depth")]
    depth: f32,
    /// Tiles where a lane crosses the river and stays walkable.
    #[serde(default)]
    bridges: Vec<[f32; 2]>,
    /// Water surface height at the first and last point (world units).
    #[serde(default)]
    levels: Option<[f32; 2]>,
}

fn default_river_width() -> f32 {
    3.5
}

fn default_river_depth() -> f32 {
    30.0
}

#[derive(Debug, Clone, Deserialize)]
struct LaneJson {
    id: String,
    waypoints: Vec<[f32; 2]>,
    towers: Vec<StructureJson>,
    inhibitors: Vec<StructureJson>,
    #[serde(default)]
    watchtowers: Vec<StructureJson>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Atmosphere {
    pub sun_dir: [f32; 3],
    pub sun_color: [f32; 3],
    pub ambient_sky: [f32; 3],
    pub ambient_ground: [f32; 3],
    pub fog_color: [f32; 3],
    pub fog_density: f32,
    pub fog_height: f32,
    pub sky_top: [f32; 3],
    pub sky_horizon: [f32; 3],
    pub ground_fog: f32,
}

impl Default for Atmosphere {
    fn default() -> Self {
        Self {
            // Late-afternoon sun, low enough to throw long shadows across the
            // lanes and to keep the ground out of the washed-out noon range.
            // Only faintly warm: a yellow key turned the grass olive
            // (2026-11-24 study, docs/ROADMAPPLAN.md).
            sun_dir: [-0.42, 0.62, 0.38],
            sun_color: [0.96, 0.93, 0.82],
            // Overcast sky bounce: neutral warm grey rather than the saturated
            // blue of the early build, which was turning every shadow cyan.
            ambient_sky: [0.50, 0.53, 0.51],
            ambient_ground: [0.21, 0.25, 0.19],
            // Grey-green haze. The reference never resolves the ground at
            // distance; it dissolves into a cool green mist.
            fog_color: [0.33, 0.41, 0.38],
            fog_density: 0.000022,
            fog_height: 340.0,
            sky_top: [0.27, 0.35, 0.40],
            sky_horizon: [0.56, 0.60, 0.56],
            ground_fog: 0.14,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct MapJson {
    #[serde(default)]
    name: String,
    wave_interval: f32,
    /// Player spawn tile. Defaults to the first lane's second waypoint.
    #[serde(default)]
    spawn: Option<[f32; 2]>,
    nexus: Vec<StructureJson>,
    lanes: Vec<LaneJson>,
    #[serde(default)]
    camps: Vec<CampJson>,
    #[serde(default)]
    brush: Vec<[f32; 4]>,
    #[serde(default)]
    biomes: Vec<BiomeJson>,
    #[serde(default)]
    river: Option<RiverJson>,
    #[serde(default)]
    castle: Option<CastleJson>,
    #[serde(default)]
    atmosphere: Atmosphere,
    /// Phase 5 set dressing: statues, banners, cliff rocks, flora.
    #[serde(default)]
    props: props::PropsJson,
    /// Cache model laid as planks across each river bridge.
    #[serde(default)]
    bridge_model: Option<BridgeModelJson>,
}

/// A cache model tiled across every bridge deck.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct BridgeModelJson {
    pub model: u32,
    /// Extra yaw in degrees, to line the model's long axis up with the lane.
    #[serde(default)]
    pub yaw: f32,
    #[serde(default = "one")]
    pub scale: f32,
    /// How far apart the pieces are along the lane, in tiles.
    #[serde(default = "one")]
    pub spacing: f32,
    /// Pieces side by side across the deck.
    #[serde(default = "one_u32")]
    pub across: u32,
}

fn one_u32() -> u32 {
    1
}

#[derive(Debug, Clone, Deserialize)]
struct CastleJson {
    /// Map bounds in tiles: x0, z0, x1, z1.
    bounds: [f32; 4],
    wall_model: u32,
    #[serde(default = "one")]
    wall_scale: f32,
    tower_model: u32,
    #[serde(default = "one")]
    tower_scale: f32,
    #[serde(default)]
    gate_model: u32,
    #[serde(default = "one")]
    gate_scale: f32,
    #[serde(default)]
    platform_model: Option<u32>,
    #[serde(default)]
    statue_model: Option<u32>,
    /// Fraction of each perimeter edge between towers. 0.25 = four spans.
    #[serde(default = "default_tower_spacing")]
    tower_spacing: f32,
    #[serde(default)]
    bases: Vec<CastleBaseJson>,
}

fn default_tower_spacing() -> f32 {
    0.25
}

fn one() -> f32 {
    1.0
}

#[derive(Debug, Clone, Deserialize)]
struct CastleBaseJson {
    team: u8,
    center: [f32; 2],
    size: f32,
    /// "bottom_left" or "top_right".
    corner: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CastlePieceKind {
    Wall,
    Tower,
    Gate,
    Platform,
    Statue,
}

/// Generated castle wall/tower geometry shared by collision and rendering.
#[derive(Debug, Clone)]
struct CastlePiece {
    kind: CastlePieceKind,
    model: u32,
    tile: [f32; 2],
    yaw: f32,
    scale: f32,
    /// Forced stone texture for cache-model statues.
    texture: u32,
    /// Footprint in tiles centred on `tile` (1 = 1x1, 2 = 2x2).
    footprint: f32,
    block: bool,
    /// The team whose courtyard this piece encloses: it takes that god's
    /// palette (`CastlePalette::for_team`). The open perimeter is `None`.
    team: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct CastleSpec {
    pub bounds: (f32, f32, f32, f32),
    pub wall_model: u32,
    pub wall_scale: f32,
    pub tower_model: u32,
    pub tower_scale: f32,
    pub gate_model: u32,
    pub gate_scale: f32,
    pub platform_model: Option<u32>,
    pub statue_model: Option<u32>,
    pub tower_spacing: f32,
    pub bases: Vec<CastleBase>,
}

#[derive(Debug, Clone)]
pub struct CastleBase {
    pub team: u8,
    pub center: (f32, f32),
    pub size: f32,
    pub corner: CastleCorner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastleCorner {
    BottomLeft,
    TopRight,
}

/// The team buff a camp grants its killers for a minute. Might and Swift
/// come from the plain camps and the first two bosses; each later region
/// boss has a buff of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampBuff {
    Might,
    Swift,
    /// The Thermonuclear smoke devil (Human City): +40% movement speed and
    /// ability cooldowns recover 60% faster.
    SpeedDemon,
    /// Sarachnis (Vampire Swamp): the team's basic attacks envenom.
    BroodVenom,
}

impl CampBuff {
    /// The map file's name for it (`"buff"` on a camp).
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "might" => Some(Self::Might),
            "swift" => Some(Self::Swift),
            "speed_demon" => Some(Self::SpeedDemon),
            "brood_venom" => Some(Self::BroodVenom),
            _ => None,
        }
    }

    /// How players read it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Might => "Might",
            Self::Swift => "Swift",
            Self::SpeedDemon => "Speed Demon",
            Self::BroodVenom => "Brood Venom",
        }
    }

    /// The colour its name, timer and camp plates take.
    pub fn colour(self) -> [u8; 3] {
        match self {
            Self::Might => [255, 160, 60],
            Self::Swift => [90, 220, 255],
            Self::SpeedDemon => [255, 90, 200],
            Self::BroodVenom => [120, 230, 70],
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct CampJson {
    id: String,
    tile: [f32; 2],
    model: u32,
    idle: Option<u32>,
    walk: Option<u32>,
    health: f32,
    respawn: f32,
    buff: String,
    scale: f32,
    #[serde(default)]
    damage: f32,
    #[serde(default)]
    range: f32,
    #[serde(default)]
    interval: f32,
    #[serde(default)]
    speed: f32,
    #[serde(default)]
    aggro: f32,
    #[serde(default)]
    leash: f32,
    /// A region boss (bots take it as a team for the buff).
    #[serde(default)]
    boss: bool,
    /// The creature's name (its cache NPC name), for the objectives panel
    /// and the hover tooltip.
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CampSpawn {
    pub id: String,
    pub pos: Vec3,
    pub model_id: u32,
    pub idle_sequence: Option<u32>,
    pub walk_sequence: Option<u32>,
    pub health: f32,
    pub respawn: f32,
    pub buff: CampBuff,
    pub scale: f32,
    pub damage: f32,
    pub range: f32,
    pub interval: f32,
    pub speed: f32,
    pub aggro: f32,
    pub leash: f32,
    pub boss: bool,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BiomePocket {
    pub id: String,
    pub side: String,
    pub label: String,
    pub region: (u32, u32),
    pub rect: (u32, u32, u32, u32),
    pub origin: (i32, i32),
    pub tint: [f32; 3],
    pub grass_variant: u8,
    pub budget: usize,
}

#[derive(Debug, Clone)]
pub struct RiverSpec {
    pub points: Vec<(f32, f32)>,
    pub width: f32,
    pub depth: f32,
    pub bridges: Vec<(f32, f32)>,
    pub levels: Option<(f32, f32)>,
}

#[derive(Debug, Clone)]
pub struct LaneSpawn {
    pub id: String,
    pub waypoints: Vec<Vec3>,
}

#[derive(Debug, Clone)]
pub struct MapLayout {
    pub name: String,
    pub lanes: Vec<LaneSpawn>,
    pub structures: Vec<StructureSpawn>,
    pub camps: Vec<CampSpawn>,
    pub brush: Vec<(f32, f32, f32, f32)>,
    pub biomes: Vec<BiomePocket>,
    pub river: Option<RiverSpec>,
    pub castle: Option<CastleSpec>,
    /// Authored player spawn tile, if any.
    pub spawn: Option<(f32, f32)>,
    pub wave_interval: f32,
    pub atmosphere: Atmosphere,
    pub props: props::PropsJson,
    pub bridge_model: Option<BridgeModelJson>,
}

impl CastleSpec {
    /// The team whose courtyard covers `tile`, its walls included: the
    /// inner walls, gates, corner towers, the stretch of perimeter wall
    /// that closes the courtyard, the dais and its statues.
    fn team_at(&self, tile: [f32; 2]) -> Option<u8> {
        let edge = 1.0;
        self.bases.iter().find_map(|base| {
            let (bx0, bz0, bx1, bz1) = self.base_rect(base);
            let inside = tile[0] >= bx0 - edge
                && tile[0] <= bx1 + edge
                && tile[1] >= bz0 - edge
                && tile[1] <= bz1 + edge;
            inside.then_some(base.team)
        })
    }

    fn base_rect(&self, base: &CastleBase) -> (f32, f32, f32, f32) {
        let half = base.size * 0.5;
        (
            base.center.0 - half,
            base.center.1 - half,
            base.center.0 + half,
            base.center.1 + half,
        )
    }

    /// Scenery-free rectangles: the wall bands and the two base courtyards.
    pub fn keep_clear(&self) -> Vec<(f32, f32, f32, f32)> {
        let (x0, z0, x1, z1) = self.bounds;
        let band = 3.0;
        let mut rects = vec![
            (x0 - band, z0 - band, x1 + band, z0 + band),
            (x0 - band, z1 - band, x1 + band, z1 + band),
            (x0 - band, z0 - band, x0 + band, z1 + band),
            (x1 - band, z0 - band, x1 + band, z1 + band),
        ];
        for base in &self.bases {
            let (bx0, bz0, bx1, bz1) = self.base_rect(base);
            rects.push((bx0 - band, bz0 - band, bx1 + band, bz1 + band));
        }
        rects
    }

    /// Generate the full castle geometry. This one list feeds both collision
    /// and the static mesh builder, so they can never disagree.
    fn layout(&self) -> Vec<CastlePiece> {
        let (x0, z0, x1, z1) = self.bounds;
        let mut walls: Vec<(f32, f32, f32)> = Vec::new();
        let mut towers: Vec<(f32, f32)> = Vec::new();
        let mut gates: Vec<(f32, f32, f32)> = Vec::new();
        let mut statues: Vec<(f32, f32, f32)> = Vec::new();

        for x in x0.round() as i32..=x1.round() as i32 {
            let x = x as f32;
            walls.push((x, z0, 0.0));
            walls.push((x, z1, std::f32::consts::PI));
        }
        for z in z0.round() as i32 + 1..z1.round() as i32 {
            let z = z as f32;
            walls.push((x0, z, std::f32::consts::FRAC_PI_2));
            walls.push((x1, z, -std::f32::consts::FRAC_PI_2));
        }

        let step = self.tower_spacing.clamp(0.05, 0.5);
        let mut f = step;
        while f < 0.999 {
            let x = x0 + (x1 - x0) * f;
            let z = z0 + (z1 - z0) * f;
            towers.push((x, z0));
            towers.push((x, z1));
            towers.push((x0, z));
            towers.push((x1, z));
            f += step;
        }
        towers.extend([(x0, z0), (x1, z0), (x0, z1), (x1, z1)]);

        let gate_half = 4.0;
        for base in &self.bases {
            let (bx0, bz0, bx1, bz1) = self.base_rect(base);
            let (cx, cz) = base.center;
            tracing::trace!(
                "castle base team {} at {:.1},{:.1} corner {:?}",
                base.team,
                cx,
                cz,
                base.corner
            );
            let dir = match base.corner {
                CastleCorner::BottomLeft => 1.0,
                CastleCorner::TopRight => -1.0,
            };

            // Inner walls face the map centre; the two outer sides use the
            // perimeter wall. Each inner side gets a gate gap centred on the
            // nexus so lanes can leave the courtyard.
            let inner_x = if dir > 0.0 { bx1 } else { bx0 };
            let inner_z = if dir > 0.0 { bz0 } else { bz1 };
            for z in bz0.round() as i32..=bz1.round() as i32 {
                let z = z as f32;
                if (z - cz).abs() < gate_half {
                    continue;
                }
                walls.push((inner_x, z, std::f32::consts::FRAC_PI_2));
            }
            for x in bx0.round() as i32..=bx1.round() as i32 {
                let x = x as f32;
                if (x - cx).abs() < gate_half {
                    continue;
                }
                walls.push((x, inner_z, 0.0));
            }

            let gate_yaw_x = if dir > 0.0 {
                -std::f32::consts::FRAC_PI_2
            } else {
                std::f32::consts::FRAC_PI_2
            };
            let gate_yaw_z = if dir > 0.0 { 0.0 } else { std::f32::consts::PI };
            gates.push((inner_x, cz, gate_yaw_x));
            gates.push((cx, inner_z, gate_yaw_z));

            // Corner towers inset from the perimeter so their 2x2 footprints
            // do not hang outside the map.
            let inset = 2.5;
            for (tx, tz) in [
                (bx0 + inset, bz0 + inset),
                (bx1 - inset, bz0 + inset),
                (bx0 + inset, bz1 - inset),
                (bx1 - inset, bz1 - inset),
            ] {
                towers.push((tx, tz));
            }

            if self.statue_model.is_some() {
                statues.push((cx - 4.0, cz, std::f32::consts::FRAC_PI_2));
                statues.push((cx + 4.0, cz, -std::f32::consts::FRAC_PI_2));
                statues.push((cx, cz - 4.0, std::f32::consts::PI));
                statues.push((cx, cz + 4.0, 0.0));
            }
        }

        towers.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        });
        towers.dedup_by(|a, b| (a.0 - b.0).abs() < 3.0 && (a.1 - b.1).abs() < 3.0);

        walls.retain(|wall| {
            !towers
                .iter()
                .any(|t| (wall.0 - t.0).abs() < 2.0 && (wall.1 - t.1).abs() < 2.0)
        });

        let mut pieces = Vec::new();
        for (x, z, yaw) in walls {
            pieces.push(CastlePiece {
                kind: CastlePieceKind::Wall,
                model: self.wall_model,
                tile: [x, z],
                yaw,
                scale: self.wall_scale,
                texture: 46,
                footprint: 1.0,
                block: true,
                team: None,
            });
        }
        for (x, z) in towers {
            pieces.push(CastlePiece {
                kind: CastlePieceKind::Tower,
                model: self.tower_model,
                tile: [x, z],
                yaw: 0.0,
                scale: self.tower_scale,
                texture: 46,
                footprint: 2.0,
                block: true,
                team: None,
            });
        }
        if self.gate_model != 0 {
            for (x, z, yaw) in gates {
                pieces.push(CastlePiece {
                    kind: CastlePieceKind::Gate,
                    model: self.gate_model,
                    tile: [x, z],
                    yaw,
                    scale: self.gate_scale,
                    texture: 47,
                    footprint: 2.0,
                    block: false,
                    team: None,
                });
            }
        }
        if let Some(model) = self.platform_model {
            for base in &self.bases {
                pieces.push(CastlePiece {
                    kind: CastlePieceKind::Platform,
                    model,
                    tile: [base.center.0, base.center.1],
                    yaw: 0.0,
                    scale: 1.0,
                    texture: 44,
                    footprint: 1.0,
                    block: false,
                    team: None,
                });
            }
        }
        if let Some(model) = self.statue_model {
            for (x, z, yaw) in statues {
                pieces.push(CastlePiece {
                    kind: CastlePieceKind::Statue,
                    model,
                    tile: [x, z],
                    yaw,
                    scale: 1.0,
                    texture: 46,
                    footprint: 1.0,
                    block: false,
                    team: None,
                });
            }
        }
        for piece in &mut pieces {
            piece.team = self.team_at(piece.tile);
        }
        pieces
    }
}

fn point_in_tile_rects(rects: &[(f32, f32, f32, f32)], x: f32, z: f32) -> bool {
    rects
        .iter()
        .any(|&(x0, z0, x1, z1)| x >= x0 && x <= x1 && z >= z0 && z <= z1)
}

impl MapLayout {
    pub fn load(path: &Path, height: &HeightGrid) -> Result<Self> {
        let data = std::fs::read_to_string(path)
            .with_context(|| format!("reading map {}", path.display()))?;
        let map: MapJson = serde_json::from_str(&data)?;
        if map.lanes.is_empty() {
            anyhow::bail!("map {} needs at least one lane", path.display());
        }

        let world_of = |tile: [f32; 2]| {
            let x = (tile[0] + 0.5) * 128.0;
            let z = (tile[1] + 0.5) * 128.0;
            Vec3::new(x, height.height_at(x, z), z)
        };
        let to_spawn = |s: &StructureJson| StructureSpawn {
            team: s.team,
            kind: s.kind,
            lane_index: None,
            tile: s.tile,
            model_id: s.model,
            pos: world_of(s.tile),
            health: s.health,
            range: s.range,
            damage: s.damage,
            interval: s.interval,
            scale: s.scale,
            aggro: s.aggro,
        };

        let mut lanes = Vec::with_capacity(map.lanes.len());
        let mut structures = Vec::new();
        for s in &map.nexus {
            structures.push(to_spawn(s));
        }
        for (lane_index, lane) in map.lanes.iter().enumerate() {
            if lane.waypoints.len() < 2 {
                anyhow::bail!("lane {} needs at least two waypoints", lane.id);
            }
            lanes.push(LaneSpawn {
                id: lane.id.clone(),
                waypoints: lane.waypoints.iter().map(|&tile| world_of(tile)).collect(),
            });
            for s in &lane.towers {
                let mut spawn = to_spawn(s);
                spawn.lane_index = Some(lane_index);
                structures.push(spawn);
            }
            for s in &lane.inhibitors {
                let mut spawn = to_spawn(s);
                spawn.lane_index = Some(lane_index);
                structures.push(spawn);
            }
            for s in &lane.watchtowers {
                let mut spawn = to_spawn(s);
                spawn.lane_index = Some(lane_index);
                structures.push(spawn);
            }
        }

        let camps = map
            .camps
            .iter()
            .map(|camp| CampSpawn {
                id: camp.id.clone(),
                pos: world_of(camp.tile),
                model_id: camp.model,
                idle_sequence: camp.idle,
                walk_sequence: camp.walk,
                health: camp.health,
                respawn: camp.respawn,
                buff: CampBuff::parse(&camp.buff).unwrap_or_else(|| {
                    tracing::warn!(
                        "map: camp {} has unknown buff {:?}; Might",
                        camp.id,
                        camp.buff
                    );
                    CampBuff::Might
                }),
                scale: camp.scale,
                damage: camp.damage,
                range: camp.range,
                interval: camp.interval,
                speed: camp.speed,
                aggro: camp.aggro,
                leash: camp.leash,
                boss: camp.boss,
                name: camp.name.clone(),
            })
            .collect();

        let brush = map
            .brush
            .iter()
            .map(|b| {
                (
                    (b[0] + 0.5) * 128.0,
                    (b[1] + 0.5) * 128.0,
                    (b[2] + 0.5) * 128.0,
                    (b[3] + 0.5) * 128.0,
                )
            })
            .collect();

        let biomes = map
            .biomes
            .iter()
            .map(|biome| {
                let rect = biome.rect.unwrap_or([0, 0, 64, 64]);
                BiomePocket {
                    id: biome.id.clone(),
                    side: biome.side.clone(),
                    label: biome.label.clone(),
                    region: (biome.region[0], biome.region[1]),
                    rect: (rect[0], rect[1], rect[2], rect[3]),
                    origin: (
                        biome.origin[0].round() as i32,
                        biome.origin[1].round() as i32,
                    ),
                    tint: biome.tint.unwrap_or([1.0, 1.0, 1.0]),
                    grass_variant: biome
                        .grass
                        .unwrap_or_else(|| default_grass_variant(&biome.id)),
                    budget: biome.budget,
                }
            })
            .collect();

        let river = map.river.as_ref().map(|river| RiverSpec {
            points: river.points.iter().map(|p| (p[0], p[1])).collect(),
            width: river.width,
            depth: river.depth,
            bridges: river.bridges.iter().map(|p| (p[0], p[1])).collect(),
            levels: river.levels.map(|l| (l[0], l[1])),
        });

        let castle = map.castle.as_ref().map(|castle| CastleSpec {
            bounds: (
                castle.bounds[0],
                castle.bounds[1],
                castle.bounds[2],
                castle.bounds[3],
            ),
            wall_model: castle.wall_model,
            wall_scale: castle.wall_scale,
            tower_model: castle.tower_model,
            tower_scale: castle.tower_scale,
            gate_model: castle.gate_model,
            gate_scale: castle.gate_scale,
            platform_model: castle.platform_model,
            statue_model: castle.statue_model,
            tower_spacing: castle.tower_spacing.clamp(0.05, 0.5),
            bases: castle
                .bases
                .iter()
                .map(|base| CastleBase {
                    team: base.team,
                    center: (base.center[0], base.center[1]),
                    size: base.size.max(8.0),
                    corner: if base.corner.eq_ignore_ascii_case("top_right") {
                        CastleCorner::TopRight
                    } else {
                        CastleCorner::BottomLeft
                    },
                })
                .collect(),
        });

        // Keep authored structures out of the lane driving surface. Towers,
        // inhibitors, watchtowers and nexuses are all allowed to sit beside a
        // lane, but a 3x3 collision footprint on the centre line traps waves
        // and reads as an object dropped on the road.
        let clearance = 5.0 * 128.0;
        let mut occupied: Vec<Vec3> = Vec::with_capacity(structures.len());
        for structure in &mut structures {
            if structure.kind == StructureKind::Nexus {
                occupied.push(structure.pos);
                continue;
            }
            if distance_to_lanes(&lanes, structure.pos.x, structure.pos.z) >= clearance {
                occupied.push(structure.pos);
                continue;
            }
            let origin = structure.pos;
            let mut best = None;
            'rings: for ring in 1..=12 {
                let radius = ring as f32 * 64.0;
                let steps = 16 + ring * 4;
                for step in 0..steps {
                    let angle = step as f32 * std::f32::consts::TAU / steps as f32;
                    let x = origin.x + angle.cos() * radius;
                    let z = origin.z + angle.sin() * radius;
                    if !height.contains(x, z)
                        || distance_to_lanes(&lanes, x, z) < clearance
                        || occupied
                            .iter()
                            .any(|p| p.distance(Vec3::new(x, p.y, z)) < 3.0 * 128.0)
                    {
                        continue;
                    }
                    best = Some(Vec3::new(x, height.height_at(x, z), z));
                    break 'rings;
                }
            }
            if let Some(pos) = best {
                tracing::info!(
                    "structure {:?} moved off lane: tile {:.1},{:.1} -> {:.1},{:.1}",
                    structure.kind,
                    origin.x / 128.0,
                    origin.z / 128.0,
                    pos.x / 128.0,
                    pos.z / 128.0
                );
                structure.pos = pos;
                structure.tile = [pos.x / 128.0 - 0.5, pos.z / 128.0 - 0.5];
            }
            occupied.push(structure.pos);
        }

        Ok(Self {
            name: map.name,
            lanes,
            structures,
            camps,
            brush,
            biomes,
            river,
            castle,
            spawn: map.spawn.map(|s| (s[0], s[1])),
            wave_interval: map.wave_interval,
            atmosphere: map.atmosphere,
            props: map.props,
            bridge_model: map.bridge_model,
        })
    }

    /// The tile rectangle `[x0, y0, x1, y1]` a biome pocket covers.
    pub fn biome_rect(&self, id: &str) -> Option<[f32; 4]> {
        self.biomes.iter().find(|b| b.id == id).map(|b| {
            [
                b.origin.0 as f32,
                b.origin.1 as f32,
                (b.origin.0 + b.rect.2 as i32) as f32,
                (b.origin.1 + b.rect.3 as i32) as f32,
            ]
        })
    }

    /// True when a map tile falls inside one of the biome pockets.
    pub fn in_biome(&self, tile_x: i32, tile_y: i32) -> bool {
        self.biomes.iter().any(|biome| {
            tile_x >= biome.origin.0
                && tile_y >= biome.origin.1
                && tile_x < biome.origin.0 + biome.rect.2 as i32
                && tile_y < biome.origin.1 + biome.rect.3 as i32
        })
    }

    /// Which side of the river a tile sits on: negative is the dire half,
    /// positive is the radiant half.
    pub fn river_side(&self, tile_x: f32, tile_y: f32) -> f32 {
        let Some(river) = &self.river else {
            return 1.0;
        };
        if river.points.len() < 2 {
            return 1.0;
        }
        let (_, segment, _) = nearest_on_polyline(&river.points, tile_x, tile_y);
        let (ax, ay) = river.points[segment];
        let (bx, by) = river.points[segment + 1];
        let cross = (bx - ax) * (tile_y - ay) - (by - ay) * (tile_x - ax);
        if cross < 0.0 { -1.0 } else { 1.0 }
    }

    /// Closest point on any lane centre line plus the distance to it.
    pub fn nearest_lane_point(&self, x: f32, z: f32) -> (Vec2, f32) {
        let p = Vec2::new(x, z);
        let mut best = (p, f32::MAX);
        for lane in &self.lanes {
            for pair in lane.waypoints.windows(2) {
                let a = Vec2::new(pair[0].x, pair[0].z);
                let b = Vec2::new(pair[1].x, pair[1].z);
                let ab = b - a;
                let len2 = ab.length_squared();
                let t = if len2 <= 1e-6 {
                    0.0
                } else {
                    ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
                };
                let closest = a + ab * t;
                let d = closest.distance(p);
                if d < best.1 {
                    best = (closest, d);
                }
            }
        }
        best
    }

    pub fn distance_to_lane(&self, x: f32, z: f32) -> f32 {
        let p = Vec2::new(x, z);
        let mut best = f32::MAX;
        for lane in &self.lanes {
            for pair in lane.waypoints.windows(2) {
                let a = Vec2::new(pair[0].x, pair[0].z);
                let b = Vec2::new(pair[1].x, pair[1].z);
                best = best.min(point_segment_distance(p, a, b));
            }
        }
        best
    }

    pub fn clear_collision(
        &self,
        grid: &mut Grid,
        origin_tile_x: i32,
        origin_tile_y: i32,
        radius_tiles: f32,
    ) {
        for lane in &self.lanes {
            for pair in lane.waypoints.windows(2) {
                let a = Vec2::new(pair[0].x, pair[0].z);
                let b = Vec2::new(pair[1].x, pair[1].z);
                let steps = ((a.distance(b) / 24.0).ceil() as i32).max(1);
                for i in 0..=steps {
                    let p = a.lerp(b, i as f32 / steps as f32);
                    let tx = (p.x / 128.0).floor() as i32 - origin_tile_x;
                    let ty = (p.y / 128.0).floor() as i32 - origin_tile_y;
                    let r = radius_tiles.ceil() as i32;
                    for dy in -r..=r {
                        for dx in -r..=r {
                            if (dx * dx + dy * dy) as f32 <= radius_tiles * radius_tiles {
                                grid.set_blocked(tx + dx, ty + dy, false);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn block_structures(&self, grid: &mut Grid, origin_tile_x: i32, origin_tile_y: i32) {
        for structure in &self.structures {
            let tx = structure.tile[0].floor() as i32 - origin_tile_x;
            let ty = structure.tile[1].floor() as i32 - origin_tile_y;
            grid.block_rect(tx - 1, ty - 1, 3, 3);
        }
    }
}

fn distance_to_lanes(lanes: &[LaneSpawn], x: f32, z: f32) -> f32 {
    let p = Vec2::new(x, z);
    let mut best = f32::MAX;
    for lane in lanes {
        for pair in lane.waypoints.windows(2) {
            let a = Vec2::new(pair[0].x, pair[0].z);
            let b = Vec2::new(pair[1].x, pair[1].z);
            best = best.min(point_segment_distance(p, a, b));
        }
    }
    best
}

fn point_segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 <= 1e-6 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

#[derive(Debug, Clone)]
pub struct NpcSpawn {
    pub model_id: u32,
    pub idle_sequence: u32,
    pub walk_sequence: Option<u32>,
    pub pos: Vec3,
    pub scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmbientKind {
    Butterfly,
    Cow,
    Crab,
}

/// Non-combat scenery creatures: they wander, cannot be selected or damaged,
/// and never show health bars.
#[derive(Debug, Clone)]
pub struct AmbientSpawn {
    pub kind: AmbientKind,
    pub model_id: u32,
    pub attachments: Vec<u32>,
    pub idle_sequence: u32,
    pub walk_sequence: u32,
    pub pos: Vec3,
    pub scale: f32,
    pub recolors: Vec<(i16, i16)>,
}

pub struct SceneData {
    pub terrain: MeshData,
    pub models: MeshData,
    pub water: MeshData,
    pub shadows: MeshData,
    pub shadow_casters: MeshData,
    pub placements: usize,
    pub height: HeightGrid,
    pub collision: WorldCollision,
    pub map: MapLayout,
    pub npc_spawns: Vec<NpcSpawn>,
    pub ambient_spawns: Vec<AmbientSpawn>,
    pub player_spawn: Vec3,
    pub atmosphere: Atmosphere,
}

pub fn build(
    cache: &Cache,
    center_x: u32,
    center_y: u32,
    radius: u32,
    map_path: &Path,
) -> Result<SceneData> {
    let keys = cache
        .keys()
        .context("cache has no keys.json; cannot resolve map groups")?;

    let min_x = center_x.saturating_sub(radius);
    let max_x = center_x + radius;
    let min_y = center_y.saturating_sub(radius);
    let max_y = center_y + radius;

    let mut terrains: HashMap<(u32, u32), Terrain> = HashMap::new();
    let mut all_locations = Vec::new();

    for rx in min_x..=max_x {
        for ry in min_y..=max_y {
            let Some(locations_group) = keys.find_locations_group(rx, ry) else {
                continue;
            };
            let terrain_group = locations_group.saturating_sub(1);
            let terrain = Terrain::load(rx, ry, &cache.group(5, terrain_group)?.data)?;
            let locations = Locations::load(rx, ry, &cache.group(5, locations_group)?.data)?;
            all_locations.push((rx, ry, locations));
            terrains.insert((rx, ry), terrain);
        }
    }

    let underlays = load_underlays(cache)?;
    let overlays = load_overlays(cache)?;
    let object_files = cache
        .archive_files_with_ids(2, 6)
        .context("loading object definitions")?
        .into_iter()
        .collect::<HashMap<u32, Vec<u8>>>();

    let origin_tile_x = min_x as i32 * 64;
    let origin_tile_y = min_y as i32 * 64;
    let size = ((max_x - min_x + 1) * 64 + 1) as usize;

    let mut heights = vec![0.0f32; size * size];
    let mut colors = vec![[60u8, 90, 50]; size * size];
    let mut grass_variant = vec![0u8; size * size];
    // Packed per-tile ground overlay: bits 8..15 are the cache texture ID,
    // bits 16..23 are the blend weight. The low byte carries the grass
    // variant when the terrain triangle is built.
    let mut tile_overlay = vec![0u32; size * size];
    // Cache sea: no underlay and a blue water overlay. Parts of the block
    // under the castle are open sea off Karamja; they are turned into land
    // once the map layout is known.
    let mut sea = vec![false; size * size];

    for gy in 0..size {
        for gx in 0..size {
            let mut sum = 0.0f32;
            let mut color = [0u32; 3];
            let mut count = 0.0f32;

            for dy in -1..=0 {
                for dx in -1..=0 {
                    let tx = origin_tile_x + gx as i32 + dx;
                    let ty = origin_tile_y + gy as i32 + dy;
                    let rx = (tx >> 6) as u32;
                    let ry = (ty >> 6) as u32;
                    let lx = (tx & 63) as usize;
                    let ly = (ty & 63) as usize;
                    let Some(terrain) = terrains.get(&(rx, ry)) else {
                        continue;
                    };
                    let tile = terrain.tile(0, lx, ly);
                    sum += tile.height as f32 * 8.0;
                    let c = tile_color(terrain, &underlays, &overlays, lx, ly);
                    color[0] += c[0] as u32;
                    color[1] += c[1] as u32;
                    color[2] += c[2] as u32;
                    count += 1.0;
                }
            }

            let idx = gy * size + gx;
            heights[idx] = if count > 0.0 { sum / count } else { 0.0 };
            colors[idx] = if count > 0.0 {
                [
                    (color[0] as f32 / count) as u8,
                    (color[1] as f32 / count) as u8,
                    (color[2] as f32 / count) as u8,
                ]
            } else {
                [60, 90, 50]
            };

            // Keep the cache's textured ground overlays. Most base tiles
            // have a colour-only underlay, but the biome pockets and paths
            // carry real overlay textures.
            let tx = origin_tile_x + gx as i32;
            let ty = origin_tile_y + gy as i32;
            let rx = (tx >> 6) as u32;
            let ry = (ty >> 6) as u32;
            let lx = (tx & 63) as usize;
            let ly = (ty & 63) as usize;
            if let Some(terrain) = terrains.get(&(rx, ry)) {
                let tile = terrain.tile(0, lx, ly);
                // OSRS land always carries an underlay; a tile with only an
                // overlay (usually the textured water overlay) is sea.
                sea[idx] = tile.underlay_id == 0 && tile.overlay_id != 0;
                if let Some(overlay) = overlays.get(&(tile.overlay_id as u32))
                    && overlay.texture != 0
                {
                    let weight = if overlay.hide_underlay { 200 } else { 130 };
                    tile_overlay[idx] = pack_ground_overlay(overlay.texture, weight);
                }
            }
        }
    }

    // The map layout defines the biome pockets and the river, so it has to
    // be read before the terrain is finalised.
    let provisional_height = HeightGrid {
        origin_tile_x,
        origin_tile_y,
        step: 128.0,
        size,
        heights: heights.clone(),
    };
    let mut map = MapLayout::load(map_path, &provisional_height)?;
    let castle_keep = map
        .castle
        .as_ref()
        .map(|castle| castle.keep_clear())
        .unwrap_or_default();
    let castle_pieces = map
        .castle
        .as_ref()
        .map(|castle| castle.layout())
        .unwrap_or_default();

    // ------------------------------------------------------------------
    // Biome pockets: rectangles of real OSRS regions transplanted into the
    // map so the radiant and dire halves carry their own scenery and
    // ground palette. The dire pockets hold dark magic, vampires, ogres
    // and goblins; the radiant pockets hold human cities, elven woods and
    // the gnome stronghold.
    // ------------------------------------------------------------------
    let mut pocket_data: HashMap<String, (Terrain, Locations)> = HashMap::new();
    for biome in &map.biomes {
        let Some(locations_group) = keys.find_locations_group(biome.region.0, biome.region.1)
        else {
            tracing::warn!(
                "biome {}: region {:?} missing from cache",
                biome.id,
                biome.region
            );
            continue;
        };
        let terrain_group = locations_group.saturating_sub(1);
        let terrain = Terrain::load(
            biome.region.0,
            biome.region.1,
            &cache.group(5, terrain_group)?.data,
        )?;
        let locations = Locations::load(
            biome.region.0,
            biome.region.1,
            &cache.group(5, locations_group)?.data,
        )?;
        pocket_data.insert(biome.id.clone(), (terrain, locations));
    }

    let mut pocket_water: Vec<(i32, i32)> = Vec::new();
    // Per-tile biome coverage and variant, so the display mesh can choose
    // the grass variant on a smooth contour per cell instead of dithering
    // whole tiles (which left a scatter of square patches at every edge).
    let mut biome_coverage = vec![0.0f32; size * size];
    let mut biome_variant = vec![0u8; size * size];
    for biome in &map.biomes {
        let Some((terrain, _)) = pocket_data.get(&biome.id) else {
            continue;
        };
        let (rx, ry, pw, ph) = biome.rect;
        for ly in 0..ph {
            for lx in 0..pw {
                let sx = rx + lx;
                let sy = ry + ly;
                if sx >= 64 || sy >= 64 {
                    continue;
                }
                let map_tx = biome.origin.0 + lx as i32;
                let map_ty = biome.origin.1 + ly as i32;
                let gx = map_tx - origin_tile_x;
                let gy = map_ty - origin_tile_y;
                if gx < 0 || gy < 0 || gx >= size as i32 - 1 || gy >= size as i32 - 1 {
                    continue;
                }
                let idx = gy as usize * size + gx as usize;
                let tile = terrain.tile(0, sx as usize, sy as usize);
                // Blend the pocket into the surrounding ground over a wide,
                // noise-warped border. The old seven-tile rectangle read as
                // a flat dark patch of fog/water in the low jungle.
                let edge = lx
                    .min(ly)
                    .min(pw.saturating_sub(1).saturating_sub(lx))
                    .min(ph.saturating_sub(1).saturating_sub(ly)) as f32;
                let wobble = (value_noise(map_tx as f32 * 0.17 + 4.0, map_ty as f32 * 0.17 - 9.0)
                    - 0.5)
                    * 5.0;
                let blend = ((edge + wobble) / 12.0).clamp(0.0, 1.0);
                let blend = blend * blend * (3.0 - 2.0 * blend);
                tile_overlay[idx] = 0;
                if let Some(overlay) = overlays.get(&(tile.overlay_id as u32))
                    && overlay.texture != 0
                {
                    let base_weight = if overlay.hide_underlay { 200.0 } else { 130.0 };
                    let weight = (base_weight * blend).round() as u8;
                    if weight > 0 {
                        tile_overlay[idx] = pack_ground_overlay(overlay.texture, weight);
                    }
                }
                let pocket_h = tile.height as f32 * 8.0;
                heights[idx] = heights[idx] * (1.0 - blend) + pocket_h * blend;
                let c = tile_color(terrain, &underlays, &overlays, sx as usize, sy as usize);
                // Keep the biome hue but lift the darkest antique tints; a
                // 0.5x multiply turned large low pockets into flat dark slabs.
                let tint = [
                    0.70 + biome.tint[0] * 0.35,
                    0.70 + biome.tint[1] * 0.35,
                    0.70 + biome.tint[2] * 0.35,
                ];
                let tinted = [
                    (c[0] as f32 * tint[0]).clamp(0.0, 255.0) as u8,
                    (c[1] as f32 * tint[1]).clamp(0.0, 255.0) as u8,
                    (c[2] as f32 * tint[2]).clamp(0.0, 255.0) as u8,
                ];
                for k in 0..3 {
                    colors[idx][k] = (colors[idx][k] as f32 * (1.0 - blend)
                        + tinted[k] as f32 * blend)
                        .clamp(0.0, 255.0) as u8;
                }
                // Dither the grass variant across the pocket's soft edge so
                // biome transitions read as a natural field change rather
                // than a rectangular tile boundary.
                let coverage = blend.clamp(0.0, 1.0);
                if coverage > biome_coverage[idx] {
                    biome_coverage[idx] = coverage;
                    biome_variant[idx] = biome.grass_variant;
                }
                if coverage >= 1.0
                    || value_noise(map_tx as f32 * 0.55, map_ty as f32 * 0.55) < coverage
                {
                    grass_variant[idx] = biome.grass_variant;
                }
                if tile.underlay_id == 0 {
                    pocket_water.push((gx, gy));
                }
                // The pocket replaces the block's own tile; follow its
                // water/land status where the pocket dominates.
                if blend > 0.5 {
                    sea[idx] = tile.underlay_id == 0 && tile.overlay_id != 0;
                }
            }
        }
    }

    // Pockets are in; reclaim cache sea and pocket ponds as land before the
    // authored river is carved.
    reclaim_sea(
        &mut heights,
        &mut colors,
        &mut tile_overlay,
        &sea,
        size,
        origin_tile_x,
        origin_tile_y,
        map.castle.as_ref(),
    );

    // ------------------------------------------------------------------
    // The river: a diagonal waterway that splits the rift into a radiant
    // half and a dire half, with a walkable crossing in the centre.
    // ------------------------------------------------------------------
    let mut river_blocked: Vec<(i32, i32)> = Vec::new();
    let mut water_mask = vec![0.0f32; size * size];
    let mut water_level = vec![0.0f32; size * size];
    if let Some(river) = &map.river {
        let points = &river.points;
        if points.len() >= 2 {
            let mut levels: Vec<f32> = Vec::with_capacity(points.len());
            for (index, point) in points.iter().enumerate() {
                let gx = point.0.round() as i32 - origin_tile_x;
                let gy = point.1.round() as i32 - origin_tile_y;
                let sampled = if gx >= 0 && gy >= 0 && (gx as usize) < size && (gy as usize) < size
                {
                    heights[gy as usize * size + gx as usize]
                } else {
                    0.0
                };
                let level = match river.levels {
                    Some((start, end)) => {
                        let t = index as f32 / (points.len() - 1).max(1) as f32;
                        start + (end - start) * t
                    }
                    None => sampled,
                };
                levels.push(level);
            }
            // Water always runs downhill.
            for index in 1..levels.len() {
                if levels[index] > levels[index - 1] - 0.5 {
                    levels[index] = levels[index - 1] - 0.5;
                }
            }
            for gy in 0..size - 1 {
                for gx in 0..size - 1 {
                    let tx = (origin_tile_x + gx as i32) as f32;
                    let ty = (origin_tile_y + gy as i32) as f32;
                    let (dist, segment, along) = nearest_on_polyline(points, tx, ty);
                    if dist > river.width * 2.2 {
                        continue;
                    }
                    let level = levels[segment] + (levels[segment + 1] - levels[segment]) * along;
                    let k = 1.0 - smoothstep(river.width * 0.45, river.width * 1.9, dist);
                    if k <= 0.001 {
                        continue;
                    }
                    // Keep the dry bridge shelf narrow. The old 5.5-tile
                    // radius left a large flat green patch around every deck;
                    // the deck itself already covers the walkable crossing.
                    let bridge = river
                        .bridges
                        .iter()
                        .any(|b| (b.0 - tx) * (b.0 - tx) + (b.1 - ty) * (b.1 - ty) < 8.0);
                    let depth = if bridge {
                        river.depth * 0.2
                    } else {
                        river.depth
                    };
                    let idx = gy * size + gx;
                    if !bridge {
                        water_mask[idx] = water_mask[idx].max(k);
                        water_level[idx] = level;
                    }
                    let bed = (level - depth).min(heights[idx]);
                    heights[idx] = heights[idx] * (1.0 - k) + bed * k;
                    // Water blue only where the water sheet actually covers
                    // the bed; the band beyond it becomes a wet-soil bank.
                    // Blue past the sheet was classified as water by the
                    // terrain shader and drew a flat near-black slab along
                    // every shore.
                    let water_k = smoothstep(0.45, 0.8, k);
                    let bank_k = ((k * 1.8).min(1.0) - water_k).max(0.0) * 0.85;
                    let water = [46u8, 94, 132];
                    let bank = [104u8, 90, 60];
                    for c in 0..3 {
                        let base = colors[idx][c] as f32;
                        let banked = base * (1.0 - bank_k) + bank[c] as f32 * bank_k;
                        colors[idx][c] = (banked * (1.0 - water_k) + water[c] as f32 * water_k)
                            .clamp(0.0, 255.0) as u8;
                    }
                    if !bridge && k > 0.55 {
                        river_blocked.push((gx as i32, gy as i32));
                    }
                }
            }
        }
    }

    // Terrain conditioning: the raw cache heightfield is spiky in places and
    // dead flat in others. Relax single-tile spikes, add gentle rolling
    // relief, and level the team courtyards so bases read as built plazas
    // instead of rocky fields. Lanes are re-levelled by the pass below.
    relax_heights(&mut heights, size, &water_mask, 2);
    sculpt_terrain(
        &mut heights,
        size,
        origin_tile_x,
        origin_tile_y,
        &water_mask,
        map.castle.as_ref(),
    );
    flatten_bases(
        &mut heights,
        size,
        origin_tile_x,
        origin_tile_y,
        map.castle.as_ref(),
    );
    // Smooth the per-tile color field before triangulating. Two box-blur
    // passes remove the hard square boundaries between adjacent tiles; the
    // resulting per-corner colors interpolate across the terrain mesh.
    let mut smooth_colors = colors.clone();
    for _ in 0..2 {
        smooth_colors = box_blur_colors(&smooth_colors, size, 2);
    }
    // Lift the darkest ground. Low jungle/riverbank tiles otherwise read as
    // flat dark fog/water against the brighter fields; preserve highlights
    // instead of applying a global gamma that washes out the grass.
    const SHADOW_FLOOR: f32 = 140.0;
    for color in &mut smooth_colors {
        for channel in color {
            let value = *channel as f32;
            let lifted = if value < SHADOW_FLOOR {
                SHADOW_FLOOR - (SHADOW_FLOOR - value) * 0.35
            } else {
                value
            };
            *channel = lifted.clamp(0.0, 255.0) as u8;
        }
    }

    // ------------------------------------------------------------------
    // Lanes are baked into the terrain itself instead of being drawn as a
    // ribbon on top of it: the mask lives in the terrain vertex alpha and
    // the shader blends the OSRS path textures into the ground. That keeps
    // the road seamless with the world and stops the overlay from
    // z-fighting with the terrain.
    // ------------------------------------------------------------------
    const LANE_HALF_WIDTH: f32 = 235.0;
    const LANE_PATH_COLOUR: [u8; 3] = [104, 86, 62];
    let mut lane_mask = vec![0.0f32; size * size];
    for gy in 0..size {
        for gx in 0..size {
            let wx = (origin_tile_x + gx as i32) as f32 * 128.0;
            let wz = (origin_tile_y + gy as i32) as f32 * 128.0;
            let (nearest, distance) = map.nearest_lane_point(wx, wz);
            // A slow noise keeps the road edge from looking ruler straight.
            let wobble = (value_noise(wx * 0.0035, wz * 0.0035) - 0.5) * 190.0;
            let t = ((distance + wobble) / LANE_HALF_WIDTH).clamp(0.0, 2.0);
            let mask = 1.0 - smoothstep(0.78, 1.22, t);
            let idx = gy * size + gx;
            lane_mask[idx] = mask;
            if mask > 0.001 {
                for c in 0..3 {
                    smooth_colors[idx][c] = (smooth_colors[idx][c] as f32 * (1.0 - mask * 0.92)
                        + LANE_PATH_COLOUR[c] as f32 * mask * 0.92)
                        .clamp(0.0, 255.0) as u8;
                }
                // Level the road along the lane centre line.
                let centre = heights[(nearest.y / 128.0 - origin_tile_y as f32)
                    .clamp(0.0, size as f32 - 1.0) as usize
                    * size
                    + (nearest.x / 128.0 - origin_tile_x as f32).clamp(0.0, size as f32 - 1.0)
                        as usize];
                heights[idx] = heights[idx] * (1.0 - mask * 0.88) + centre * mask * 0.88;
            }
        }
    }

    // ------------------------------------------------------------------
    // Display terrain at TERRAIN_SUBDIV vertices per tile. Heights are a
    // Catmull-Rom upsample of the conditioned tile grid plus terraced
    // plateaus evaluated at full resolution, so cliff lips and feet are
    // crisp instead of being smeared across one 128-unit quad. Colours and
    // masks are bilinear; the packed ground identity stays per tile.
    // ------------------------------------------------------------------
    const TERRAIN_SUBDIV: usize = 2;
    let fine_step = 128.0 / TERRAIN_SUBDIV as f32;
    let fine_size = (size - 1) * TERRAIN_SUBDIV + 1;
    let inv = 1.0 / TERRAIN_SUBDIV as f32;
    let origin_wx = origin_tile_x as f32 * 128.0;
    let origin_wz = origin_tile_y as f32 * 128.0;
    let channel = |c: usize| {
        smooth_colors
            .iter()
            .map(|v| v[c] as f32)
            .collect::<Vec<f32>>()
    };
    let (red, green, blue) = (channel(0), channel(1), channel(2));
    let mut fine_heights = vec![0.0f32; fine_size * fine_size];
    let mut fine_colors = vec![[0u8; 3]; fine_size * fine_size];
    let mut fine_lane = vec![0.0f32; fine_size * fine_size];
    let mut fine_water = vec![0.0f32; fine_size * fine_size];
    let mut plateau_cells = 0usize;
    for fy in 0..fine_size {
        for fx in 0..fine_size {
            let gx = fx as f32 * inv;
            let gy = fy as f32 * inv;
            let wx = origin_wx + fx as f32 * fine_step;
            let wz = origin_wz + fy as f32 * fine_step;
            let water_k = sample_bilinear(&water_mask, size, gx, gy);
            let base = sample_bicubic(&heights, size, gx, gy);
            let clearance = if water_k > 0.01 {
                0.0
            } else {
                plateau_clearance(&map, wx, wz)
            };
            let lift = plateau_height(wx, wz, clearance);
            if lift > TERRACE_STEP * 0.5 {
                plateau_cells += 1;
            }
            let i = fy * fine_size + fx;
            fine_heights[i] = base + lift;
            fine_water[i] = water_k;
            fine_lane[i] = sample_bilinear(&lane_mask, size, gx, gy);
            fine_colors[i] = [
                sample_bilinear(&red, size, gx, gy).clamp(0.0, 255.0) as u8,
                sample_bilinear(&green, size, gx, gy).clamp(0.0, 255.0) as u8,
                sample_bilinear(&blue, size, gx, gy).clamp(0.0, 255.0) as u8,
            ];
        }
    }
    tracing::info!(
        "terrain: {}x{} vertices at {} units, {:.1}% plateau",
        fine_size,
        fine_size,
        fine_step,
        plateau_cells as f32 * 100.0 / (fine_size * fine_size) as f32
    );

    // Overlay weight per display cell: a bilinear blend between the
    // centres of neighbouring tiles that carry the same overlay texture.
    // That halves the hard square edge the per-tile weight produced.
    let overlay_for_cell = |fx: usize, fy: usize| -> u32 {
        let tile_x = (fx / TERRAIN_SUBDIV).min(size - 2);
        let tile_y = (fy / TERRAIN_SUBDIV).min(size - 2);
        let own = tile_overlay[tile_y * size + tile_x];
        let texture = (own >> 8) & 0xFF;
        if texture == 0 {
            return 0;
        }
        let u = (fx as f32 + 0.5) * inv - 0.5;
        let v = (fy as f32 + 0.5) * inv - 0.5;
        let x0 = u.floor() as i32;
        let y0 = v.floor() as i32;
        let tu = u - x0 as f32;
        let tv = v - y0 as f32;
        let mut weight = 0.0f32;
        for (dx, dy, w) in [
            (0, 0, (1.0 - tu) * (1.0 - tv)),
            (1, 0, tu * (1.0 - tv)),
            (0, 1, (1.0 - tu) * tv),
            (1, 1, tu * tv),
        ] {
            let x = (x0 + dx).clamp(0, size as i32 - 2) as usize;
            let y = (y0 + dy).clamp(0, size as i32 - 2) as usize;
            let other = tile_overlay[y * size + x];
            if (other >> 8) & 0xFF == texture {
                weight += ((other >> 16) & 0xFF) as f32 * w;
            }
        }
        pack_ground_overlay(texture as u8, weight.round().clamp(0.0, 255.0) as u8)
    };

    let normal_at = |vx: usize, vy: usize| {
        let i = vy * fine_size + vx;
        let h = fine_heights[i];
        let hl = if vx > 0 { fine_heights[i - 1] } else { h };
        let hr = if vx + 1 < fine_size {
            fine_heights[i + 1]
        } else {
            h
        };
        let hd = if vy > 0 {
            fine_heights[i - fine_size]
        } else {
            h
        };
        let hu = if vy + 1 < fine_size {
            fine_heights[i + fine_size]
        } else {
            h
        };
        Vec3::new(hl - hr, 2.0 * fine_step, hd - hu).normalize_or_zero()
    };
    // Cavity AO over a two-cell radius so it measures the same 128-unit
    // neighbourhood as before, plus a cliff-foot term from the plateau.
    let ao_at = |vx: usize, vy: usize| {
        let i = vy * fine_size + vx;
        let h = fine_heights[i];
        let r = TERRAIN_SUBDIV;
        let hl = if vx >= r { fine_heights[i - r] } else { h };
        let hr = if vx + r < fine_size {
            fine_heights[i + r]
        } else {
            h
        };
        let hd = if vy >= r {
            fine_heights[i - r * fine_size]
        } else {
            h
        };
        let hu = if vy + r < fine_size {
            fine_heights[i + r * fine_size]
        } else {
            h
        };
        let cavity = (((hl + hr + hd + hu) * 0.25 - h) / 96.0).clamp(-1.0, 1.0);
        (0.82 - cavity * 0.30).clamp(0.52, 1.0)
    };

    // Packed ground identity for one display cell. Biome edges threshold the
    // bilinear coverage against a slow noise so the variant boundary is an
    // organic contour stepped at cell size, not a salt-and-pepper of tiles.
    let ground_for_cell = |fx: usize, fy: usize| -> u32 {
        let tile_x = (fx / TERRAIN_SUBDIV).min(size - 2);
        let tile_y = (fy / TERRAIN_SUBDIV).min(size - 2);
        let tile = tile_y * size + tile_x;
        let mut variant = grass_variant[tile];
        if biome_coverage[tile] > 0.0 || biome_variant[tile] != 0 {
            let cx = (fx as f32 + 0.5) * inv;
            let cy = (fy as f32 + 0.5) * inv;
            let cover = sample_bilinear(&biome_coverage, size, cx, cy);
            let wx = origin_wx + cx * 128.0;
            let wz = origin_wz + cy * 128.0;
            let edge = value_noise(wx * 0.0021 + 3.0, wz * 0.0021 - 6.0) * 0.7
                + value_noise(wx * 0.0065, wz * 0.0065 + 2.0) * 0.3;
            if cover >= 0.999 || cover > edge {
                variant = biome_variant[tile];
            } else if grass_variant[tile] == biome_variant[tile] {
                // The tile was dithered into the biome; fall back to the
                // open-country default carried by the neighbours.
                variant = 0;
            }
        }
        overlay_for_cell(fx, fy) | variant as u32
    };
    let vertex = |cx: usize, cy: usize, ground: u32| {
        let i = cy * fine_size + cx;
        let p = Vec3::new(
            origin_wx + cx as f32 * fine_step,
            fine_heights[i],
            origin_wz + cy as f32 * fine_step,
        );
        let n = normal_at(cx, cy);
        let mut v = SceneVertex::new(p, n, fine_colors[i]);
        // The lane mask rides in the vertex alpha for the terrain.
        v.color[3] = fine_lane[i];
        // uv.x carries the baked slope so the shader can add cliff detail
        // without warping UVs from the per-fragment normal; uv.y carries the
        // river mask so the road shader can stop the lane soil from painting
        // the river bed.
        v.uv = [(1.0 - n.y).clamp(0.0, 1.0), fine_water[i]];
        v.ao = ao_at(cx, cy);
        v.material = 4;
        // Low byte: grass variant; upper bytes: cache overlay texture and
        // weight. Flat interpolation keeps every triangle of a cell equal.
        v.texture_id = ground;
        v
    };

    // Adaptive resolution. The 64-unit grid quadruples the triangle count,
    // which cost ~2.4 ms of scene GPU time on the 680M, but most of the map
    // is calm floor that a 128-unit quad already reproduces. A tile is
    // refined only where its fine heights leave the coarse quad's plane
    // (cliff lips, risers, banks) or its ground identity changes inside it.
    const _: () = assert!(TERRAIN_SUBDIV == 2, "adaptive terrain assumes 2x");
    const REFINE_ERROR: f32 = 3.0;
    let tiles = size - 1;
    let fine_h = |x: usize, y: usize| fine_heights[y * fine_size + x];
    let mut refine = vec![false; tiles * tiles];
    for ty in 0..tiles {
        for tx in 0..tiles {
            let (x0, y0) = (tx * 2, ty * 2);
            let h00 = fine_h(x0, y0);
            let h20 = fine_h(x0 + 2, y0);
            let h02 = fine_h(x0, y0 + 2);
            let h22 = fine_h(x0 + 2, y0 + 2);
            let mut error = 0.0f32;
            for (dx, dy) in [(1, 0), (0, 1), (1, 1), (2, 1), (1, 2)] {
                let u = dx as f32 * 0.5;
                let v = dy as f32 * 0.5;
                let plane =
                    (h00 * (1.0 - u) + h20 * u) * (1.0 - v) + (h02 * (1.0 - u) + h22 * u) * v;
                error = error.max((fine_h(x0 + dx, y0 + dy) - plane).abs());
            }
            let g = ground_for_cell(x0, y0);
            let mixed = g != ground_for_cell(x0 + 1, y0)
                || g != ground_for_cell(x0, y0 + 1)
                || g != ground_for_cell(x0 + 1, y0 + 1);
            refine[ty * tiles + tx] = error > REFINE_ERROR || mixed;
        }
    }
    let refined = |tx: i64, ty: i64| {
        tx >= 0
            && ty >= 0
            && (tx as usize) < tiles
            && (ty as usize) < tiles
            && refine[ty as usize * tiles + tx as usize]
    };

    let mut terrain_mesh = MeshData::new();
    let mut refined_tiles = 0usize;
    for ty in 0..tiles {
        for tx in 0..tiles {
            let (x0, y0) = (tx * 2, ty * 2);
            if refine[ty * tiles + tx] {
                refined_tiles += 1;
                for fy in y0..y0 + 2 {
                    for fx in x0..x0 + 2 {
                        let ground = ground_for_cell(fx, fy);
                        let v00 = vertex(fx, fy, ground);
                        let v10 = vertex(fx + 1, fy, ground);
                        let v01 = vertex(fx, fy + 1, ground);
                        let v11 = vertex(fx + 1, fy + 1, ground);
                        // Split along the diagonal that best follows the
                        // surface, so cliff lips do not zig-zag.
                        let d0 = (fine_h(fx, fy) - fine_h(fx + 1, fy + 1)).abs();
                        let d1 = (fine_h(fx + 1, fy) - fine_h(fx, fy + 1)).abs();
                        if d0 <= d1 {
                            terrain_mesh.push_triangle(v00, v01, v11);
                            terrain_mesh.push_triangle(v00, v11, v10);
                        } else {
                            terrain_mesh.push_triangle(v00, v01, v10);
                            terrain_mesh.push_triangle(v10, v01, v11);
                        }
                    }
                }
                continue;
            }
            let ground = ground_for_cell(x0, y0);
            let (itx, ity) = (tx as i64, ty as i64);
            // Edges shared with a refined neighbour carry its midpoint, so
            // the coarse tile fans around its centre and stays watertight.
            let left = refined(itx - 1, ity);
            let right = refined(itx + 1, ity);
            let below = refined(itx, ity - 1);
            let above = refined(itx, ity + 1);
            if !(left || right || below || above) {
                let v00 = vertex(x0, y0, ground);
                let v10 = vertex(x0 + 2, y0, ground);
                let v01 = vertex(x0, y0 + 2, ground);
                let v11 = vertex(x0 + 2, y0 + 2, ground);
                let d0 = (fine_h(x0, y0) - fine_h(x0 + 2, y0 + 2)).abs();
                let d1 = (fine_h(x0 + 2, y0) - fine_h(x0, y0 + 2)).abs();
                if d0 <= d1 {
                    terrain_mesh.push_triangle(v00, v01, v11);
                    terrain_mesh.push_triangle(v00, v11, v10);
                } else {
                    terrain_mesh.push_triangle(v00, v01, v10);
                    terrain_mesh.push_triangle(v10, v01, v11);
                }
                continue;
            }
            // Boundary loop in the same winding as the quads above:
            // up the x0 edge, across the top, down the far edge, back.
            let mut ring: Vec<(usize, usize)> = Vec::with_capacity(8);
            ring.push((x0, y0));
            if left {
                ring.push((x0, y0 + 1));
            }
            ring.push((x0, y0 + 2));
            if above {
                ring.push((x0 + 1, y0 + 2));
            }
            ring.push((x0 + 2, y0 + 2));
            if right {
                ring.push((x0 + 2, y0 + 1));
            }
            ring.push((x0 + 2, y0));
            if below {
                ring.push((x0 + 1, y0));
            }
            let centre = vertex(x0 + 1, y0 + 1, ground);
            for k in 0..ring.len() {
                let a = ring[k];
                let b = ring[(k + 1) % ring.len()];
                terrain_mesh.push_triangle(
                    centre,
                    vertex(a.0, a.1, ground),
                    vertex(b.0, b.1, ground),
                );
            }
        }
    }
    tracing::info!(
        "terrain: {:.1}% of tiles refined, {} triangles",
        refined_tiles as f32 * 100.0 / (tiles * tiles) as f32,
        terrain_mesh.indices.len() / 3
    );

    // ------------------------------------------------------------------
    // River water surface. The terrain carves the bed and keeps the water
    // colour; this mesh draws the actual reflective sheet on top. It is a
    // separate blended draw so the shader can do fresnel, sky reflection and
    // animated ripples without changing the opaque terrain path. Build it at
    // half-tile resolution from the water mask so banks are 64 units instead
    // of a 128-unit staircase.
    // ------------------------------------------------------------------
    let mut water_mesh = MeshData::new();
    build_water_surface(
        &mut water_mesh,
        size,
        origin_tile_x,
        origin_tile_y,
        &water_mask,
        &water_level,
    );
    tracing::info!(
        "water surface: {} verts, {} tris",
        water_mesh.vertices.len(),
        water_mesh.indices.len() / 3
    );

    let height = HeightGrid {
        origin_tile_x,
        origin_tile_y,
        step: fine_step,
        size: fine_size,
        heights: fine_heights,
    };

    // Re-seat map features on the final terrain: pockets and the river can
    // shift the ground under a lane or a camp.
    for structure in &mut map.structures {
        structure.pos.y = height.height_at(structure.pos.x, structure.pos.z);
    }
    for camp in &mut map.camps {
        camp.pos.y = height.height_at(camp.pos.x, camp.pos.z);
    }
    for lane in &mut map.lanes {
        for waypoint in &mut lane.waypoints {
            waypoint.y = height.height_at(waypoint.x, waypoint.z);
        }
    }

    // Collision: blocking scenery footprints only. Water is traversable by
    // design: heroes and lane units wade through the shallow river and ponds
    // instead of treating every shoreline as an invisible wall.
    let tiles_x = ((max_x - min_x + 1) * 64) as usize;
    let tiles_y = ((max_y - min_y + 1) * 64) as usize;
    let mut collision_grid = Grid::new(tiles_x, tiles_y);
    let mut sight_grid = Grid::new(tiles_x, tiles_y);

    // Collision is only added for objects that are actually rendered, so the
    // player never walks into an invisible wall from one of the thousands of
    // unrendered OSRS objects in the loaded block.
    let mut def_cache: HashMap<u32, ObjectDefinition> = HashMap::new();

    let center = Vec3::new(
        (center_x as f32 * 64.0 + 32.0) * 128.0,
        height.height_at(
            (center_x as f32 * 64.0 + 32.0) * 128.0,
            (center_y as f32 * 64.0 + 32.0) * 128.0,
        ),
        (center_y as f32 * 64.0 + 32.0) * 128.0,
    );

    // Sort scenery by distance to the player so the near field looks dense and
    // the 900-model budget is spent where it is visible.
    let base_keep_clear: Vec<Vec3> = map
        .camps
        .iter()
        .map(|camp| camp.pos)
        .chain(map.structures.iter().map(|structure| structure.pos))
        .collect();
    let mut candidates = Vec::new();
    let mut decor_candidates = Vec::new();
    for (rx, ry, locations) in &all_locations {
        for loc in &locations.locations {
            let wx = ((*rx as f32 * 64.0) + loc.x as f32 + 0.5) * 128.0;
            let wz = ((*ry as f32 * 64.0) + loc.y as f32 + 0.5) * 128.0;
            let lane_distance = map.distance_to_lane(wx, wz);

            // Ground clutter is selected independently of the tree budget
            // and sorted by distance to a lane, so every lane gets the dense
            // flower/fern/rock layer instead of only the map centre.
            if loc.z == 0 && lane_distance >= DECOR_LANE_CLEAR {
                let data = object_files.get(&loc.id);
                if let Some(data) = data
                    && let Ok(def) = ObjectDefinition::parse(loc.id, data)
                    && def.size_x.max(def.size_y) <= 3
                    && let Some(kind) = GroundDecor::from_definition(&def)
                    && let Some(model_id) = def.first_model()
                    && !DECOR_MODEL_BLOCKLIST.contains(&model_id)
                    && !map.in_biome(
                        *rx as i32 * 64 + loc.x as i32,
                        *ry as i32 * 64 + loc.y as i32,
                    )
                    && !point_in_tile_rects(&castle_keep, wx / 128.0, wz / 128.0)
                    && base_keep_clear
                        .iter()
                        .all(|p| p.distance(Vec3::new(wx, p.y, wz)) >= DECOR_STRUCTURE_CLEAR)
                {
                    let seed = loc
                        .id
                        .wrapping_mul(0x9E37_79B9)
                        .wrapping_add((loc.x as u32) << 16)
                        .wrapping_add(loc.y as u32);
                    decor_candidates.push((
                        lane_distance + hash01(seed) * 900.0,
                        *rx,
                        *ry,
                        *loc,
                        kind,
                        model_id,
                    ));
                }
            }

            if lane_distance < 6.5 * 128.0 {
                continue;
            }
            if point_in_tile_rects(&castle_keep, wx / 128.0, wz / 128.0) {
                continue;
            }
            if base_keep_clear
                .iter()
                .any(|p| p.distance(Vec3::new(wx, p.y, wz)) < 6.0 * 128.0)
            {
                continue;
            }
            let dx = wx - center.x;
            let dz = wz - center.z;
            candidates.push((dx * dx + dz * dz, *rx, *ry, *loc));
        }
    }
    candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    decor_candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut model_mesh = MeshData::new();
    let mut shadow_mesh = MeshData::new();
    let mut shadow_caster_mesh = MeshData::new();
    let mut model_cache: HashMap<u32, Model> = HashMap::new();
    // A cache bridge model replaces the procedural boardwalk.
    let bridge_decks = if map.bridge_model.is_some() {
        0
    } else {
        append_bridge_decks(&mut model_mesh, &map, &height)
    };
    tracing::info!("bridge decks: {} crossings", bridge_decks);
    let mut placements = 0usize;
    let mut decor_placements = 0usize;
    const MODEL_LIMIT: usize = 900;

    for (_, rx, ry, loc) in candidates {
        if placements >= MODEL_LIMIT {
            break;
        }
        let Some(terrain) = terrains.get(&(rx, ry)) else {
            continue;
        };
        let Some(data) = object_files.get(&loc.id) else {
            continue;
        };
        let Ok(def) = ObjectDefinition::parse(loc.id, data) else {
            continue;
        };
        if !is_scenery(&def) {
            continue;
        }
        let gx = rx as i32 * 64 + loc.x as i32 - origin_tile_x;
        let gy = ry as i32 * 64 + loc.y as i32 - origin_tile_y;
        if gx >= 0
            && gy >= 0
            && (gx as usize) < size
            && (gy as usize) < size
            && water_mask[gy as usize * size + gx as usize] > 0.03
        {
            continue;
        }
        let Some(model_id) = def.first_model() else {
            continue;
        };
        let model = cached_model(cache, &mut model_cache, model_id)?;

        let base = Vec3::new(
            ((rx as f32 * 64.0) + loc.x as f32 + 0.5) * 128.0,
            terrain.height(loc.z as usize, loc.x as usize, loc.y as usize) as f32 * 8.0,
            ((ry as f32 * 64.0) + loc.y as f32 + 0.5) * 128.0,
        );
        let angle = loc.rotation as f32 * std::f32::consts::FRAC_PI_2;
        append_model_scaled_tinted(
            &mut model_mesh,
            model,
            base,
            angle,
            NATURAL_SCENERY_SCALE,
            [1.0, 1.0, 1.0],
            scenery_texture_hint(&def, is_plant_object(&def)),
            is_dead_tree(&def),
        );
        append_model_caster(
            &mut shadow_caster_mesh,
            model,
            base,
            angle,
            NATURAL_SCENERY_SCALE,
            CasterStyle::Tree,
        );
        let shadow_size =
            (def.size_x.max(def.size_y) as f32 * 128.0 * 0.55 * NATURAL_SCENERY_SCALE)
                .clamp(70.0, 440.0);
        push_blob_quad(&mut shadow_mesh, base, shadow_size, 0.24);
        // What you can see is what blocks: collide with the rendered prop.
        let mut w = def.size_x.max(1) as i32;
        let mut h = def.size_y.max(1) as i32;
        if loc.rotation % 2 == 1 {
            std::mem::swap(&mut w, &mut h);
        }
        let gx = (rx as i32 * 64 + loc.x as i32) - origin_tile_x;
        let gy = (ry as i32 * 64 + loc.y as i32) - origin_tile_y;
        collision_grid.block_rect(gx, gy, w, h);
        sight_grid.block_rect(gx, gy, w, h);
        placements += 1;
    }

    // ------------------------------------------------------------------
    // Ground decoration: the cache's flower beds, ferns, grass tufts,
    // mushrooms, loose rocks and twigs. These are rendered without collision
    // or sight blocking, and the budget is sorted by distance to a lane so
    // the detail follows the player.
    // ------------------------------------------------------------------
    for (_, rx, ry, loc, kind, model_id) in decor_candidates {
        if decor_placements >= DECOR_LIMIT {
            break;
        }
        let Some(terrain) = terrains.get(&(rx, ry)) else {
            continue;
        };
        let tile = terrain.tile(0, loc.x as usize, loc.y as usize);
        if tile.underlay_id == 0 {
            continue;
        }
        let gx = rx as i32 * 64 + loc.x as i32 - origin_tile_x;
        let gy = ry as i32 * 64 + loc.y as i32 - origin_tile_y;
        if gx < 0 || gy < 0 || gx as usize >= size || gy as usize >= size {
            continue;
        }
        if water_mask[gy as usize * size + gx as usize] > 0.03 {
            continue;
        }
        let model = cached_model(cache, &mut model_cache, model_id)?;
        let (model_min, model_max) = model.bounds();
        let raw_height = (model_max[1] - model_min[1]).max(0) as f32;
        let wx = ((rx as f32 * 64.0) + loc.x as f32 + 0.5) * 128.0;
        let wz = ((ry as f32 * 64.0) + loc.y as f32 + 0.5) * 128.0;
        let base = Vec3::new(wx, height.height_at(wx, wz), wz);
        let seed = loc
            .id
            .wrapping_mul(0x9E37_79B9)
            .wrapping_add((loc.x as u32) << 16)
            .wrapping_add(loc.y as u32);
        let scale = 0.9 + hash01(seed) * 0.5;
        if raw_height * scale > 150.0 {
            continue;
        }
        let angle = loc.rotation as f32 * std::f32::consts::FRAC_PI_2;
        append_ground_decor(&mut model_mesh, model, base, angle, scale, kind, seed);
        let shadow = kind.shadow_size() * scale;
        if shadow > 0.0 {
            push_blob_quad(&mut shadow_mesh, base, shadow, 0.16);
        }
        decor_placements += 1;

        // A second, jittered copy of roughly half the props. Real object
        // lists are sparse (about one decoration every sixty tiles), while
        // the reference ground is carpeted. The clone is cheap: same decoded
        // model, no collision, and skipped when it would land in water.
        let clone_seed = seed.wrapping_mul(0x85EB_CA6B).wrapping_add(0x27D4_EB2F);
        if hash01(clone_seed) < 0.55 {
            let ox = (hash01(clone_seed << 1) - 0.5) * 2.1 * 128.0;
            let oz = (hash01(clone_seed << 3) - 0.5) * 2.1 * 128.0;
            let cx = wx + ox;
            let cz = wz + oz;
            let cgx = (cx / 128.0).floor() as i32 - origin_tile_x;
            let cgy = (cz / 128.0).floor() as i32 - origin_tile_y;
            if cgx >= 0
                && cgy >= 0
                && (cgx as usize) < size
                && (cgy as usize) < size
                && water_mask[cgy as usize * size + cgx as usize] <= 0.03
            {
                let cbase = Vec3::new(cx, height.height_at(cx, cz), cz);
                let cscale = scale * (0.72 + hash01(clone_seed << 5) * 0.5);
                let cangle = angle + (hash01(clone_seed << 7) - 0.5) * std::f32::consts::FRAC_PI_2;
                append_ground_decor(
                    &mut model_mesh,
                    model,
                    cbase,
                    cangle,
                    cscale,
                    kind,
                    clone_seed,
                );
                let cshadow = kind.shadow_size() * cscale;
                if cshadow > 0.0 {
                    push_blob_quad(&mut shadow_mesh, cbase, cshadow, 0.14);
                }
                decor_placements += 1;
            }
        }
    }
    tracing::info!("ground decor: {} props", decor_placements);

    // ------------------------------------------------------------------
    // Open-ground scatter. The cache location list is sparse (about one
    // decoration every sixty tiles), while the reference ground is
    // carpeted. Seed small cache-native flowers, ferns, grass, mushrooms
    // and thistles over eligible grass, using a deterministic meadow mask
    // to cluster them and leaving lanes, water, courtyards and structure
    // clearings bare.
    // ------------------------------------------------------------------
    // Low-poly cache plants only: the lush look needs thousands of props,
    // and the 200+ face flower/grass variants are reserved for the sparser
    // location-driven decor above.
    let clutter_objects: &[u32] = &[1187, 1192, 1173, 1298, 1299, 1300, 1163, 5135, 5136, 3795];
    let mut clutter_templates: Vec<(u32, GroundDecor)> = Vec::new();
    for id in clutter_objects {
        let Some(data) = object_files.get(id) else {
            continue;
        };
        let Ok(def) = ObjectDefinition::parse(*id, data) else {
            continue;
        };
        let Some(kind) = GroundDecor::from_definition(&def) else {
            continue;
        };
        let Some(model_id) = def.first_model() else {
            continue;
        };
        if !clutter_templates
            .iter()
            .any(|(existing, _)| *existing == model_id)
        {
            clutter_templates.push((model_id, kind));
        }
    }
    let mut scattered = 0usize;
    if !clutter_templates.is_empty() {
        for gy in 4..size.saturating_sub(4) {
            for gx in 4..size.saturating_sub(4) {
                let seed = (gx as u32)
                    .wrapping_mul(0x9E37_79B9)
                    .wrapping_add((gy as u32) << 16)
                    .wrapping_add(0x51ED_270B);
                let wx = (origin_tile_x + gx as i32) as f32 * 128.0 + (hash01(seed) - 0.5) * 118.0;
                let wz = (origin_tile_y + gy as i32) as f32 * 128.0
                    + (hash01(seed ^ 0xA53C_9E71) - 0.5) * 118.0;
                let idx = gy * size + gx;
                if water_mask[idx] > 0.03 {
                    continue;
                }
                let meadow = value_noise(wx * 0.0013 + 19.0, wz * 0.0013 - 7.0);
                let chance = 0.010 + meadow * 0.030;
                if hash01(seed ^ 0xD1B5_4A35) > chance {
                    continue;
                }
                if map.distance_to_lane(wx, wz) < 2.2 * 128.0 {
                    continue;
                }
                if point_in_tile_rects(&castle_keep, wx / 128.0, wz / 128.0) {
                    continue;
                }
                if base_keep_clear
                    .iter()
                    .any(|p| p.distance(Vec3::new(wx, p.y, wz)) < 2.4 * 128.0)
                {
                    continue;
                }
                // Keep steep cliff faces clean; scattered plants belong on
                // soil and grass shelves.
                let h = height.height_at(wx, wz);
                let slope = ((height.height_at(wx + 128.0, wz) - h).abs()
                    + (height.height_at(wx - 128.0, wz) - h).abs()
                    + (height.height_at(wx, wz + 128.0) - h).abs()
                    + (height.height_at(wx, wz - 128.0) - h).abs())
                    / (4.0 * 128.0);
                if slope > 0.48 {
                    continue;
                }
                let template_index = ((hash01(seed ^ 0x2F6E_2B1D) * clutter_templates.len() as f32)
                    as usize)
                    .min(clutter_templates.len() - 1);
                let (model_id, kind) = clutter_templates[template_index];
                let model = cached_model(cache, &mut model_cache, model_id)?;
                let (model_min, model_max) = model.bounds();
                let raw_height = (model_max[1] - model_min[1]).max(0) as f32;
                let scale = 0.72 + hash01(seed ^ 0x8A5C_D3E9) * 0.52;
                if raw_height * scale > 150.0 {
                    continue;
                }
                let base = Vec3::new(wx, height.height_at(wx, wz), wz);
                let angle = hash01(seed ^ 0x7B1D_3C4F) * std::f32::consts::TAU;
                append_ground_decor(&mut model_mesh, model, base, angle, scale, kind, seed);
                let shadow = kind.shadow_size() * scale;
                if shadow > 0.0 {
                    push_blob_quad(&mut shadow_mesh, base, shadow, 0.14);
                }
                scattered += 1;
            }
        }
    }
    tracing::info!("open-ground scatter: {} props", scattered);
    decor_placements += scattered;

    // Boulders break up the sculpted hills. They belong on slopes, unlike
    // the flower/fern layer above, so they get their own cheaper pass.
    let boulder_models = [685u32, 1805, 1773];
    let mut boulders = 0usize;
    for gy in 4..size.saturating_sub(4) {
        for gx in 4..size.saturating_sub(4) {
            let seed = (gx as u32)
                .wrapping_mul(0x85EB_CA6B)
                .wrapping_add((gy as u32) << 17)
                .wrapping_add(0x27D4_EB2F);
            let idx = gy * size + gx;
            if water_mask[idx] > 0.03 {
                continue;
            }
            let wx = (origin_tile_x + gx as i32) as f32 * 128.0 + (hash01(seed) - 0.5) * 96.0;
            let wz = (origin_tile_y + gy as i32) as f32 * 128.0
                + (hash01(seed ^ 0x51ED_270B) - 0.5) * 96.0;
            if map.distance_to_lane(wx, wz) < 2.4 * 128.0 {
                continue;
            }
            if point_in_tile_rects(&castle_keep, wx / 128.0, wz / 128.0) {
                continue;
            }
            if base_keep_clear
                .iter()
                .any(|p| p.distance(Vec3::new(wx, p.y, wz)) < 2.4 * 128.0)
            {
                continue;
            }
            let h = height.height_at(wx, wz);
            let slope = ((height.height_at(wx + 128.0, wz) - h).abs()
                + (height.height_at(wx - 128.0, wz) - h).abs()
                + (height.height_at(wx, wz + 128.0) - h).abs()
                + (height.height_at(wx, wz - 128.0) - h).abs())
                / (4.0 * 128.0);
            if !(0.10..=0.58).contains(&slope) {
                continue;
            }
            if hash01(seed ^ 0xD1B5_4A35) > 0.0045 {
                continue;
            }
            let model_index = ((hash01(seed ^ 0x2F6E_2B1D) * boulder_models.len() as f32) as usize)
                .min(boulder_models.len() - 1);
            let model_id = boulder_models[model_index];
            let model = cached_model(cache, &mut model_cache, model_id)?;
            let scale = 0.62 + hash01(seed ^ 0x8A5C_D3E9) * 0.72;
            let base = Vec3::new(wx, h, wz);
            let angle = hash01(seed ^ 0x7B1D_3C4F) * std::f32::consts::TAU;
            append_ground_decor(
                &mut model_mesh,
                model,
                base,
                angle,
                scale,
                GroundDecor::Rock,
                seed,
            );
            push_blob_quad(
                &mut shadow_mesh,
                base,
                GroundDecor::Rock.shadow_size() * scale,
                0.16,
            );
            boulders += 1;
        }
    }
    tracing::info!("slope boulders: {} props", boulders);
    decor_placements += boulders;

    // ------------------------------------------------------------------
    // Shoreline reeds and ferns. The low riverbanks were large flat dark
    // patches because the location-driven decor is sparse there. A bank
    // pass with cache reed/fern/grass models gives the water a natural edge.
    // ------------------------------------------------------------------
    let shore_models: &[(u32, GroundDecor)] = &[
        (1674, GroundDecor::Grass),
        (1696, GroundDecor::Fern),
        (1707, GroundDecor::Fern),
        (3795, GroundDecor::Grass),
        (1610, GroundDecor::Flower),
        (1686, GroundDecor::Mushroom),
    ];
    let mut shoreline = 0usize;
    for gy in 4..size.saturating_sub(4) {
        for gx in 4..size.saturating_sub(4) {
            let idx = gy * size + gx;
            if water_mask[idx] > 0.05 {
                continue;
            }
            let mut near_water = false;
            for dy in -2i32..=2 {
                for dx in -2i32..=2 {
                    let sx = gx as i32 + dx;
                    let sy = gy as i32 + dy;
                    if sx < 0 || sy < 0 || sx >= size as i32 || sy >= size as i32 {
                        continue;
                    }
                    if water_mask[sy as usize * size + sx as usize] > 0.18 {
                        near_water = true;
                        break;
                    }
                }
                if near_water {
                    break;
                }
            }
            if !near_water {
                continue;
            }
            let seed = (gx as u32)
                .wrapping_mul(0xD1B5_4A35)
                .wrapping_add((gy as u32) << 19)
                .wrapping_add(0x85EB_CA6B);
            if hash01(seed) > 0.16 {
                continue;
            }
            let wx = (origin_tile_x + gx as i32) as f32 * 128.0
                + (hash01(seed ^ 0x9E37_79B9) - 0.5) * 112.0;
            let wz = (origin_tile_y + gy as i32) as f32 * 128.0
                + (hash01(seed ^ 0x27D4_EB2F) - 0.5) * 112.0;
            if map.distance_to_lane(wx, wz) < 1.6 * 128.0 {
                continue;
            }
            if point_in_tile_rects(&castle_keep, wx / 128.0, wz / 128.0) {
                continue;
            }
            if base_keep_clear
                .iter()
                .any(|p| p.distance(Vec3::new(wx, p.y, wz)) < 2.2 * 128.0)
            {
                continue;
            }
            let template = shore_models[((hash01(seed ^ 0x2F6E_2B1D) * shore_models.len() as f32)
                as usize)
                .min(shore_models.len() - 1)];
            let model = cached_model(cache, &mut model_cache, template.0)?;
            let base = Vec3::new(wx, height.height_at(wx, wz), wz);
            let scale = 0.72 + hash01(seed ^ 0x8A5C_D3E9) * 0.55;
            let angle = hash01(seed ^ 0x7B1D_3C4F) * std::f32::consts::TAU;
            append_ground_decor(&mut model_mesh, model, base, angle, scale, template.1, seed);
            let shadow = template.1.shadow_size() * scale;
            if shadow > 0.0 {
                push_blob_quad(&mut shadow_mesh, base, shadow, 0.14);
            }
            shoreline += 1;
        }
    }
    tracing::info!("shoreline scatter: {} props", shoreline);
    decor_placements += shoreline;

    // ------------------------------------------------------------------
    // Biome pocket scenery: real OSRS objects taken from each pocket's
    // source region and transplanted into the map.
    // ------------------------------------------------------------------
    let keep_clear: Vec<Vec3> = map
        .camps
        .iter()
        .map(|camp| camp.pos)
        .chain(map.structures.iter().map(|structure| structure.pos))
        .collect();
    let mut flat_planes_skipped = 0usize;
    for biome in &map.biomes {
        let Some((_, locations)) = pocket_data.get(&biome.id) else {
            continue;
        };
        let (rx, ry, pw, ph) = biome.rect;
        let center_x = biome.origin.0 as f32 + pw as f32 * 0.5;
        let center_y = biome.origin.1 as f32 + ph as f32 * 0.5;
        let mut candidates: Vec<(u8, f32, u32, u32, u32, u32, u8)> = Vec::new();
        for loc in &locations.locations {
            let loc_x = loc.x as u32;
            let loc_y = loc.y as u32;
            if loc_x < rx || loc_y < ry || loc_x >= rx + pw || loc_y >= ry + ph {
                continue;
            }
            if let std::collections::hash_map::Entry::Vacant(entry) = def_cache.entry(loc.id) {
                if let Some(data) = object_files.get(&loc.id) {
                    if let Ok(def) = ObjectDefinition::parse(loc.id, data) {
                        entry.insert(def);
                    }
                }
            }
            let Some(def) = def_cache.get(&loc.id) else {
                continue;
            };
            if !is_pocket_scenery(def) {
                continue;
            }
            let Some(model_id) = def.first_model() else {
                continue;
            };
            let dx = loc.x as f32 + 0.5 - center_x;
            let dy = loc.y as f32 + 0.5 - center_y;
            candidates.push((
                scenery_priority(def),
                dx * dx + dy * dy,
                loc.id,
                loc_x,
                loc_y,
                model_id,
                loc.rotation,
            ));
        }
        candidates.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        });

        let mut placed = 0usize;
        for (_, _, object_id, sx, sy, model_id, rotation) in candidates {
            if placed >= biome.budget {
                break;
            }
            let Some(def) = def_cache.get(&object_id) else {
                continue;
            };
            let model = if let Some(model) = model_cache.get(&model_id) {
                model
            } else {
                let group = cache.group(7, model_id)?;
                let model = Model::load(model_id, &group.data)?;
                model_cache.insert(model_id, model);
                model_cache.get(&model_id).unwrap()
            };
            if is_flat_ground_plane(model) {
                flat_planes_skipped += 1;
                continue;
            }
            let map_tx = biome.origin.0 + sx as i32 - rx as i32;
            let map_ty = biome.origin.1 + sy as i32 - ry as i32;
            let wx = (map_tx as f32 + 0.5) * 128.0;
            let wz = (map_ty as f32 + 0.5) * 128.0;
            if !height.contains(wx, wz) {
                continue;
            }
            // Pockets can overlap a lane; never drop a door or a wall in the
            // middle of the road, and keep camps/towers free of props so a
            // chair never ends up glued to a jungle creature.
            if map.distance_to_lane(wx, wz) < 6.5 * 128.0 {
                continue;
            }
            if point_in_tile_rects(&castle_keep, wx / 128.0, wz / 128.0) {
                continue;
            }
            if keep_clear
                .iter()
                .any(|p| p.distance(Vec3::new(wx, p.y, wz)) < 6.0 * 128.0)
            {
                continue;
            }
            let base = Vec3::new(wx, height.height_at(wx, wz), wz);
            let angle = rotation as f32 * std::f32::consts::FRAC_PI_2;
            append_model_scaled_tinted(
                &mut model_mesh,
                model,
                base,
                angle,
                POCKET_SCENERY_SCALE,
                biome.tint,
                scenery_texture_hint(def, is_plant_object(def)),
                is_dead_tree(def),
            );
            append_model_caster(
                &mut shadow_caster_mesh,
                model,
                base,
                angle,
                POCKET_SCENERY_SCALE,
                if is_plant_object(def) {
                    CasterStyle::Tree
                } else {
                    CasterStyle::Building
                },
            );
            let shadow_size =
                (def.size_x.max(def.size_y) as f32 * 128.0 * 0.55 * POCKET_SCENERY_SCALE)
                    .clamp(65.0, 420.0);
            push_blob_quad(&mut shadow_mesh, base, shadow_size, 0.24);

            let mut w = def.size_x.max(1) as i32;
            let mut h = def.size_y.max(1) as i32;
            if rotation % 2 == 1 {
                std::mem::swap(&mut w, &mut h);
            }
            collision_grid.block_rect(map_tx - origin_tile_x, map_ty - origin_tile_y, w, h);
            sight_grid.block_rect(map_tx - origin_tile_x, map_ty - origin_tile_y, w, h);
            placed += 1;
        }
        placements += placed;
        tracing::info!(
            "biome {} ({}): {} models from region {:?}",
            biome.id,
            biome.side,
            placed,
            biome.region
        );
    }

    // Lanes stay walkable even where a biome pocket overlaps them, and the
    // river is only fordable where a lane crosses it.
    map.clear_collision(&mut collision_grid, origin_tile_x, origin_tile_y, 4.5);
    map.block_structures(&mut collision_grid, origin_tile_x, origin_tile_y);
    map.block_structures(&mut sight_grid, origin_tile_x, origin_tile_y);
    // What stays shut even inside a Web Nest: structures and castle walls.
    let mut solid_grid = Grid::new(tiles_x, tiles_y);
    map.block_structures(&mut solid_grid, origin_tile_x, origin_tile_y);

    // Castle walls, towers and gatehouses. Collision is applied after the
    // lane clearing pass so the walls always win, while gate pieces stay
    // walkable and the lanes can pass through them.
    for piece in &castle_pieces {
        if !piece.block {
            continue;
        }
        let gx = piece.tile[0].round() as i32 - origin_tile_x;
        let gy = piece.tile[1].round() as i32 - origin_tile_y;
        let footprint = piece.footprint.round().max(1.0) as i32;
        let half = footprint / 2;
        collision_grid.block_rect(gx - half, gy - half, footprint, footprint);
        sight_grid.block_rect(gx - half, gy - half, footprint, footprint);
        solid_grid.block_rect(gx - half, gy - half, footprint, footprint);
    }
    let castle_count = append_castle_mesh(
        &castle_pieces,
        &height,
        &mut model_mesh,
        &mut shadow_caster_mesh,
    );
    placements += castle_count;
    tracing::info!(
        "castle: {} wall/tower/platform pieces ({} blocked footprints)",
        castle_count,
        castle_pieces.iter().filter(|piece| piece.block).count()
    );
    tracing::info!(
        "river tiles blocked: {}, pocket tiles: {}",
        river_blocked.len(),
        pocket_water.len()
    );

    tracing::info!("pocket scenery: skipped {flat_planes_skipped} flat ground planes");
    // ------------------------------------------------------------------
    // Tree lines: OSRS tree models form a natural barrier along both sides
    // of every lane. Spacing, distance, scale and species are seeded from a
    // fixed RNG so the forest looks hand scattered but every match is the
    // same world.
    // ------------------------------------------------------------------
    let structure_positions: Vec<Vec3> = map.structures.iter().map(|s| s.pos).collect();
    let tree_context = TreeContext {
        cache,
        map: &map,
        height: &height,
        structures: &structure_positions,
        river_blocked: &river_blocked,
        origin: (origin_tile_x, origin_tile_y),
    };
    let mut tree_count = 0usize;
    let mut prop_count = 0usize;
    let mut rng = Rng::new(0x5EED_9E37_79B9_7F4A);
    {
        let mut sink = ScenerySink {
            model_cache: &mut model_cache,
            model_mesh: &mut model_mesh,
            shadow_mesh: &mut shadow_mesh,
            shadow_caster_mesh: &mut shadow_caster_mesh,
            collision_grid: &mut collision_grid,
        };
        for lane in &map.lanes {
            for pair in lane.waypoints.windows(2) {
                let a = pair[0];
                let b = pair[1];
                let delta = b - a;
                let length = delta.length();
                if length < 1.0 {
                    continue;
                }
                let dir = delta / length;
                let perp = Vec3::new(-dir.z, 0.0, dir.x);
                let mut travelled = rng.range(0.0, 220.0);
                while travelled < length {
                    let point = a + dir * travelled;
                    for side in [-1.0f32, 1.0] {
                        // Leave occasional gaps so the tree wall is not a
                        // perfect hedge.
                        if rng.chance(0.12) {
                            continue;
                        }
                        let offset = rng.range(5.4, 10.5) * 128.0;
                        let along = rng.range(-1.4, 1.4) * 128.0;
                        let pos = point + perp * (offset * side) + dir * along;
                        let seed = rng.next_u32();
                        if rng.chance(0.30) {
                            place_lane_prop(&tree_context, &mut sink, pos, seed, &mut prop_count);
                        } else {
                            place_tree(&tree_context, &mut sink, pos, seed, &mut tree_count);
                        }
                        // Occasional cluster behind the first tree.
                        if rng.chance(0.10) {
                            let extra = point
                                + perp * (rng.range(6.5, 12.0) * 128.0 * side)
                                + dir * rng.range(-1.6, 1.6) * 128.0;
                            let extra_seed = rng.next_u32();
                            if rng.chance(0.30) {
                                place_lane_prop(
                                    &tree_context,
                                    &mut sink,
                                    extra,
                                    extra_seed,
                                    &mut prop_count,
                                );
                            } else {
                                place_tree(
                                    &tree_context,
                                    &mut sink,
                                    extra,
                                    extra_seed,
                                    &mut tree_count,
                                );
                            }
                        }
                    }
                    travelled += rng.range(380.0, 880.0);
                }
            }
        }
    }
    tracing::info!(
        "tree lines: {} trees, {} boulders/bushes",
        tree_count,
        prop_count
    );

    // ------------------------------------------------------------------
    // Cliff dressing. Terrace risers are steeper than the boulder pass's
    // slope window, so cliffs were bare grey ramps and plateau tops were
    // empty lawns. Half-buried cache boulders break up each riser, and tree
    // clusters on the plateau tops give the raised jungle the dark canopy
    // masses of the reference. Deterministic per tile.
    // ------------------------------------------------------------------
    let mut cliff_rocks = 0usize;
    let mut plateau_trees = 0usize;
    {
        // Only the grey textured boulder: the other boulder models render as
        // pale cream domes at outcrop scale.
        const CLIFF_ROCKS: [u32; 1] = [1805];
        let mut sink = ScenerySink {
            model_cache: &mut model_cache,
            model_mesh: &mut model_mesh,
            shadow_mesh: &mut shadow_mesh,
            shadow_caster_mesh: &mut shadow_caster_mesh,
            collision_grid: &mut collision_grid,
        };
        let slope_at = |wx: f32, wz: f32| {
            let h = height.height_at(wx, wz);
            ((height.height_at(wx + 96.0, wz) - h).abs()
                + (height.height_at(wx - 96.0, wz) - h).abs()
                + (height.height_at(wx, wz + 96.0) - h).abs()
                + (height.height_at(wx, wz - 96.0) - h).abs())
                / (4.0 * 96.0)
        };
        for gy in 3..size.saturating_sub(4) {
            for gx in 3..size.saturating_sub(4) {
                let seed = (gx as u32)
                    .wrapping_mul(0x9E37_79B1)
                    .wrapping_add((gy as u32).wrapping_mul(0x85EB_CA77))
                    .wrapping_add(0xC2B2_AE3D);
                if water_mask[gy * size + gx] > 0.03 {
                    continue;
                }
                let wx = ((origin_tile_x + gx as i32) as f32 + 0.5) * 128.0
                    + (hash01(seed) - 0.5) * 96.0;
                let wz = ((origin_tile_y + gy as i32) as f32 + 0.5) * 128.0
                    + (hash01(seed ^ 0x51ED_270B) - 0.5) * 96.0;
                if map.distance_to_lane(wx, wz) < 3.0 * 128.0
                    || point_in_tile_rects(&castle_keep, wx / 128.0, wz / 128.0)
                {
                    continue;
                }
                let slope = slope_at(wx, wz);
                if slope > 0.7 {
                    if hash01(seed ^ 0xD1B5_4A35) > 0.07 {
                        continue;
                    }
                    let model_id = CLIFF_ROCKS[(seed as usize >> 3) % CLIFF_ROCKS.len()];
                    let model = cached_model(cache, sink.model_cache, model_id)?;
                    let scale = 0.6 + hash01(seed ^ 0x8A5C_D3E9) * 0.55;
                    let h = height.height_at(wx, wz);
                    // Sink the rock into the face so it reads as an outcrop
                    // rather than a ball resting on a ramp.
                    let base = Vec3::new(wx, h - 30.0 * scale, wz);
                    let angle = hash01(seed ^ 0x7B1D_3C4F) * std::f32::consts::TAU;
                    append_ground_decor(
                        sink.model_mesh,
                        model,
                        base,
                        angle,
                        scale,
                        GroundDecor::Rock,
                        seed,
                    );
                    cliff_rocks += 1;
                } else if slope < 0.12 {
                    // Plateau tops only: well above the floor and at least a
                    // tile and a half back from the lip.
                    let clearance = plateau_clearance(&map, wx, wz);
                    if plateau_height(wx, wz, clearance) < TERRACE_STEP * 0.92 {
                        continue;
                    }
                    let h = height.height_at(wx, wz);
                    let back_from_lip = [(192.0, 0.0), (-192.0, 0.0), (0.0, 192.0), (0.0, -192.0)]
                        .iter()
                        .all(|(dx, dz)| height.height_at(wx + dx, wz + dz) > h - 40.0);
                    if !back_from_lip {
                        continue;
                    }
                    let cluster = value_noise(wx * 0.0011 + 5.0, wz * 0.0011 - 3.0);
                    let density = smoothstep(0.42, 0.72, cluster) * 0.34;
                    if hash01(seed ^ 0x2F6E_2B1D) > density {
                        continue;
                    }
                    place_tree(
                        &tree_context,
                        &mut sink,
                        Vec3::new(wx, h, wz),
                        seed,
                        &mut plateau_trees,
                    );
                }
            }
        }
    }
    tracing::info!("cliff dressing: {cliff_rocks} riser rocks, {plateau_trees} plateau trees");

    // ------------------------------------------------------------------
    // Castle wall ruins: broken OSRS walls and ruin clusters ring the
    // outside of every lane, scattered with the same fixed seed.
    // ------------------------------------------------------------------
    const RUIN_MODELS: &[(u32, i32)] = &[(16918, 2), (16918, 2), (9233, 2), (9625, 3)];
    let mut ruin_count = 0usize;
    {
        let mut sink = ScenerySink {
            model_cache: &mut model_cache,
            model_mesh: &mut model_mesh,
            shadow_mesh: &mut shadow_mesh,
            shadow_caster_mesh: &mut shadow_caster_mesh,
            collision_grid: &mut collision_grid,
        };
        for (lane_index, lane) in map.lanes.iter().enumerate() {
            for pair in lane.waypoints.windows(2) {
                let a = pair[0];
                let b = pair[1];
                let delta = b - a;
                let length = delta.length();
                if length < 1.0 {
                    continue;
                }
                let dir = delta / length;
                let perp = Vec3::new(-dir.z, 0.0, dir.x);
                let mut travelled = rng.range(0.0, 900.0);
                while travelled < length {
                    let point = a + dir * travelled;
                    if rng.chance(0.28) {
                        travelled += rng.range(700.0, 1800.0);
                        continue;
                    }
                    let side = if (rng.next_u32() as usize + lane_index) % 2 == 0 {
                        -1.0
                    } else {
                        1.0
                    };
                    let offset = rng.range(10.5, 15.0) * 128.0;
                    let along = rng.range(-2.0, 2.0) * 128.0;
                    let (model_id, footprint) =
                        RUIN_MODELS[rng.next_u32() as usize % RUIN_MODELS.len()];
                    place_ruin(
                        &tree_context,
                        &mut sink,
                        point + perp * (offset * side) + dir * along,
                        rng.next_u32(),
                        model_id,
                        footprint,
                        &mut ruin_count,
                    );
                    travelled += rng.range(650.0, 1900.0);
                }
            }
        }
    }
    tracing::info!("castle ruins: {} pieces", ruin_count);

    // ------------------------------------------------------------------
    // Arena set dressing (Phase 5): the map's `props` block, then cache
    // plank pieces over every bridge deck.
    // ------------------------------------------------------------------
    {
        let world = props::PropWorld {
            cache,
            map: &map,
            height: &height,
            structures: tree_context.structures,
            river_blocked: tree_context.river_blocked,
            origin: tree_context.origin,
            water_level: &water_level,
            water_mask: &water_mask,
            size,
        };
        let mut sink = props::PropSink {
            model_cache: &mut model_cache,
            model_mesh: &mut model_mesh,
            shadow_mesh: &mut shadow_mesh,
            shadow_caster_mesh: &mut shadow_caster_mesh,
            collision_grid: &mut collision_grid,
            sight_grid: &mut sight_grid,
        };
        let counts = props::place_props(&map.props, &world, &mut sink);
        let planks = map
            .bridge_model
            .map(|bridge| props::place_bridge_planks(bridge, &world, &mut sink))
            .unwrap_or(0);
        tracing::info!(
            "props: {} placed, {} along lines, {} scattered, {} bridge planks",
            counts.placed,
            counts.lined,
            counts.scattered,
            planks
        );
    }

    let collision = WorldCollision::new(collision_grid, sight_grid, origin_tile_x, origin_tile_y)
        .with_solid(solid_grid);
    // A camp placed on scenery can't walk home or be reached in melee.
    for camp in &map.camps {
        let (x, z) = (camp.pos.x, camp.pos.z);
        if !collision.is_walkable_world(x, z) {
            tracing::warn!(
                "map: camp {} stands on a blocked tile {:?}; nearest open {:?}",
                camp.id,
                collision.tile_of(x, z),
                collision.nearest_walkable_world(x, z)
            );
        }
    }

    let spawn_hint = map
        .spawn
        .map(|(x, z)| Vec3::new((x + 0.5) * 128.0, 0.0, (z + 0.5) * 128.0))
        .or_else(|| {
            map.lanes
                .first()
                .and_then(|lane| lane.waypoints.get(1))
                .copied()
        })
        .unwrap_or(center);
    let player_spawn = find_walkable_spawn(&height, &collision, spawn_hint, &terrains)?;
    let npc_spawns = build_npc_spawns(&height, &collision, center, player_spawn, &terrains);
    let ambient_spawns = build_ambient_spawns(
        cache,
        &height,
        &collision,
        &map,
        &colors,
        size,
        (origin_tile_x, origin_tile_y),
    );

    tracing::info!(
        "scene: {} regions, {} terrain verts, {} model verts, {} placements ({} ground decor), {} npcs, map {:?} lanes {:?}, center {:?}",
        terrains.len(),
        terrain_mesh.vertices.len(),
        model_mesh.vertices.len(),
        placements,
        decor_placements,
        npc_spawns.len(),
        map.name,
        map.lanes
            .iter()
            .map(|lane| lane.id.as_str())
            .collect::<Vec<_>>(),
        center
    );

    // A distant ground plane hides the loaded-block edge and gives the sky a
    // horizon to meet. It must sit below every terrain vertex: at a fixed
    // y = -24 it poked through the carved river banks and low ground as flat,
    // untextured dark-green slabs outlined by the -24 contour.
    let lowest_ground = height.heights.iter().copied().fold(f32::MAX, f32::min);
    let backdrop_y = lowest_ground.min(-24.0) - 60.0;
    let backdrop = 220_000.0;
    let bx0 = center.x - backdrop;
    let bx1 = center.x + backdrop;
    let bz0 = center.z - backdrop;
    let bz1 = center.z + backdrop;
    let bc = [44, 62, 40];
    model_mesh.push_triangle(
        SceneVertex::new(Vec3::new(bx0, backdrop_y, bz0), Vec3::Y, bc),
        SceneVertex::new(Vec3::new(bx1, backdrop_y, bz0), Vec3::Y, bc),
        SceneVertex::new(Vec3::new(bx0, backdrop_y, bz1), Vec3::Y, bc),
    );
    model_mesh.push_triangle(
        SceneVertex::new(Vec3::new(bx1, backdrop_y, bz0), Vec3::Y, bc),
        SceneVertex::new(Vec3::new(bx1, backdrop_y, bz1), Vec3::Y, bc),
        SceneVertex::new(Vec3::new(bx0, backdrop_y, bz1), Vec3::Y, bc),
    );

    Ok(SceneData {
        terrain: terrain_mesh,
        models: model_mesh,
        water: water_mesh,
        shadows: shadow_mesh,
        shadow_casters: shadow_caster_mesh,
        placements,
        height,
        collision,
        atmosphere: map.atmosphere.clone(),
        map,
        npc_spawns,
        ambient_spawns,
        player_spawn,
    })
}

fn find_walkable_spawn(
    height: &HeightGrid,
    collision: &WorldCollision,
    center: Vec3,
    terrains: &HashMap<(u32, u32), Terrain>,
) -> Result<Vec3> {
    for radius in 4..40 {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let world_x = center.x + dx as f32 * 128.0;
                let world_z = center.z + dy as f32 * 128.0;
                if !height.contains(world_x, world_z) {
                    continue;
                }
                let tile_x = (world_x / 128.0).floor() as i32;
                let tile_y = (world_z / 128.0).floor() as i32;
                let rx = (tile_x >> 6) as u32;
                let ry = (tile_y >> 6) as u32;
                let lx = (tile_x & 63) as usize;
                let ly = (tile_y & 63) as usize;
                if !collision.is_walkable_world(world_x, world_z) {
                    continue;
                }
                if let Some(terrain) = terrains.get(&(rx, ry)) {
                    let tile = terrain.tile(0, lx, ly);
                    if tile.underlay_id > 0
                        && tile.underlay_id != 55
                        && tile.overlay_id == 0
                        && tile.height > 4
                    {
                        return Ok(Vec3::new(
                            world_x,
                            height.height_at(world_x, world_z),
                            world_z,
                        ));
                    }
                }
            }
        }
    }
    Ok(Vec3::new(
        center.x,
        height.height_at(center.x, center.z),
        center.z,
    ))
}

fn build_npc_spawns(
    height: &HeightGrid,
    collision: &WorldCollision,
    center: Vec3,
    player_spawn: Vec3,
    terrains: &HashMap<(u32, u32), Terrain>,
) -> Vec<NpcSpawn> {
    let mut spots = Vec::new();
    for radius in 8..80 {
        for angle_step in 0..32 {
            let angle = angle_step as f32 * std::f32::consts::TAU / 32.0;
            let world_x = center.x + angle.cos() * radius as f32 * 128.0;
            let world_z = center.z + angle.sin() * radius as f32 * 128.0;
            if !height.contains(world_x, world_z) {
                continue;
            }
            if Vec3::new(world_x, 0.0, world_z).distance(Vec3::new(
                player_spawn.x,
                0.0,
                player_spawn.z,
            )) < 4.0 * 128.0
            {
                continue;
            }
            if spots.iter().any(|p: &Vec3| {
                Vec3::new(world_x, 0.0, world_z).distance(Vec3::new(p.x, 0.0, p.z)) < 4.0 * 128.0
            }) {
                continue;
            }
            if !collision.is_walkable_world(world_x, world_z) {
                continue;
            }
            let tile_x = (world_x / 128.0).floor() as i32;
            let tile_y = (world_z / 128.0).floor() as i32;
            let rx = (tile_x >> 6) as u32;
            let ry = (tile_y >> 6) as u32;
            let lx = (tile_x & 63) as usize;
            let ly = (tile_y & 63) as usize;
            let Some(terrain) = terrains.get(&(rx, ry)) else {
                continue;
            };
            let tile = terrain.tile(0, lx, ly);
            if tile.underlay_id > 0
                && tile.underlay_id != 55
                && tile.overlay_id == 0
                && tile.height > 4
            {
                spots.push(Vec3::new(
                    world_x,
                    height.height_at(world_x, world_z),
                    world_z,
                ));
                if spots.len() >= 8 {
                    break;
                }
            }
        }
        if spots.len() >= 8 {
            break;
        }
    }

    let mut spawns = Vec::new();
    for pos in &spots {
        spawns.push(NpcSpawn {
            model_id: 10413,
            idle_sequence: 1721,
            walk_sequence: Some(2405),
            pos: *pos,
            scale: 1.6,
        });
    }
    spawns
}

struct AmbientTemplate {
    models: Vec<u32>,
    idle_sequence: u32,
    walk_sequence: u32,
    scale: f32,
    recolors: Vec<(i16, i16)>,
}

fn ambient_template(cache: &Cache, npc_id: u32, scale: f32) -> Option<AmbientTemplate> {
    let data = cache.file(2, 9, npc_id).ok()?;
    let def = NpcDefinition::parse(npc_id, &data).ok()?;
    if def.models.is_empty() {
        return None;
    }
    let models = def.models.clone();
    let idle_sequence = u32::try_from(def.standing_animation).ok()?;
    let walk_sequence = u32::try_from(def.walking_animation).unwrap_or(idle_sequence);
    let recolors = def
        .recolor_to_find
        .iter()
        .copied()
        .zip(def.recolor_to_replace.iter().copied())
        .collect();
    Some(AmbientTemplate {
        models,
        idle_sequence,
        walk_sequence,
        scale,
        recolors,
    })
}

fn ground_color_at(
    colors: &[[u8; 3]],
    size: usize,
    origin: (i32, i32),
    x: f32,
    z: f32,
) -> Option<[u8; 3]> {
    let gx = (x / 128.0 - origin.0 as f32).floor() as isize;
    let gy = (z / 128.0 - origin.1 as f32).floor() as isize;
    if gx < 0 || gy < 0 || gx >= size as isize || gy >= size as isize {
        return None;
    }
    Some(colors[gy as usize * size + gx as usize])
}

fn ground_is_green(colors: &[[u8; 3]], size: usize, origin: (i32, i32), x: f32, z: f32) -> bool {
    ground_color_at(colors, size, origin, x, z).is_some_and(|color| {
        color[1] > color[0].saturating_add(6) && color[1] > color[2].saturating_add(6)
    })
}

fn ground_is_land(colors: &[[u8; 3]], size: usize, origin: (i32, i32), x: f32, z: f32) -> bool {
    ground_color_at(colors, size, origin, x, z)
        .is_some_and(|color| color[2] <= color[1].saturating_add(8))
}

/// Butterflies across the natural ground and a slow herd of monkey-riding
/// cows in the grass ring around the map edge.
fn build_ambient_spawns(
    cache: &Cache,
    height: &HeightGrid,
    collision: &WorldCollision,
    map: &MapLayout,
    colors: &[[u8; 3]],
    size: usize,
    origin: (i32, i32),
) -> Vec<AmbientSpawn> {
    const BUTTERFLY_NPCS: &[u32] = &[234, 235, 236, 237, 238, 1854, 1855, 3923];
    const COW_NPC: u32 = 10598;

    let butterflies = BUTTERFLY_NPCS
        .iter()
        .filter_map(|&npc_id| ambient_template(cache, npc_id, 5.0))
        .collect::<Vec<_>>();
    let cow = ambient_template(cache, COW_NPC, 1.7);

    let mut rng = Rng::new(0xA3B1_7F2D_9C4E_51B7);
    let mut spawns = Vec::new();
    let mut butterfly_count = 0usize;
    let butterfly_target = butterflies.len() * 5;
    let mut butterfly_attempts = 0usize;
    while butterfly_count < butterfly_target && butterfly_attempts < 12_000 {
        butterfly_attempts += 1;
        let tile_x = rng.range(5.0, size as f32 - 6.0);
        let tile_y = rng.range(5.0, size as f32 - 6.0);
        let x = (origin.0 as f32 + tile_x) * 128.0 + 64.0;
        let z = (origin.1 as f32 + tile_y) * 128.0 + 64.0;
        if !height.contains(x, z)
            || !ground_is_green(colors, size, origin, x, z)
            || !collision.is_walkable_world(x, z)
            || map.distance_to_lane(x, z) < 1.5 * 128.0
        {
            continue;
        }
        let pos = Vec3::new(x, height.height_at(x, z), z);
        if spawns
            .iter()
            .any(|spawn: &AmbientSpawn| spawn.pos.distance(pos) < 2.5 * 128.0)
        {
            continue;
        }
        let template = &butterflies[butterfly_count % butterflies.len()];
        spawns.push(AmbientSpawn {
            kind: AmbientKind::Butterfly,
            model_id: template.models[0],
            attachments: template.models[1..].to_vec(),
            idle_sequence: template.idle_sequence,
            walk_sequence: template.walk_sequence,
            pos,
            scale: template.scale,
            recolors: template.recolors.clone(),
        });
        butterfly_count += 1;
    }

    // A few butterflies near each nexus guarantee the coloured variants are
    // seen without hunting the whole map.
    if !butterflies.is_empty() {
        let mut bases = Vec::new();
        for lane in &map.lanes {
            for end in [lane.waypoints.first(), lane.waypoints.last()]
                .into_iter()
                .flatten()
            {
                if !bases
                    .iter()
                    .any(|base: &Vec3| base.distance(*end) < 8.0 * 128.0)
                {
                    bases.push(*end);
                }
            }
        }
        let mut placed = 0usize;
        for base in bases {
            let mut attempts = 0usize;
            let mut here = 0usize;
            while here < 4 && attempts < 120 {
                attempts += 1;
                let angle = (here as f32 / 4.0) * std::f32::consts::TAU + rng.range(-0.45, 0.45);
                let radius = rng.range(6.0, 15.0) * 128.0;
                let x = base.x + angle.cos() * radius;
                let z = base.z + angle.sin() * radius;
                if !height.contains(x, z)
                    || !ground_is_green(colors, size, origin, x, z)
                    || !collision.is_walkable_world(x, z)
                    || map.distance_to_lane(x, z) < 2.0 * 128.0
                {
                    continue;
                }
                let pos = Vec3::new(x, height.height_at(x, z), z);
                if spawns
                    .iter()
                    .any(|spawn: &AmbientSpawn| spawn.pos.distance(pos) < 4.0 * 128.0)
                {
                    continue;
                }
                let template = &butterflies[placed % butterflies.len()];
                spawns.push(AmbientSpawn {
                    kind: AmbientKind::Butterfly,
                    model_id: template.models[0],
                    attachments: template.models[1..].to_vec(),
                    idle_sequence: template.idle_sequence,
                    walk_sequence: template.walk_sequence,
                    pos,
                    scale: template.scale,
                    recolors: template.recolors.clone(),
                });
                butterfly_count += 1;
                placed += 1;
                here += 1;
            }
        }
    }

    let mut cow_count = 0usize;
    if let Some(template) = cow {
        let min_x = origin.0 as f32 + 5.0;
        let max_x = origin.0 as f32 + size as f32 - 6.0;
        let min_y = origin.1 as f32 + 5.0;
        let max_y = origin.1 as f32 + size as f32 - 6.0;
        for side in 0..4 {
            let mut placed = 0usize;
            let mut attempts = 0usize;
            while placed < 4 && attempts < 800 {
                attempts += 1;
                let depth = rng.range(5.0, 24.0);
                let (tile_x, tile_y) = match side {
                    0 => (rng.range(min_x + 3.0, max_x - 3.0), min_y + depth),
                    1 => (rng.range(min_x + 3.0, max_x - 3.0), max_y - depth),
                    2 => (min_x + depth, rng.range(min_y + 3.0, max_y - 3.0)),
                    _ => (max_x - depth, rng.range(min_y + 3.0, max_y - 3.0)),
                };
                let x = tile_x * 128.0 + 64.0;
                let z = tile_y * 128.0 + 64.0;
                if !height.contains(x, z)
                    || !ground_is_green(colors, size, origin, x, z)
                    || !collision.is_walkable_world(x, z)
                    || map.distance_to_lane(x, z) < 7.0 * 128.0
                    || map.in_biome(tile_x as i32, tile_y as i32)
                    || map
                        .structures
                        .iter()
                        .any(|structure| structure.pos.distance(Vec3::new(x, 0.0, z)) < 6.0 * 128.0)
                {
                    continue;
                }
                let pos = Vec3::new(x, height.height_at(x, z), z);
                if spawns.iter().any(|spawn| {
                    spawn.kind == AmbientKind::Cow && spawn.pos.distance(pos) < 10.0 * 128.0
                }) {
                    continue;
                }
                spawns.push(AmbientSpawn {
                    kind: AmbientKind::Cow,
                    model_id: template.models[0],
                    attachments: template.models[1..].to_vec(),
                    idle_sequence: template.idle_sequence,
                    walk_sequence: template.walk_sequence,
                    pos,
                    scale: template.scale,
                    recolors: template.recolors.clone(),
                });
                placed += 1;
                cow_count += 1;
            }
        }
    }

    let mut crab_count = 0usize;
    if let (Some(template), Some(river)) = (ambient_template(cache, 1553, 2.0), &map.river) {
        if river.points.len() >= 2 {
            for pair in river.points.windows(2) {
                let (ax, ay) = pair[0];
                let (bx, by) = pair[1];
                let dx = bx - ax;
                let dy = by - ay;
                let len = (dx * dx + dy * dy).sqrt().max(0.001);
                let nx = -dy / len;
                let ny = dx / len;
                let steps = ((len / 4.0).ceil() as usize).max(1);
                for step in 0..steps {
                    if crab_count >= 28 {
                        break;
                    }
                    let t = (step as f32 + 0.5) / steps as f32;
                    let cx = ax + dx * t;
                    let cy = ay + dy * t;
                    for side in [-1.0f32, 1.0] {
                        if rng.chance(0.45) {
                            continue;
                        }
                        let offset = river.width + 1.6 + (step % 3) as f32 * 0.55;
                        let tile_x = cx + nx * offset * side;
                        let tile_y = cy + ny * offset * side;
                        let x = tile_x * 128.0 + 64.0;
                        let z = tile_y * 128.0 + 64.0;
                        if !height.contains(x, z)
                            || !ground_is_land(colors, size, origin, x, z)
                            || !collision.is_walkable_world(x, z)
                            || map.distance_to_lane(x, z) < 2.5 * 128.0
                            || spawns.iter().any(|spawn: &AmbientSpawn| {
                                spawn.kind == AmbientKind::Crab
                                    && spawn.pos.distance(Vec3::new(x, 0.0, z)) < 2.5 * 128.0
                            })
                        {
                            continue;
                        }
                        let pos = Vec3::new(x, height.height_at(x, z), z);
                        spawns.push(AmbientSpawn {
                            kind: AmbientKind::Crab,
                            model_id: template.models[0],
                            attachments: template.models[1..].to_vec(),
                            idle_sequence: template.idle_sequence,
                            walk_sequence: template.walk_sequence,
                            pos,
                            scale: template.scale,
                            recolors: template.recolors.clone(),
                        });
                        crab_count += 1;
                    }
                }
            }
        }
    }

    tracing::info!(
        "ambient creatures: {} butterflies ({} variants, {} attempts), {} pasture cows, {} shore crabs",
        butterfly_count,
        butterflies.len(),
        butterfly_attempts,
        cow_count,
        crab_count
    );
    spawns
}

fn is_scenery(def: &ObjectDefinition) -> bool {
    let name = def.name.to_lowercase();
    // Palms and jungle trees use tall bare trunks that read as green pillars
    // at MOBA scale. The lane bands use broadleaf trees, bushes, boulders and
    // ruins instead.
    if name.contains("palm") || name.contains("jungle") || name.contains("tropical tree") {
        return false;
    }
    // Several cache revisions reuse bare stems, dead branches and palm
    // models under generic "Tree" names. With a leaf hint those become solid
    // green columns. Keep them only when the object is explicitly a dead
    // tree (which is rendered with bark on every face).
    // Stumps, logs and young cuttings are trunk-only meshes; tinting them
    // with a leaf texture produces the tall green columns this pass removed.
    if ["stump", "fallen", "hollow", "sapling", "log"]
        .iter()
        .any(|needle| name.contains(needle))
    {
        return false;
    }
    const BARE_TREE_MODELS: &[u32] = &[1605, 1621, 1681, 1682, 1683, 1687, 1701, 1702, 1715, 1716];
    if !name.contains("dead")
        && def
            .models
            .iter()
            .any(|model| BARE_TREE_MODELS.contains(model))
    {
        return false;
    }
    name.contains("tree")
        || name.contains("plant")
        || name.contains("bush")
        || name.contains("fern")
        || name.contains("cactus")
}

fn cached_model<'a>(
    cache: &Cache,
    model_cache: &'a mut HashMap<u32, Model>,
    model_id: u32,
) -> Result<&'a Model> {
    if let std::collections::hash_map::Entry::Vacant(entry) = model_cache.entry(model_id) {
        let group = cache.group(7, model_id)?;
        let model = Model::load(model_id, &group.data)?;
        entry.insert(model);
    }
    Ok(model_cache.get(&model_id).expect("model inserted"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroundDecor {
    Flower,
    Grass,
    Fern,
    Mushroom,
    Rock,
    Twig,
    Plant,
}

impl GroundDecor {
    /// Ground clutter is deliberately non-blocking. Only objects that look
    /// like a low plant, loose stone or broken wood qualify.
    fn from_definition(def: &ObjectDefinition) -> Option<Self> {
        let name = def.name.to_lowercase();
        if name.is_empty()
            || name.contains("potted")
            || [
                "chair", "table", "bench", "crate", "chest", "bank", "counter", "shelf", "bed",
                "fire", "portal", "fountain", "well", "altar", "statue", "wall", "door", "gate",
                "ladder", "stair", "furnace", "anvil", "range", "spinning", "loom", "barrel",
                "sack", "coffin", "grave", "sign", "flag", "banner", "lantern", "candle", "torch",
                "cage", "rack", "display", "machine", "pump", "pipe", "wheel", "cart", "boat",
                "canoe", "target", "dummy",
            ]
            .iter()
            .any(|needle| name.contains(needle))
        {
            return None;
        }
        if name.contains("flower")
            || name.contains("rose")
            || name.contains("tulip")
            || name.contains("thistle")
            || name.contains("nettle")
            || name.contains("herb")
        {
            return Some(Self::Flower);
        }
        if name.contains("mushroom") || name.contains("toadstool") {
            return Some(Self::Mushroom);
        }
        if name.contains("fern") {
            return Some(Self::Fern);
        }
        if name.contains("grass") || name.contains("reed") || name.contains("weed") {
            return Some(Self::Grass);
        }
        if name.contains("twig") || name.contains("branch") {
            return Some(Self::Twig);
        }
        if name.contains("rock") || name.contains("boulder") || name.contains("stone") {
            return Some(Self::Rock);
        }
        if name.contains("plant")
            || name.contains("vine")
            || name.contains("leaf")
            || name.contains("leaves")
            || name.contains("bush")
            || name.contains("shrub")
            || name.contains("coral")
            || name.contains("shell")
        {
            return Some(Self::Plant);
        }
        // Roots and stumps are left to the full scenery pass; they are too
        // large and structural for the low ground-cover layer.
        None
    }

    fn shadow_size(self) -> f32 {
        match self {
            Self::Rock => 62.0,
            Self::Mushroom => 26.0,
            _ => 0.0,
        }
    }
}

fn load_underlays(cache: &Cache) -> Result<HashMap<u32, UnderlayDefinition>> {
    let files = cache.archive_files_with_ids(2, 1)?;
    let mut out = HashMap::new();
    for (id, data) in files {
        if let Ok(def) = UnderlayDefinition::parse(id, &data) {
            out.insert(id, def);
        }
    }
    Ok(out)
}

fn load_overlays(cache: &Cache) -> Result<HashMap<u32, OverlayDefinition>> {
    let files = cache.archive_files_with_ids(2, 4)?;
    let mut out = HashMap::new();
    for (id, data) in files {
        if let Ok(def) = OverlayDefinition::parse(id, &data) {
            out.insert(id, def);
        }
    }
    Ok(out)
}

/// Turn cache sea inside the playable castle bounds into land. The rift is
/// laid over a 5x5 region block whose corner is open sea; left alone, those
/// tiles kept the water overlay's colour and the flat sea floor, and read as
/// a murky dark-green basin next to the blue base. The colour becomes the
/// map's mean land colour, the overlay is dropped, and the height is filled
/// by relaxing only the sea tiles against the fixed land around them. The
/// authored river is carved later and is unaffected.
#[allow(clippy::too_many_arguments)]
fn reclaim_sea(
    heights: &mut [f32],
    colors: &mut [[u8; 3]],
    tile_overlay: &mut [u32],
    sea: &[bool],
    size: usize,
    origin_tile_x: i32,
    origin_tile_y: i32,
    castle: Option<&CastleSpec>,
) -> usize {
    let Some(castle) = castle else { return 0 };
    let (x0, z0, x1, z1) = castle.bounds;
    let margin = 6.0;
    let inside = |gx: usize, gy: usize| {
        let tx = (origin_tile_x + gx as i32) as f32;
        let tz = (origin_tile_y + gy as i32) as f32;
        tx >= x0 - margin && tx <= x1 + margin && tz >= z0 - margin && tz <= z1 + margin
    };
    let mut targets = vec![false; size * size];
    let mut land_sum = [0.0f64; 3];
    let mut land_count = 0.0f64;
    let mut land_height = 0.0f64;
    let mut count = 0usize;
    for gy in 0..size {
        for gx in 0..size {
            let i = gy * size + gx;
            if !inside(gx, gy) {
                continue;
            }
            if sea[i] {
                targets[i] = true;
                count += 1;
            } else {
                for c in 0..3 {
                    land_sum[c] += colors[i][c] as f64;
                }
                land_height += heights[i] as f64;
                land_count += 1.0;
            }
        }
    }
    if count == 0 || land_count == 0.0 {
        return count;
    }
    let land = [
        (land_sum[0] / land_count) as u8,
        (land_sum[1] / land_count) as u8,
        (land_sum[2] / land_count) as u8,
    ];
    let start = (land_height / land_count) as f32;
    let mut textures: Vec<u32> = targets
        .iter()
        .zip(tile_overlay.iter())
        .filter(|(t, _)| **t)
        .map(|(_, o)| (o >> 8) & 0xFF)
        .collect();
    textures.sort_unstable();
    textures.dedup();
    tracing::info!("sea overlay textures replaced: {textures:?}");
    for (i, target) in targets.iter().enumerate() {
        if *target {
            colors[i] = land;
            tile_overlay[i] = 0;
            heights[i] = start;
        }
    }
    // Harmonic fill: start the sea at the mean neighbouring land height and
    // relax only the sea tiles, so the reclaimed ground meets every shore.
    for _ in 0..160 {
        let src = heights.to_vec();
        for gy in 1..size - 1 {
            for gx in 1..size - 1 {
                let i = gy * size + gx;
                if targets[i] {
                    heights[i] = (src[i - 1] + src[i + 1] + src[i - size] + src[i + size]) * 0.25;
                }
            }
        }
    }
    tracing::info!("reclaimed {count} cache sea tiles inside the castle as land ({land:?})");
    count
}

/// Gaussian-relax the heightfield to remove single-tile rock spikes while
/// keeping broad hills. River cells are left untouched so the carved bed and
/// banks stay exactly where the water mesh expects them.
fn relax_heights(heights: &mut [f32], size: usize, water: &[f32], passes: usize) {
    const NEIGHBOURS: [(i32, i32, f32); 8] = [
        (-1, 0, 2.0),
        (1, 0, 2.0),
        (0, -1, 2.0),
        (0, 1, 2.0),
        (-1, -1, 1.0),
        (1, -1, 1.0),
        (-1, 1, 1.0),
        (1, 1, 1.0),
    ];
    for _ in 0..passes {
        let src = heights.to_vec();
        for gy in 1..size - 1 {
            for gx in 1..size - 1 {
                let i = gy * size + gx;
                if water[i] > 0.05 {
                    continue;
                }
                let mut sum = src[i] * 4.0;
                let mut weight = 4.0;
                for (dx, dy, w) in NEIGHBOURS {
                    let j = (gy as i32 + dy) as usize * size + (gx as i32 + dx) as usize;
                    if water[j] > 0.05 {
                        continue;
                    }
                    sum += src[j] * w;
                    weight += w;
                }
                heights[i] = sum / weight;
            }
        }
    }
}

/// Sculpt organic terrain over the cache heightfield. The cache base is often
/// either dead flat or noisy at one-tile scale; broad valley/hill shapes,
/// a ridged component and a little fine undulation give the map readable
/// elevation without turning every field into a rock garden.
///
/// Castle wall bands and base courtyards are held nearly level so wall
/// pieces do not step up and down between tiles. Lanes are graded after this
/// pass, and the river/water mask is left alone.
fn sculpt_terrain(
    heights: &mut [f32],
    size: usize,
    origin_tile_x: i32,
    origin_tile_y: i32,
    water: &[f32],
    castle: Option<&CastleSpec>,
) {
    // The playable floor should be calm. The reference map reads as flat,
    // readable fields with elevation concentrated in terraces and cliffs
    // (added at display resolution by `plateau_height`), not as a blanket
    // of noise hills. Compress the cache's tile-scale bumps towards a
    // heavily smoothed copy, then add one very broad, low swell so long
    // views still roll gently.
    let mut smooth = heights.to_vec();
    relax_heights(&mut smooth, size, water, 10);
    for gy in 0..size {
        for gx in 0..size {
            let i = gy * size + gx;
            if water[i] > 0.05 {
                continue;
            }
            let wx = (origin_tile_x + gx as i32) as f32 * 128.0;
            let wz = (origin_tile_y + gy as i32) as f32 * 128.0;
            let settle = castle_settlement_mask(
                (origin_tile_x + gx as i32) as f32,
                (origin_tile_y + gy as i32) as f32,
                castle,
            );
            let floor = smooth[i] + (heights[i] - smooth[i]) * 0.3;
            let swell = (value_noise(wx * 0.00011 + 13.1, wz * 0.00011 + 7.7) - 0.5) * 150.0
                + (value_noise(wx * 0.00042 + 31.0, wz * 0.00042 + 7.0) - 0.5) * 34.0;
            heights[i] = heights[i] * (1.0 - settle) + (floor + swell) * settle;
        }
    }
    relax_heights(heights, size, water, 1);
}

/// World units of one terrace step: a little under two tiles, so a cliff
/// reads as a wall a hero cannot see over, but never towers over trees.
const TERRACE_STEP: f32 = 210.0;

/// Where the jungle may rise into plateaus, 0..1. Lanes, the river, camps,
/// base courtyards and the castle wall bands stay on the floor, with a few
/// tiles of grass margin so cliffs never crowd a lane or a fight.
fn plateau_clearance(map: &MapLayout, wx: f32, wz: f32) -> f32 {
    let tx = wx / 128.0;
    let tz = wz / 128.0;
    let (_, lane_distance) = map.nearest_lane_point(wx, wz);
    let mut clear = smoothstep(4.5 * 128.0, 9.0 * 128.0, lane_distance);
    if let Some(river) = &map.river
        && river.points.len() >= 2
    {
        let (d, _, _) = nearest_on_polyline(&river.points, tx, tz);
        clear *= smoothstep(river.width * 2.2 + 2.0, river.width * 2.2 + 7.0, d);
    }
    for camp in &map.camps {
        let d = ((camp.pos.x - wx).powi(2) + (camp.pos.z - wz).powi(2)).sqrt() / 128.0;
        clear *= smoothstep(5.0, 10.0, d);
    }
    if let Some(castle) = &map.castle {
        let (x0, z0, x1, z1) = castle.bounds;
        let mut distance = distance_to_rect_border(tx, tz, x0, z0, x1, z1);
        for base in &castle.bases {
            let (bx0, bz0, bx1, bz1) = castle.base_rect(base);
            distance = distance.min(distance_to_rect_edge(tx, tz, bx0, bz0, bx1, bz1));
        }
        clear *= smoothstep(6.0, 14.0, distance);
    }
    clear
}

/// Terraced plateau height at a world position, before clearance.
/// A warped low-frequency field is cut at two contour levels; each cut is
/// a narrow smoothstep, so the ground is flat on top and at the foot with a
/// steep riser between. Rock noise roughens only the riser band so cliff
/// faces read as stone while the plateau tops stay walkable-flat.
fn plateau_height(wx: f32, wz: f32, clearance: f32) -> f32 {
    if clearance <= 0.0 {
        return 0.0;
    }
    let warp_x = (value_noise(wx * 0.00021 + 3.7, wz * 0.00021 - 8.1) - 0.5) * 1.6;
    let warp_z = (value_noise(wx * 0.00021 - 11.3, wz * 0.00021 + 2.9) - 0.5) * 1.6;
    let field = value_noise(wx * 0.00048 + warp_x + 41.0, wz * 0.00048 + warp_z - 17.0) * 0.72
        + value_noise(wx * 0.0013 + 5.0, wz * 0.0013 + 9.0) * 0.20
        + value_noise(wx * 0.0042 - 2.0, wz * 0.0042 + 7.0) * 0.08;
    // Pull the field down outside the clearance so contours bend away from
    // lanes and the river instead of ramping into them.
    let field = field - (1.0 - clearance) * 0.6;
    let lower = smoothstep(0.585, 0.625, field);
    let upper = smoothstep(0.745, 0.785, field);
    let riser = 4.0 * (lower * (1.0 - lower) + upper * (1.0 - upper));
    let rock = (value_noise(wx * 0.006 + 1.3, wz * 0.006 - 4.2) - 0.5) * 70.0
        + (value_noise(wx * 0.017 - 7.0, wz * 0.017 + 3.0) - 0.5) * 26.0;
    (lower + upper) * TERRACE_STEP + riser.min(1.0) * rock
}

/// Catmull-Rom sample of a square grid at fractional grid coordinates.
fn sample_bicubic(values: &[f32], size: usize, gx: f32, gy: f32) -> f32 {
    let x1 = gx.floor() as i32;
    let y1 = gy.floor() as i32;
    let fx = gx - x1 as f32;
    let fy = gy - y1 as f32;
    let at = |x: i32, y: i32| {
        let x = x.clamp(0, size as i32 - 1) as usize;
        let y = y.clamp(0, size as i32 - 1) as usize;
        values[y * size + x]
    };
    let cubic = |p0: f32, p1: f32, p2: f32, p3: f32, t: f32| {
        p1 + 0.5
            * t
            * (p2 - p0
                + t * (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3 + t * (3.0 * (p1 - p2) + p3 - p0)))
    };
    let mut rows = [0.0f32; 4];
    for (k, row) in rows.iter_mut().enumerate() {
        let y = y1 - 1 + k as i32;
        *row = cubic(at(x1 - 1, y), at(x1, y), at(x1 + 1, y), at(x1 + 2, y), fx);
    }
    cubic(rows[0], rows[1], rows[2], rows[3], fy)
}

/// Bilinear sample of a square grid at fractional grid coordinates.
fn sample_bilinear(values: &[f32], size: usize, gx: f32, gy: f32) -> f32 {
    let gx = gx.clamp(0.0, (size - 1) as f32);
    let gy = gy.clamp(0.0, (size - 1) as f32);
    let x0 = (gx.floor() as usize).min(size - 2);
    let y0 = (gy.floor() as usize).min(size - 2);
    let fx = gx - x0 as f32;
    let fy = gy - y0 as f32;
    let a = values[y0 * size + x0] * (1.0 - fx) + values[y0 * size + x0 + 1] * fx;
    let b = values[(y0 + 1) * size + x0] * (1.0 - fx) + values[(y0 + 1) * size + x0 + 1] * fx;
    a * (1.0 - fy) + b * fy
}

/// 0 on castle wall bands and base courtyards, 1 in open country. The
/// transition is wide enough that the plateau blends into the hills.
fn castle_settlement_mask(tx: f32, tz: f32, castle: Option<&CastleSpec>) -> f32 {
    let Some(castle) = castle else {
        return 1.0;
    };
    let (x0, z0, x1, z1) = castle.bounds;
    let mut distance = distance_to_rect_border(tx, tz, x0, z0, x1, z1);
    for base in &castle.bases {
        let (bx0, bz0, bx1, bz1) = castle.base_rect(base);
        distance = distance.min(distance_to_rect_edge(tx, tz, bx0, bz0, bx1, bz1));
    }
    smoothstep(2.0, 11.0, distance)
}

/// Distance to a rectangle's border, including from inside it. Used for the
/// perimeter wall, which encloses nearly the whole map.
fn distance_to_rect_border(x: f32, z: f32, x0: f32, z0: f32, x1: f32, z1: f32) -> f32 {
    if x >= x0 && x <= x1 && z >= z0 && z <= z1 {
        return (x - x0).min(x1 - x).min(z - z0).min(z1 - z).max(0.0);
    }
    outside_rect_distance(x, z, x0, z0, x1, z1)
}

/// Distance to a rectangle, or 0 when the point is inside it. Used for the
/// base courtyards, which should be level throughout.
fn distance_to_rect_edge(x: f32, z: f32, x0: f32, z0: f32, x1: f32, z1: f32) -> f32 {
    if x >= x0 && x <= x1 && z >= z0 && z <= z1 {
        return 0.0;
    }
    outside_rect_distance(x, z, x0, z0, x1, z1)
}

fn outside_rect_distance(x: f32, z: f32, x0: f32, z0: f32, x1: f32, z1: f32) -> f32 {
    let dx = (x0 - x).max(x - x1).max(0.0);
    let dz = (z0 - z).max(z - z1).max(0.0);
    (dx * dx + dz * dz).sqrt()
}

/// Level each team courtyard to its centre height with a soft skirt at the
/// walls. The cache ground under a base is uneven; a built plaza should be
/// flat so structures, the fountain and future paving sit flush.
fn flatten_bases(
    heights: &mut [f32],
    size: usize,
    origin_tile_x: i32,
    origin_tile_y: i32,
    castle: Option<&CastleSpec>,
) {
    let Some(castle) = castle else { return };
    for base in &castle.bases {
        let half = base.size * 0.5;
        let cx = base.center.0;
        let cz = base.center.1;
        let ci = ((cz - origin_tile_y as f32).clamp(0.0, size as f32 - 1.0) as usize) * size
            + (cx - origin_tile_x as f32).clamp(0.0, size as f32 - 1.0) as usize;
        let target = heights[ci];
        let x0 = cx - half;
        let x1 = cx + half;
        let z0 = cz - half;
        let z1 = cz + half;
        let gx0 = (x0 - origin_tile_x as f32 - 3.0).floor().max(0.0) as usize;
        let gx1 = (x1 - origin_tile_x as f32 + 3.0)
            .ceil()
            .min(size as f32 - 1.0) as usize;
        let gy0 = (z0 - origin_tile_y as f32 - 3.0).floor().max(0.0) as usize;
        let gy1 = (z1 - origin_tile_y as f32 + 3.0)
            .ceil()
            .min(size as f32 - 1.0) as usize;
        for gy in gy0..=gy1 {
            for gx in gx0..=gx1 {
                let tx = (origin_tile_x + gx as i32) as f32;
                let tz = (origin_tile_y + gy as i32) as f32;
                // Depth inside the courtyard rectangle, in tiles. Negative
                // outside; the skirt blends over three tiles at the wall.
                let depth = (tx - x0).min(x1 - tx).min(tz - z0).min(z1 - tz);
                let t = smoothstep(-1.0, 2.0, depth);
                if t <= 0.0 {
                    continue;
                }
                let i = gy * size + gx;
                heights[i] = heights[i] * (1.0 - t) + target * t;
            }
        }
    }
}

fn box_blur_colors(colors: &[[u8; 3]], size: usize, radius: i32) -> Vec<[u8; 3]> {
    let mut horizontal = vec![[0u8; 3]; colors.len()];
    for y in 0..size {
        for x in 0..size {
            let mut sum = [0u32; 3];
            let mut count = 0u32;
            for dx in -radius..=radius {
                let sx = x as i32 + dx;
                if sx < 0 || sx >= size as i32 {
                    continue;
                }
                let c = colors[y * size + sx as usize];
                sum[0] += c[0] as u32;
                sum[1] += c[1] as u32;
                sum[2] += c[2] as u32;
                count += 1;
            }
            horizontal[y * size + x] = [
                (sum[0] / count) as u8,
                (sum[1] / count) as u8,
                (sum[2] / count) as u8,
            ];
        }
    }
    let mut out = vec![[0u8; 3]; colors.len()];
    for y in 0..size {
        for x in 0..size {
            let mut sum = [0u32; 3];
            let mut count = 0u32;
            for dy in -radius..=radius {
                let sy = y as i32 + dy;
                if sy < 0 || sy >= size as i32 {
                    continue;
                }
                let c = horizontal[sy as usize * size + x];
                sum[0] += c[0] as u32;
                sum[1] += c[1] as u32;
                sum[2] += c[2] as u32;
                count += 1;
            }
            out[y * size + x] = [
                (sum[0] / count) as u8,
                (sum[1] / count) as u8,
                (sum[2] / count) as u8,
            ];
        }
    }
    out
}

fn tile_color(
    terrain: &Terrain,
    underlays: &HashMap<u32, UnderlayDefinition>,
    overlays: &HashMap<u32, OverlayDefinition>,
    x: usize,
    y: usize,
) -> [u8; 3] {
    let tile = terrain.tile(0, x, y);

    if tile.overlay_id > 0 {
        if let Some(overlay) = overlays.get(&(tile.overlay_id as u32)) {
            let color = overlay.color();
            // 0xff00ff is a sentinel for "texture-only" ground overlays. Until
            // textures are sampled, fall through to the underlay colour.
            if color != [255, 0, 255] {
                return color;
            }
        }
    }
    if tile.underlay_id > 0 {
        if let Some(underlay) = underlays.get(&(tile.underlay_id as u32)) {
            return underlay.color();
        }
    }

    if tile.height == 0 {
        [31, 70, 105]
    } else if tile.height < 8 {
        [72, 112, 55]
    } else {
        [90, 105, 62]
    }
}

/// Small deterministic xorshift RNG. The map is generated from a fixed seed
/// so every match starts from the same world.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }

    fn f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }

    fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }
}

fn append_bridge_decks(mesh: &mut MeshData, map: &MapLayout, height: &HeightGrid) -> usize {
    let Some(river) = map.river.as_ref() else {
        return 0;
    };
    let mut count = 0;
    for bridge in &river.bridges {
        let bx = bridge.0 * 128.0;
        let bz = bridge.1 * 128.0;
        let Some((tx, tz)) = nearest_lane_tangent(&map.lanes, bx, bz) else {
            continue;
        };
        let (nx, nz) = (-tz, tx);
        let mut corners = [Vec3::ZERO; 4];
        for (index, (along, across)) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)]
            .into_iter()
            .enumerate()
        {
            let x = bx + tx * BRIDGE_HALF_LENGTH * along + nx * BRIDGE_HALF_WIDTH * across;
            let z = bz + tz * BRIDGE_HALF_LENGTH * along + nz * BRIDGE_HALF_WIDTH * across;
            corners[index] = Vec3::new(x, 0.0, z);
        }
        // The deck is a boardwalk laid over the ford, not a raised platform;
        // keep it flush with the lane so heroes stand on it rather than under
        // it. The rails still give the crossing a bridge silhouette.
        let deck_y = height.height_at(bx, bz) + BRIDGE_DECK_LIFT;
        tracing::info!(
            "bridge deck {:?}: deck_y={:.1}, terrain_center={:.1}",
            bridge,
            deck_y,
            height.height_at(bx, bz)
        );
        let mut vertices = Vec::with_capacity(4);
        for corner in corners {
            let mut vertex = SceneVertex::new(
                Vec3::new(corner.x, deck_y, corner.z),
                Vec3::Y,
                [196, 172, 132],
            );
            vertex.uv = [corner.x / 128.0, corner.z / 128.0];
            vertex.material = 7;
            vertex.texture_id = 22;
            vertices.push(vertex);
        }
        mesh.push_triangle(vertices[0], vertices[2], vertices[1]);
        mesh.push_triangle(vertices[1], vertices[2], vertices[3]);

        // Low rails make the deck read as a bridge rather than a floating
        // stone slab. They are visual only; the lane remains walkable.
        for across in [-1.0f32, 1.0] {
            let a = Vec3::new(
                bx + tx * BRIDGE_HALF_LENGTH + nx * BRIDGE_HALF_WIDTH * across,
                deck_y,
                bz + tz * BRIDGE_HALF_LENGTH + nz * BRIDGE_HALF_WIDTH * across,
            );
            let b = Vec3::new(
                bx - tx * BRIDGE_HALF_LENGTH + nx * BRIDGE_HALF_WIDTH * across,
                deck_y,
                bz - tz * BRIDGE_HALF_LENGTH + nz * BRIDGE_HALF_WIDTH * across,
            );
            let mut rail = Vec::with_capacity(4);
            for (point, top) in [(a, false), (b, false), (b, true), (a, true)] {
                let mut vertex = SceneVertex::new(
                    Vec3::new(point.x, deck_y + if top { 18.0 } else { 0.0 }, point.z),
                    Vec3::Y,
                    [128, 102, 72],
                );
                vertex.uv = [point.x / 128.0, point.z / 128.0];
                vertex.material = 7;
                vertex.texture_id = 5;
                rail.push(vertex);
            }
            mesh.push_triangle(rail[0], rail[1], rail[2]);
            mesh.push_triangle(rail[0], rail[2], rail[3]);
        }
        count += 1;
    }
    count
}

fn nearest_lane_tangent(lanes: &[LaneSpawn], x: f32, z: f32) -> Option<(f32, f32)> {
    let mut best = (f32::MAX, 0.0, 0.0);
    for lane in lanes {
        for pair in lane.waypoints.windows(2) {
            let (ax, az) = (pair[0].x, pair[0].z);
            let (dx, dz) = (pair[1].x - ax, pair[1].z - az);
            let length2 = dx * dx + dz * dz;
            if length2 < 1e-6 {
                continue;
            }
            let t = (((x - ax) * dx + (z - az) * dz) / length2).clamp(0.0, 1.0);
            let px = ax + dx * t;
            let pz = az + dz * t;
            let distance2 = (x - px) * (x - px) + (z - pz) * (z - pz);
            if distance2 < best.0 {
                let length = length2.sqrt();
                best = (distance2, dx / length, dz / length);
            }
        }
    }
    (best.0 < f32::MAX).then_some((best.1, best.2))
}

/// Pack a cache ground-overlay texture and its blend weight into the high
/// bytes of a terrain vertex's `texture_id`. The low byte is reserved for the
/// grass variant.
fn pack_ground_overlay(texture: u8, weight: u8) -> u32 {
    ((texture as u32) << 8) | ((weight as u32) << 16)
}

fn value_noise(x: f32, z: f32) -> f32 {
    let ix = x.floor();
    let iz = z.floor();
    let fx = x - ix;
    let fz = z - iz;
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uz = fz * fz * (3.0 - 2.0 * fz);
    // Hash the neighbouring lattice point itself. The old version hashed
    // the cell plus a corner flag, so adjacent cells disagreed on their
    // shared corners and the field jumped at every integer line.
    let h = |dx: i32, dz: i32| {
        let mut seed = ((ix as i32).wrapping_add(dx) as u32).wrapping_mul(73856093);
        seed ^= ((iz as i32).wrapping_add(dz) as u32).wrapping_mul(19349663);
        hash01(seed)
    };
    let a = h(0, 0);
    let b = h(1, 0);
    let c = h(0, 1);
    let d = h(1, 1);
    let top = a + (b - a) * ux;
    let bottom = c + (d - c) * ux;
    top + (bottom - top) * uz
}

/// Deterministic 0..1 hash used for scenery jitter.
fn hash01(seed: u32) -> f32 {
    let mut h = seed ^ 0x9E37_79B9;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    (h & 0x00FF_FFFF) as f32 / 0x0100_0000 as f32
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Build the river surface from the water mask at half-tile resolution.
/// The old 128-unit quads produced stair-stepped banks; 64-unit cells with a
/// mask-driven alpha fade give a softer shoreline without the bridge gaps a
/// centreline ribbon can leave on bends.
fn build_water_surface(
    mesh: &mut MeshData,
    size: usize,
    origin_tile_x: i32,
    origin_tile_y: i32,
    water: &[f32],
    levels: &[f32],
) {
    const SUB: usize = 2;
    let span = (size - 1) * SUB;
    let step = 128.0 / SUB as f32;
    for gy in 0..span {
        for gx in 0..span {
            let tx = gx as f32 / SUB as f32;
            let ty = gy as f32 / SUB as f32;
            let mut masks = [0.0f32; 4];
            let mut level_sum = 0.0;
            let mut level_weight = 0.0;
            for (index, (dx, dy)) in [(0, 0), (1, 0), (0, 1), (1, 1)].iter().enumerate() {
                let px = tx + *dx as f32 / SUB as f32;
                let py = ty + *dy as f32 / SUB as f32;
                let mask = bilinear(water, size, px, py);
                masks[index] = mask;
                if mask > 0.001 {
                    level_sum += bilinear(levels, size, px, py) * mask;
                    level_weight += mask;
                }
            }
            if masks.iter().copied().fold(0.0, f32::max) < 0.02 || level_weight <= 0.0 {
                continue;
            }
            let level = level_sum / level_weight + 0.75;
            let x0 = (origin_tile_x as f32 + tx) * 128.0;
            let z0 = (origin_tile_y as f32 + ty) * 128.0;
            let x1 = x0 + step;
            let z1 = z0 + step;
            let opacity = |mask: f32| (smoothstep(0.0, 0.35, mask) * 0.88).clamp(0.0, 0.88);
            let mut v00 = SceneVertex::new(Vec3::new(x0, level, z0), Vec3::Y, [255, 255, 255]);
            let mut v10 = SceneVertex::new(Vec3::new(x1, level, z0), Vec3::Y, [255, 255, 255]);
            let mut v01 = SceneVertex::new(Vec3::new(x0, level, z1), Vec3::Y, [255, 255, 255]);
            let mut v11 = SceneVertex::new(Vec3::new(x1, level, z1), Vec3::Y, [255, 255, 255]);
            v00.uv[0] = masks[0];
            v10.uv[0] = masks[1];
            v01.uv[0] = masks[2];
            v11.uv[0] = masks[3];
            v00.color[3] = opacity(masks[0]);
            v10.color[3] = opacity(masks[1]);
            v01.color[3] = opacity(masks[2]);
            v11.color[3] = opacity(masks[3]);
            v00.material = 6;
            v10.material = 6;
            v01.material = 6;
            v11.material = 6;
            mesh.push_triangle(v00, v01, v10);
            mesh.push_triangle(v10, v01, v11);
        }
    }
}

fn bilinear(values: &[f32], size: usize, x: f32, y: f32) -> f32 {
    let max = size as f32 - 1.0;
    let x = x.clamp(0.0, max);
    let y = y.clamp(0.0, max);
    let x0 = (x.floor() as usize).min(size - 2);
    let y0 = (y.floor() as usize).min(size - 2);
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let i = y0 * size + x0;
    let top = values[i] * (1.0 - fx) + values[i + 1] * fx;
    let bottom = values[i + size] * (1.0 - fx) + values[i + size + 1] * fx;
    top * (1.0 - fy) + bottom * fy
}

/// Distance to a polyline plus the index of the closest segment and how far
/// along that segment the closest point sits.
fn nearest_on_polyline(points: &[(f32, f32)], x: f32, y: f32) -> (f32, usize, f32) {
    let mut best = (f32::MAX, 0usize, 0.0f32);
    for (index, pair) in points.windows(2).enumerate() {
        let (ax, ay) = pair[0];
        let (bx, by) = pair[1];
        let abx = bx - ax;
        let aby = by - ay;
        let len2 = abx * abx + aby * aby;
        if len2 <= 1e-6 {
            continue;
        }
        let t = (((x - ax) * abx + (y - ay) * aby) / len2).clamp(0.0, 1.0);
        let px = ax + abx * t;
        let py = ay + aby * t;
        let d = ((x - px) * (x - px) + (y - py) * (y - py)).sqrt();
        if d < best.0 {
            best = (d, index, t);
        }
    }
    best
}

/// True for the OSRS objects that are plants, so their flat coloured faces
/// can be replaced with foliage green.
fn is_plant_object(def: &ObjectDefinition) -> bool {
    let name = def.name.to_lowercase();
    [
        "tree", "palm", "jungle", "bush", "plant", "leaves", "sapling",
    ]
    .iter()
    .any(|needle| name.contains(needle))
}

/// Scenery that is worth the draw call inside a biome pocket.
fn is_pocket_scenery(def: &ObjectDefinition) -> bool {
    let name = def.name.to_lowercase();
    if name.is_empty() {
        return false;
    }
    ![
        "flower", "daisies", "grass", "fern", "plant", "weed", "herb", "sapling", "mushroom",
        "palm", "jungle",
    ]
    .iter()
    .any(|needle| name.contains(needle))
}

/// A cache model that is a large, nearly flat ground plane: marsh and floor
/// decorations, rugs, pond beds. Transplanted into a pocket they render as
/// untextured dark slabs over the terrain (and used to block movement and
/// sight), so pockets skip them; the terrain already carries the ground.
fn is_flat_ground_plane(model: &Model) -> bool {
    let (min, max) = model.bounds();
    let height = (max[1] - min[1]) as f32;
    let footprint = ((max[0] - min[0]) as f32).max((max[2] - min[2]) as f32);
    height < 24.0 && footprint > 150.0
}

/// Order pocket scenery so landmark objects are placed before clutter.
fn scenery_priority(def: &ObjectDefinition) -> u8 {
    let name = def.name.to_lowercase();
    if name.contains("tree") || name.contains("palm") || name.contains("jungle") {
        return 0;
    }
    if [
        "pillar", "statue", "ruin", "castle", "bank", "shop", "door", "wall", "fence", "grave",
        "tent", "altar", "monument", "cauldron", "fountain", "banner", "hut", "village", "tower",
    ]
    .iter()
    .any(|needle| name.contains(needle))
    {
        return 1;
    }
    2
}

struct ScenerySink<'a> {
    model_cache: &'a mut HashMap<u32, Model>,
    model_mesh: &'a mut MeshData,
    shadow_mesh: &'a mut MeshData,
    shadow_caster_mesh: &'a mut MeshData,
    collision_grid: &'a mut Grid,
}

struct TreeContext<'a> {
    cache: &'a Cache,
    map: &'a MapLayout,
    height: &'a HeightGrid,
    structures: &'a [Vec3],
    river_blocked: &'a [(i32, i32)],
    origin: (i32, i32),
}

/// Place one broken castle wall or ruin cluster. Ruins sit behind the tree
/// line so they never block a lane.
#[allow(clippy::too_many_arguments)]
fn place_ruin(
    ctx: &TreeContext<'_>,
    sink: &mut ScenerySink<'_>,
    pos: Vec3,
    seed: u32,
    model_id: u32,
    footprint: i32,
    count: &mut usize,
) {
    let tile_x = (pos.x / 128.0).floor() as i32;
    let tile_y = (pos.z / 128.0).floor() as i32;
    if !ctx.height.contains(pos.x, pos.z) {
        return;
    }
    if ctx
        .structures
        .iter()
        .any(|s| s.distance(pos) < 10.0 * 128.0)
    {
        return;
    }
    if ctx.map.distance_to_lane(pos.x, pos.z) < 8.0 * 128.0 {
        return;
    }
    if ctx.map.in_biome(tile_x, tile_y) {
        return;
    }
    let gx = tile_x - ctx.origin.0;
    let gy = tile_y - ctx.origin.1;
    if ctx.river_blocked.contains(&(gx, gy)) {
        return;
    }

    let model = if let Some(model) = sink.model_cache.get(&model_id) {
        model
    } else {
        let Ok(group) = ctx.cache.group(7, model_id) else {
            return;
        };
        let Ok(model) = Model::load(model_id, &group.data) else {
            return;
        };
        sink.model_cache.insert(model_id, model);
        sink.model_cache.get(&model_id).unwrap()
    };

    let base = Vec3::new(pos.x, ctx.height.height_at(pos.x, pos.z), pos.z);
    let angle = (hash01(seed ^ 0x1234_5678) * 4.0).floor() * std::f32::consts::FRAC_PI_2;
    let scale = RUIN_SCALE_MIN + hash01(seed.wrapping_add(3)) * (RUIN_SCALE_MAX - RUIN_SCALE_MIN);
    append_model_scaled_tinted(
        sink.model_mesh,
        model,
        base,
        angle,
        scale,
        [0.92, 0.9, 0.86],
        pick_texture(TEX_STONE, seed),
        false,
    );
    append_model_caster(
        sink.shadow_caster_mesh,
        model,
        base,
        angle,
        scale,
        CasterStyle::Building,
    );
    push_blob_quad(
        sink.shadow_mesh,
        base,
        (165.0 * scale).clamp(180.0, 340.0),
        0.22,
    );
    sink.collision_grid
        .block_rect(gx - footprint / 2, gy - footprint / 2, footprint, footprint);
    *count += 1;
}

/// Larger thematic props that break up the tree line without bringing back
/// the tall palm/jungle pillars: mossy boulders, low bushes and ferns.
fn place_lane_prop(
    ctx: &TreeContext<'_>,
    sink: &mut ScenerySink<'_>,
    pos: Vec3,
    seed: u32,
    count: &mut usize,
) {
    let tile_x = (pos.x / 128.0).floor() as i32;
    let tile_y = (pos.z / 128.0).floor() as i32;
    if !ctx.height.contains(pos.x, pos.z)
        || ctx.structures.iter().any(|s| s.distance(pos) < 9.0 * 128.0)
        || ctx.map.distance_to_lane(pos.x, pos.z) < 5.2 * 128.0
        || ctx.map.in_biome(tile_x, tile_y)
    {
        return;
    }
    let gx = tile_x - ctx.origin.0;
    let gy = tile_y - ctx.origin.1;
    if ctx.river_blocked.contains(&(gx, gy)) {
        return;
    }

    // Boulders use stone; bushes and ferns use foliage.
    let props: &[(u32, bool)] = &[
        (1669, true),
        (1670, true),
        (1671, true),
        (1672, true),
        (1773, false),
        (1805, false),
        (1391, false),
    ];
    let (model_id, leafy) = props[(seed as usize) % props.len()];
    let model = if let Some(model) = sink.model_cache.get(&model_id) {
        model
    } else {
        let Ok(group) = ctx.cache.group(7, model_id) else {
            return;
        };
        let Ok(model) = Model::load(model_id, &group.data) else {
            return;
        };
        sink.model_cache.insert(model_id, model);
        sink.model_cache.get(&model_id).unwrap()
    };
    let base = Vec3::new(pos.x, ctx.height.height_at(pos.x, pos.z), pos.z);
    let angle = hash01(seed ^ 0x7a3d_2b19) * std::f32::consts::TAU;
    let scale = 0.75 + hash01(seed.wrapping_add(9)) * 0.45;
    append_model_scaled_tinted(
        sink.model_mesh,
        model,
        base,
        angle,
        scale,
        if leafy {
            [0.92, 1.0, 0.88]
        } else {
            [0.96, 0.94, 0.88]
        },
        pick_texture(if leafy { TEX_LEAVES } else { TEX_STONE }, seed),
        false,
    );
    push_blob_quad(sink.shadow_mesh, base, 110.0 * scale, 0.2);
    if !leafy {
        sink.collision_grid.block_rect(gx, gy, 1, 1);
    }
    *count += 1;
}

/// Plant one OSRS tree model. Trees skirt the lanes, avoid structures and
/// pockets, and tint themselves by the side of the river they stand on.
fn place_tree(
    ctx: &TreeContext<'_>,
    sink: &mut ScenerySink<'_>,
    pos: Vec3,
    seed: u32,
    count: &mut usize,
) {
    let tile_x = (pos.x / 128.0).floor() as i32;
    let tile_y = (pos.z / 128.0).floor() as i32;
    if !ctx.height.contains(pos.x, pos.z) {
        return;
    }
    if ctx.structures.iter().any(|s| s.distance(pos) < 9.0 * 128.0) {
        return;
    }
    if ctx.map.distance_to_lane(pos.x, pos.z) < 5.2 * 128.0 {
        return;
    }
    if ctx.map.in_biome(tile_x, tile_y) {
        return;
    }
    let gx = tile_x - ctx.origin.0;
    let gy = tile_y - ctx.origin.1;
    if ctx.river_blocked.contains(&(gx, gy)) {
        return;
    }

    // The pool is the same on both sides of the river; the dire side only
    // tints its canopy darker.
    let side = ctx.map.river_side(tile_x as f32, tile_y as f32);
    // Only models with a real brown trunk and a full canopy belong in the
    // lane tree line. The older pool mixed in bare tropical stems and dead
    // trunks, which rendered as tall green pillars when given a leaf texture.
    let pool: &[u32] = if side < 0.0 {
        &[1570, 1611, 1637, 1650, 1614]
    } else {
        &[1570, 1637, 1611, 1650, 1614]
    };
    let model_id = pool[(seed as usize) % pool.len()];
    let model = if let Some(model) = sink.model_cache.get(&model_id) {
        model
    } else {
        let Ok(group) = ctx.cache.group(7, model_id) else {
            return;
        };
        let Ok(model) = Model::load(model_id, &group.data) else {
            return;
        };
        sink.model_cache.insert(model_id, model);
        sink.model_cache.get(&model_id).unwrap()
    };

    let base = Vec3::new(pos.x, ctx.height.height_at(pos.x, pos.z), pos.z);
    let angle = hash01(seed ^ 0x5bf0_3635) * std::f32::consts::TAU;
    let scale = TREE_LINE_SCALE_MIN
        + hash01(seed.wrapping_add(7)) * (TREE_LINE_SCALE_MAX - TREE_LINE_SCALE_MIN);
    // The dire side's canopy is drier and darker; it stays a canopy. It
    // used to pass `wood_only` here, which put bark on every face of these
    // full-canopy models (and over their own cache textures), so the whole
    // dire tree line rendered as bark cones and bark blobs.
    let tint = if side < 0.0 {
        [0.86f32, 0.84, 0.76]
    } else {
        [0.78, 1.06, 0.94]
    };
    append_model_scaled_tinted(
        sink.model_mesh,
        model,
        base,
        angle,
        scale,
        tint,
        pick_texture(TEX_LEAVES, seed),
        false,
    );
    append_model_caster(
        sink.shadow_caster_mesh,
        model,
        base,
        angle,
        scale,
        CasterStyle::Tree,
    );
    push_blob_quad(
        sink.shadow_mesh,
        base,
        (150.0 * scale).clamp(180.0, 350.0),
        0.22,
    );
    sink.collision_grid.block_rect(gx, gy, 2, 2);
    *count += 1;
}

fn push_blob_quad(mesh: &mut MeshData, center: Vec3, size: f32, alpha: f32) {
    let y = center.y + 4.0;
    let base = mesh.vertices.len() as u32;
    mesh.vertices.push(SceneVertex::blob(
        Vec3::new(center.x - size, y, center.z - size),
        alpha,
        [0.0, 0.0],
    ));
    mesh.vertices.push(SceneVertex::blob(
        Vec3::new(center.x + size, y, center.z - size),
        alpha,
        [1.0, 0.0],
    ));
    mesh.vertices.push(SceneVertex::blob(
        Vec3::new(center.x - size, y, center.z + size),
        alpha,
        [0.0, 1.0],
    ));
    mesh.vertices.push(SceneVertex::blob(
        Vec3::new(center.x + size, y, center.z + size),
        alpha,
        [1.0, 1.0],
    ));
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
}

#[derive(Debug, Clone, Copy)]
enum ModelStyle {
    Scenery {
        tint: [f32; 3],
        texture_hint: u32,
        /// Dead, snowy and bare-branch trees use bark on every face.
        wood_only: bool,
    },
    Decor {
        kind: GroundDecor,
        seed: u32,
    },
    /// Castle pieces force one bright stone texture across every face so the
    /// cache wall mesh reads as the white fortress in the reference.
    #[allow(dead_code)]
    Castle {
        tint: [f32; 3],
        texture: u32,
    },
}

/// A small palette of real cache texture IDs used when a model has no texture
/// reference of its own. Keeping every face on a cache texture prevents the
/// flat-colour triangles that used to clash with the sampled foliage and stone.
const TEX_LEAVES: &[u32] = &[8, 28, 30, 41, 60];
/// Texture hint for props that keep their plain cache colours.
const TEX_PLAIN: u32 = u32::MAX;
const TEX_STONE: &[u32] = &[2, 10, 44, 45, 46, 47];
const TEX_WOOD: &[u32] = &[5, 16, 22, 23];
const TEX_BARK: &[u32] = &[5, 23];
const TEX_FLOWERS: &[u32] = &[31, 40, 56];
const TEX_ROOF: &[u32] = &[6, 50];

fn pick_texture(pool: &[u32], seed: u32) -> u32 {
    let index = (hash01(seed) * pool.len() as f32) as usize;
    pool[index.min(pool.len() - 1)]
}

fn is_dead_tree(def: &ObjectDefinition) -> bool {
    let name = def.name.to_lowercase();
    [
        "dead", "dying", "burnt", "charred", "bare", "snow", "winter",
    ]
    .iter()
    .any(|needle| name.contains(needle))
}

fn scenery_texture_hint(def: &ObjectDefinition, foliage: bool) -> u32 {
    let name = def.name.to_lowercase();
    let seed = def
        .id
        .wrapping_mul(0x9E37_79B9)
        .wrapping_add(def.size_x as u32);
    if foliage {
        return pick_texture(TEX_LEAVES, seed);
    }
    if name.contains("roof") || name.contains("thatch") {
        return pick_texture(TEX_ROOF, seed);
    }
    if name.contains("door")
        || name.contains("hut")
        || name.contains("tent")
        || name.contains("table")
        || name.contains("chair")
        || name.contains("sign")
        || name.contains("bank")
        || name.contains("stall")
    {
        return pick_texture(TEX_WOOD, seed);
    }
    pick_texture(TEX_STONE, seed)
}

fn decor_texture_hint(kind: GroundDecor, seed: u32) -> u32 {
    match kind {
        GroundDecor::Flower | GroundDecor::Mushroom => pick_texture(TEX_FLOWERS, seed),
        GroundDecor::Grass | GroundDecor::Fern | GroundDecor::Plant => {
            pick_texture(TEX_LEAVES, seed)
        }
        GroundDecor::Rock => pick_texture(TEX_STONE, seed),
        GroundDecor::Twig => pick_texture(TEX_WOOD, seed),
    }
}

/// Most frequent valid face texture, used to fill in untextured faces on an
/// otherwise textured model so the model does not show flat-colour patches.
pub fn model_dominant_texture(model: &Model) -> Option<u32> {
    let textures = model.face_textures.as_ref()?;
    let mut counts = [0u16; 256];
    for &texture in textures {
        if (0..256).contains(&texture) {
            counts[texture as usize] = counts[texture as usize].saturating_add(1);
        }
    }
    counts
        .iter()
        .enumerate()
        .max_by_key(|(_, count)| **count)
        .filter(|(_, count)| **count > 0)
        .map(|(id, _)| id as u32)
}

#[allow(clippy::too_many_arguments)]
fn append_castle_mesh(
    pieces: &[CastlePiece],
    height: &HeightGrid,
    model_mesh: &mut MeshData,
    shadow_caster_mesh: &mut MeshData,
) -> usize {
    let mut placed = 0usize;
    for piece in pieces {
        let wx = (piece.tile[0] + 0.5) * 128.0;
        let wz = (piece.tile[1] + 0.5) * 128.0;
        let base = Vec3::new(wx, height.height_at(wx, wz), wz);
        let rot = Mat3::from_rotation_y(piece.yaw);
        tracing::trace!(
            "castle {:?} model {} texture {}",
            piece.kind,
            piece.model,
            piece.texture
        );
        let pal = CastlePalette::for_team(piece.team);
        match piece.kind {
            CastlePieceKind::Wall => {
                append_castle_wall(model_mesh, base, rot, piece.scale, &pal);
                append_box(
                    shadow_caster_mesh,
                    rot,
                    base,
                    Vec3::new(-64.0, 0.0, -42.0) * piece.scale,
                    Vec3::new(64.0, 270.0, 42.0) * piece.scale,
                );
            }
            CastlePieceKind::Tower => {
                append_castle_tower(model_mesh, base, rot, piece.scale, &pal);
                append_box(
                    shadow_caster_mesh,
                    rot,
                    base,
                    Vec3::new(-96.0, 0.0, -96.0) * piece.scale,
                    Vec3::new(96.0, 480.0, 96.0) * piece.scale,
                );
            }
            CastlePieceKind::Gate => {
                append_castle_gate(model_mesh, base, rot, piece.scale, &pal);
                append_box(
                    shadow_caster_mesh,
                    rot,
                    base,
                    Vec3::new(-560.0, 0.0, -50.0) * piece.scale,
                    Vec3::new(560.0, 420.0, 50.0) * piece.scale,
                );
            }
            CastlePieceKind::Platform => {
                append_castle_platform(model_mesh, base, piece.scale, &pal);
            }
            CastlePieceKind::Statue => {
                append_castle_statue(model_mesh, base, rot, piece.scale, &pal);
            }
        }
        placed += 1;
    }
    placed
}

const CASTLE_TOP: [u8; 3] = [234, 232, 223];
const CASTLE_SIDE: [u8; 3] = [222, 219, 209];
const CASTLE_DARK: [u8; 3] = [188, 185, 176];
const CASTLE_BLUE: [u8; 3] = [92, 146, 214];

/// Stone colours for one stretch of castle. The open perimeter is plain
/// white; each courtyard wears its god's colours: Saradomin white with a
/// blue walkway band and gold merlons, Zamorak dark stone with a blood-red
/// band and a red dais pool.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CastlePalette {
    top: [u8; 3],
    side: [u8; 3],
    dark: [u8; 3],
    /// Walkway caps and gate lintels.
    band: [u8; 3],
    /// Crenellations.
    merlon: [u8; 3],
    /// The pool ring on the nexus dais.
    pool: [u8; 3],
}

impl CastlePalette {
    const NEUTRAL: Self = Self {
        top: CASTLE_TOP,
        side: CASTLE_SIDE,
        dark: CASTLE_DARK,
        band: CASTLE_TOP,
        merlon: CASTLE_TOP,
        pool: CASTLE_BLUE,
    };
    const SARADOMIN: Self = Self {
        top: CASTLE_TOP,
        side: CASTLE_SIDE,
        dark: CASTLE_DARK,
        band: [64, 104, 196],
        merlon: [226, 184, 72],
        pool: CASTLE_BLUE,
    };
    const ZAMORAK: Self = Self {
        top: [104, 96, 98],
        side: [86, 79, 82],
        dark: [58, 53, 56],
        band: [168, 34, 28],
        merlon: [70, 62, 66],
        pool: [150, 34, 30],
    };

    fn for_team(team: Option<u8>) -> Self {
        match team {
            Some(0) => Self::SARADOMIN,
            Some(_) => Self::ZAMORAK,
            None => Self::NEUTRAL,
        }
    }
}

fn push_stone_quad(mesh: &mut MeshData, a: Vec3, b: Vec3, c: Vec3, d: Vec3, color: [u8; 3]) {
    let base = mesh.vertices.len() as u32;
    let normal = (b - a).cross(c - a).normalize_or_zero();
    for p in [a, b, c, d] {
        mesh.vertices.push(SceneVertex::stone(p, normal, color));
    }
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
}

fn push_stone_tri(mesh: &mut MeshData, a: Vec3, b: Vec3, c: Vec3, color: [u8; 3]) {
    let base = mesh.vertices.len() as u32;
    let normal = (b - a).cross(c - a).normalize_or_zero();
    for p in [a, b, c] {
        mesh.vertices.push(SceneVertex::stone(p, normal, color));
    }
    mesh.indices.extend_from_slice(&[base, base + 1, base + 2]);
}

#[allow(clippy::too_many_arguments)]
fn append_castle_box(
    mesh: &mut MeshData,
    base: Vec3,
    rot: Mat3,
    center: Vec3,
    half: Vec3,
    top: [u8; 3],
    side: [u8; 3],
    bottom: [u8; 3],
) {
    let p = |x: f32, y: f32, z: f32| {
        base + rot * (center + Vec3::new(x * half.x, y * half.y, z * half.z))
    };
    // Top and bottom.
    push_stone_quad(
        mesh,
        p(-1.0, 1.0, -1.0),
        p(1.0, 1.0, -1.0),
        p(1.0, 1.0, 1.0),
        p(-1.0, 1.0, 1.0),
        top,
    );
    push_stone_quad(
        mesh,
        p(-1.0, -1.0, -1.0),
        p(1.0, -1.0, -1.0),
        p(1.0, -1.0, 1.0),
        p(-1.0, -1.0, 1.0),
        bottom,
    );
    // Four sides.
    push_stone_quad(
        mesh,
        p(-1.0, -1.0, -1.0),
        p(1.0, -1.0, -1.0),
        p(1.0, 1.0, -1.0),
        p(-1.0, 1.0, -1.0),
        side,
    );
    push_stone_quad(
        mesh,
        p(-1.0, -1.0, 1.0),
        p(1.0, -1.0, 1.0),
        p(1.0, 1.0, 1.0),
        p(-1.0, 1.0, 1.0),
        side,
    );
    push_stone_quad(
        mesh,
        p(-1.0, -1.0, -1.0),
        p(-1.0, -1.0, 1.0),
        p(-1.0, 1.0, 1.0),
        p(-1.0, 1.0, -1.0),
        side,
    );
    push_stone_quad(
        mesh,
        p(1.0, -1.0, -1.0),
        p(1.0, -1.0, 1.0),
        p(1.0, 1.0, 1.0),
        p(1.0, 1.0, -1.0),
        side,
    );
}

fn append_castle_wall(mesh: &mut MeshData, base: Vec3, rot: Mat3, scale: f32, pal: &CastlePalette) {
    let s = scale;
    let half_w = 64.0 * s;
    let half_d = 42.0 * s;
    let height = 270.0 * s;
    // Plinth.
    append_castle_box(
        mesh,
        base,
        rot,
        Vec3::new(0.0, 18.0 * s, 0.0),
        Vec3::new(half_w + 4.0 * s, 18.0 * s, half_d + 6.0 * s),
        pal.side,
        pal.side,
        pal.dark,
    );
    // Body.
    append_castle_box(
        mesh,
        base,
        rot,
        Vec3::new(0.0, height * 0.5, 0.0),
        Vec3::new(half_w, height * 0.5, half_d),
        pal.top,
        pal.side,
        pal.dark,
    );
    // Walkway cap.
    append_castle_box(
        mesh,
        base,
        rot,
        Vec3::new(0.0, height + 5.0 * s, 0.0),
        Vec3::new(half_w + 5.0 * s, 5.0 * s, half_d + 5.0 * s),
        pal.band,
        pal.band,
        pal.dark,
    );
    // Merlon.
    append_castle_box(
        mesh,
        base,
        rot,
        Vec3::new(0.0, height + 10.0 * s + 26.0 * s, 0.0),
        Vec3::new(22.0 * s, 26.0 * s, half_d * 0.78),
        pal.merlon,
        pal.merlon,
        pal.dark,
    );
}

fn append_castle_tower(
    mesh: &mut MeshData,
    base: Vec3,
    rot: Mat3,
    scale: f32,
    pal: &CastlePalette,
) {
    let s = scale;
    let height = 480.0 * s;
    append_castle_box(
        mesh,
        base,
        rot,
        Vec3::new(0.0, 18.0 * s, 0.0),
        Vec3::new(110.0 * s, 18.0 * s, 110.0 * s),
        pal.side,
        pal.side,
        pal.dark,
    );
    append_castle_box(
        mesh,
        base,
        rot,
        Vec3::new(0.0, height * 0.5, 0.0),
        Vec3::new(96.0 * s, height * 0.5, 96.0 * s),
        pal.top,
        pal.side,
        pal.dark,
    );
    append_castle_box(
        mesh,
        base,
        rot,
        Vec3::new(0.0, height + 8.0 * s, 0.0),
        Vec3::new(116.0 * s, 8.0 * s, 116.0 * s),
        pal.band,
        pal.band,
        pal.dark,
    );
    for (x, z) in [(-82.0, -82.0), (82.0, -82.0), (-82.0, 82.0), (82.0, 82.0)] {
        append_castle_box(
            mesh,
            base,
            rot,
            Vec3::new(x * s, height + 16.0 * s + 28.0 * s, z * s),
            Vec3::new(26.0 * s, 28.0 * s, 26.0 * s),
            pal.merlon,
            pal.merlon,
            pal.dark,
        );
    }
}

fn append_castle_gate(mesh: &mut MeshData, base: Vec3, rot: Mat3, scale: f32, pal: &CastlePalette) {
    let s = scale;
    let span = 512.0 * s;
    let pillar_h = 380.0 * s;
    for side in [-1.0, 1.0] {
        append_castle_box(
            mesh,
            base,
            rot,
            Vec3::new(side * (span + 30.0 * s), pillar_h * 0.5, 0.0),
            Vec3::new(34.0 * s, pillar_h * 0.5, 48.0 * s),
            pal.top,
            pal.side,
            pal.dark,
        );
    }
    append_castle_box(
        mesh,
        base,
        rot,
        Vec3::new(0.0, pillar_h + 30.0 * s, 0.0),
        Vec3::new(span + 64.0 * s, 30.0 * s, 48.0 * s),
        pal.band,
        pal.band,
        pal.dark,
    );
    for x in [-span * 0.6, 0.0, span * 0.6] {
        append_castle_box(
            mesh,
            base,
            rot,
            Vec3::new(x, pillar_h + 60.0 * s + 26.0 * s, 0.0),
            Vec3::new(30.0 * s, 26.0 * s, 40.0 * s),
            pal.merlon,
            pal.merlon,
            pal.dark,
        );
    }
}

fn append_castle_platform(mesh: &mut MeshData, base: Vec3, scale: f32, pal: &CastlePalette) {
    let s = scale;
    let outer = 680.0 * s;
    let inner = 540.0 * s;
    let height = 10.0 * s;
    let segments = 24u32;
    let center = base + Vec3::Y * height;
    let inner_center = base + Vec3::Y * (height + 4.0 * s);
    for i in 0..segments {
        let a = i as f32 / segments as f32 * std::f32::consts::TAU;
        let b = (i + 1) as f32 / segments as f32 * std::f32::consts::TAU;
        let outer_a = center + Vec3::new(a.cos() * outer, 0.0, a.sin() * outer);
        let outer_b = center + Vec3::new(b.cos() * outer, 0.0, b.sin() * outer);
        let inner_a = inner_center + Vec3::new(a.cos() * inner, 0.0, a.sin() * inner);
        let inner_b = inner_center + Vec3::new(b.cos() * inner, 0.0, b.sin() * inner);
        push_stone_tri(mesh, center, outer_a, outer_b, pal.top);
        push_stone_tri(mesh, inner_center, inner_a, inner_b, pal.pool);
        // Side skirt.
        let foot_a = base + Vec3::new(a.cos() * outer, 0.0, a.sin() * outer);
        let foot_b = base + Vec3::new(b.cos() * outer, 0.0, b.sin() * outer);
        push_stone_quad(mesh, foot_a, outer_a, outer_b, foot_b, pal.side);
    }
}

fn append_castle_statue(
    mesh: &mut MeshData,
    base: Vec3,
    rot: Mat3,
    scale: f32,
    pal: &CastlePalette,
) {
    let s = scale;
    append_castle_box(
        mesh,
        base,
        rot,
        Vec3::new(0.0, 20.0 * s, 0.0),
        Vec3::new(44.0 * s, 20.0 * s, 44.0 * s),
        pal.top,
        pal.side,
        pal.dark,
    );
    append_castle_box(
        mesh,
        base,
        rot,
        Vec3::new(0.0, 105.0 * s, 0.0),
        Vec3::new(24.0 * s, 85.0 * s, 24.0 * s),
        pal.top,
        pal.side,
        pal.dark,
    );
    append_castle_box(
        mesh,
        base,
        rot,
        Vec3::new(0.0, 205.0 * s, 0.0),
        Vec3::new(38.0 * s, 18.0 * s, 38.0 * s),
        pal.top,
        pal.side,
        pal.dark,
    );
}

#[allow(clippy::too_many_arguments)]
fn append_model_scaled_tinted(
    mesh: &mut MeshData,
    model: &Model,
    base: Vec3,
    angle: f32,
    scale: f32,
    tint: [f32; 3],
    texture_hint: u32,
    wood_only: bool,
) {
    append_model_styled(
        mesh,
        model,
        base,
        angle,
        scale,
        ModelStyle::Scenery {
            tint,
            texture_hint,
            wood_only,
        },
    );
}

fn append_ground_decor(
    mesh: &mut MeshData,
    model: &Model,
    base: Vec3,
    angle: f32,
    scale: f32,
    kind: GroundDecor,
    seed: u32,
) {
    append_model_styled(
        mesh,
        model,
        base,
        angle,
        scale,
        ModelStyle::Decor { kind, seed },
    );
}

#[derive(Clone, Copy)]
enum CasterStyle {
    Tree,
    Building,
}

/// Low-poly proxy for the sun shadow map. Re-rendering every static tree and
/// ruin face into a 1024px map was the largest remaining renderer cost; a
/// trunk/canopy box pair preserves the silhouette at shadow-map resolution
/// for a fraction of the triangles.
fn append_model_caster(
    mesh: &mut MeshData,
    model: &Model,
    base: Vec3,
    angle: f32,
    scale: f32,
    style: CasterStyle,
) {
    let rot = Mat3::from_rotation_y(angle);
    let (bounds_min, bounds_max) = model.bounds();
    let min = Vec3::new(
        bounds_min[0] as f32,
        -(bounds_max[1] as f32),
        bounds_min[2] as f32,
    ) * scale;
    let max = Vec3::new(
        bounds_max[0] as f32,
        -(bounds_min[1] as f32),
        bounds_max[2] as f32,
    ) * scale;
    match style {
        CasterStyle::Tree => {
            let height = (max.y - min.y).max(1.0);
            let center = (min + max) * 0.5;
            let half_w = (max.x - min.x).max(1.0) * 0.5;
            let half_d = (max.z - min.z).max(1.0) * 0.5;
            // Trunk: narrow column from the ground to the lower canopy.
            append_box(
                mesh,
                rot,
                base,
                Vec3::new(center.x - half_w * 0.22, min.y, center.z - half_d * 0.22),
                Vec3::new(
                    center.x + half_w * 0.22,
                    min.y + height * 0.42,
                    center.z + half_d * 0.22,
                ),
            );
            // Canopy: broad box from the lower branches to the crown.
            append_box(
                mesh,
                rot,
                base,
                Vec3::new(min.x, min.y + height * 0.28, min.z),
                max,
            );
        }
        CasterStyle::Building => append_box(mesh, rot, base, min, max),
    }
}

fn append_box(mesh: &mut MeshData, rot: Mat3, base: Vec3, min: Vec3, max: Vec3) {
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(max.x, max.y, max.z),
    ];
    let corner = |index: usize| base + rot * corners[index];
    for [a, b, c] in [
        [0, 2, 1],
        [1, 2, 3],
        [4, 5, 6],
        [5, 7, 6],
        [0, 1, 4],
        [1, 5, 4],
        [2, 6, 3],
        [3, 6, 7],
        [0, 4, 2],
        [2, 4, 6],
        [1, 3, 5],
        [3, 7, 5],
    ] {
        mesh.push_triangle(
            SceneVertex::new(corner(a), Vec3::Y, [0, 0, 0]),
            SceneVertex::new(corner(b), Vec3::Y, [0, 0, 0]),
            SceneVertex::new(corner(c), Vec3::Y, [0, 0, 0]),
        );
    }
}

fn append_model_styled(
    mesh: &mut MeshData,
    model: &Model,
    base: Vec3,
    angle: f32,
    scale: f32,
    style: ModelStyle,
) {
    let rot = Mat3::from_rotation_y(angle);
    let dominant = model_dominant_texture(model);
    let (bounds_min, bounds_max) = model.bounds();
    let model_height = ((bounds_max[1] - bounds_min[1]).max(1) as f32 * scale).max(1.0);
    let wood_hint = pick_texture(
        TEX_BARK,
        (base.x as u32)
            .wrapping_mul(0x9E37_79B9)
            .wrapping_add(base.z as u32),
    );
    let wood_only = matches!(
        style,
        ModelStyle::Scenery {
            wood_only: true,
            ..
        }
    );

    for (face_idx, &[a, b, c]) in model.faces.iter().enumerate() {
        let pa = model_vertex_scaled(model, a as usize, base, rot, scale);
        let pb = model_vertex_scaled(model, b as usize, base, rot, scale);
        let pc = model_vertex_scaled(model, c as usize, base, rot, scale);

        let mut normal = (pb - pa).cross(pc - pa).normalize_or_zero();
        if normal.length_squared() < 1e-8 {
            normal = rot
                * Vec3::from_array(
                    model
                        .face_normals
                        .get(face_idx)
                        .copied()
                        .unwrap_or([0.0, 1.0, 0.0]),
                );
        }

        // The face's own cache texture, when it has one.
        let own_texture = model
            .face_textures
            .as_ref()
            .and_then(|t| t.get(face_idx))
            .copied()
            .map(|t| t as u32)
            .filter(|&t| t < 256);
        // Props that ask for "plain" keep their cache vertex colours: only
        // faces with their own cache texture are textured.
        let plain = own_texture.is_none()
            && matches!(style, ModelStyle::Scenery { texture_hint, .. } if texture_hint == TEX_PLAIN);
        let mut texture_id = match style {
            ModelStyle::Castle { texture, .. } => texture,
            _ if wood_only => wood_hint,
            _ => own_texture.or(dominant).unwrap_or_else(|| match style {
                ModelStyle::Scenery { texture_hint, .. } => texture_hint,
                ModelStyle::Decor { kind, seed } => decor_texture_hint(kind, seed),
                ModelStyle::Castle { texture, .. } => texture,
            }),
        };
        let leaf_based = !wood_only
            && !matches!(style, ModelStyle::Castle { .. })
            && (dominant.is_some_and(|id| TEX_LEAVES.contains(&id))
                || matches!(style, ModelStyle::Scenery { texture_hint, .. } if TEX_LEAVES.contains(&texture_hint)));
        if leaf_based {
            let centroid = (pa + pb + pc) / 3.0;
            let top = (centroid.y - base.y) / model_height;
            let radial = Vec2::new(centroid.x - base.x, centroid.z - base.z).length();
            let trunk_radius =
                (((bounds_max[0] - bounds_min[0]).max(bounds_max[2] - bounds_min[2])) as f32
                    * scale
                    * 0.20)
                    .max(28.0);
            // Some broadleaf cache models carry their trunk almost to the
            // crown; only the outer canopy should receive leaf texture.
            // Faces the cache textures itself keep their texture: an
            // evergreen's narrow top tier sits inside the trunk radius and
            // was painted as a bark ring.
            if own_texture.is_none() && top < 0.85 && radial < trunk_radius {
                texture_id = wood_hint;
            }
        }
        let rgb = match style {
            ModelStyle::Scenery { tint, .. } if plain => {
                let colour = model
                    .face_colors
                    .get(face_idx)
                    .map_or([200, 200, 200], |&hsl| {
                        rift_cache::model::hsl_to_rgb(hsl, 1.0)
                    });
                [
                    (colour[0] as f32 * tint[0]).clamp(0.0, 255.0) as u8,
                    (colour[1] as f32 * tint[1]).clamp(0.0, 255.0) as u8,
                    (colour[2] as f32 * tint[2]).clamp(0.0, 255.0) as u8,
                ]
            }
            ModelStyle::Scenery { tint, .. } => [
                (tint[0] * 255.0).clamp(0.0, 255.0) as u8,
                (tint[1] * 255.0).clamp(0.0, 255.0) as u8,
                (tint[2] * 255.0).clamp(0.0, 255.0) as u8,
            ],
            ModelStyle::Decor { .. } => [255, 255, 255],
            ModelStyle::Castle { tint, .. } => [
                (tint[0] * 255.0).clamp(0.0, 255.0) as u8,
                (tint[1] * 255.0).clamp(0.0, 255.0) as u8,
                (tint[2] * 255.0).clamp(0.0, 255.0) as u8,
            ],
        };

        // One planar projection per face, chosen from the face normal. This
        // is the OSRS texture-plane model in miniature: large cone and roof
        // triangles stay coherent instead of smearing a triplanar blend.
        let abs_normal = normal.abs();
        let uv_for = |p: Vec3| {
            let local = p - base;
            if abs_normal.y >= abs_normal.x && abs_normal.y >= abs_normal.z {
                [local.x / 128.0, local.z / 128.0]
            } else if abs_normal.x >= abs_normal.z {
                [local.z / 128.0, local.y / 128.0]
            } else {
                [local.x / 128.0, local.y / 128.0]
            }
        };

        let ao_at = |p: Vec3| {
            (0.78 + 0.22 * ((p.y - base.y).max(0.0) / 400.0).clamp(0.0, 1.0)).clamp(0.70, 1.0)
        };
        let mut va = SceneVertex::new(pa, normal, rgb);
        let mut vb = SceneVertex::new(pb, normal, rgb);
        let mut vc = SceneVertex::new(pc, normal, rgb);
        // Material 7 = static cache-textured scenery. It samples textures and
        // lighting but does not receive the coarse static shadow proxies, which
        // otherwise paint box-shaped self-shadow patches across tree canopies.
        let material = if plain { 0 } else { 7 };
        if plain {
            texture_id = 0;
        }
        va.material = material;
        vb.material = material;
        vc.material = material;
        va.texture_id = texture_id;
        vb.texture_id = texture_id;
        vc.texture_id = texture_id;
        va.uv = uv_for(pa);
        vb.uv = uv_for(pb);
        vc.uv = uv_for(pc);
        va.ao = ao_at(pa);
        vb.ao = ao_at(pb);
        vc.ao = ao_at(pc);
        mesh.push_triangle(va, vb, vc);
    }
}

pub fn model_vertex_scaled(model: &Model, i: usize, base: Vec3, rot: Mat3, scale: f32) -> Vec3 {
    let local = Vec3::new(
        model.vertex_x[i] as f32,
        -(model.vertex_y[i] as f32),
        model.vertex_z[i] as f32,
    ) * scale;
    base + rot * local
}

pub fn model_vertex_animated(
    positions: &[[f32; 3]],
    i: usize,
    base: Vec3,
    rot: Mat3,
    scale: f32,
) -> Vec3 {
    let p = positions.get(i).copied().unwrap_or([0.0; 3]);
    let local = Vec3::new(p[0], -p[1], p[2]) * scale;
    base + rot * local
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_opened_disc_lets_a_walker_through_scenery_inside_it_only() {
        // A wall of scenery along x = 10 with a gap at the far end, origin offset so world and grid
        // coordinates differ.
        let mut grid = Grid::new(20, 20);
        for y in 0..16 {
            grid.set_blocked(10, y, true);
        }
        // A castle wall tile inside the web: it stays shut.
        let mut solid = Grid::new(20, 20);
        solid.set_blocked(10, 3, true);
        grid.set_blocked(10, 3, true);
        let sight = grid.clone();
        let collision = WorldCollision::new(grid, sight, 5, 7).with_solid(solid);
        let world = |tx: i32, ty: i32| {
            (
                (5 + tx) as f32 * 128.0 + 64.0,
                (7 + ty) as f32 * 128.0 + 64.0,
            )
        };
        let (wx, wz) = world(10, 4);
        assert!(!collision.is_walkable_world(wx, wz));

        let web = Vec3::new(wx, 0.0, wz);
        let opened = collision.opened(&[(web, 300.0)]);
        assert!(
            opened.is_walkable_world(wx, wz),
            "the wall inside the web opens"
        );
        let (fx, fz) = world(10, 12);
        assert!(
            !opened.is_walkable_world(fx, fz),
            "the wall outside the web stays"
        );
        let (sx, sz) = world(10, 3);
        assert!(
            !opened.is_walkable_world(sx, sz),
            "castle walls stay shut in a web"
        );
        assert!(opened.blocks_sight_tile(10, 4), "trees still block sight");
        assert!(
            !collision.is_walkable_world(wx, wz),
            "the shared collision is untouched"
        );

        // Straight across the web, not around the end of the wall.
        let height = HeightGrid {
            origin_tile_x: 5,
            origin_tile_y: 7,
            step: 128.0,
            size: 2,
            heights: vec![0.0; 4],
        };
        let (ax, az) = world(7, 4);
        let (bx, bz) = world(13, 4);
        let path = opened.path_world(Vec3::new(ax, 0.0, az), Vec3::new(bx, 0.0, bz), &height);
        assert_eq!(path.len(), 1, "one straight leg through the web: {path:?}");
        let around = collision.path_world(Vec3::new(ax, 0.0, az), Vec3::new(bx, 0.0, bz), &height);
        assert!(around.len() > 1, "without the web the path goes round");
    }

    #[test]
    fn srgb_endpoints_are_stable() {
        assert_eq!(srgb_to_linear(0.0), 0.0);
        assert!((srgb_to_linear(1.0) - 1.0).abs() < 1e-6);
        let mid = srgb_to_linear(0.5);
        assert!(mid > 0.2 && mid < 0.25);
    }

    #[test]
    fn scene_vertex_converts_colour_and_normalizes_normal() {
        let vertex = SceneVertex::new(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(0.0, 2.0, 0.0),
            [255, 0, 0],
        );
        assert!((vertex.normal[1] - 1.0).abs() < 1e-6);
        assert!((vertex.color[0] - 1.0).abs() < 1e-6);
        assert_eq!(vertex.color[1], 0.0);
        assert_eq!(vertex.material, 0);
    }

    #[test]
    fn unlit_and_blob_materials_are_tagged() {
        let unlit = SceneVertex::unlit(Vec3::ZERO, [10, 20, 30]);
        assert_eq!(unlit.material, 1);
        let blob = SceneVertex::blob(Vec3::ZERO, 0.4, [0.0, 0.0]);
        assert_eq!(blob.material, 2);
        assert!((blob.color[3] - 0.4).abs() < 1e-6);
    }

    #[test]
    fn bridge_tangent_follows_the_nearest_lane_segment() {
        let lanes = vec![LaneSpawn {
            id: "mid".to_string(),
            waypoints: vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(1280.0, 0.0, -1280.0)],
        }];
        let (tx, tz) = nearest_lane_tangent(&lanes, 1000.0, -1000.0).unwrap();
        let diagonal = 0.5f32.sqrt();
        assert!((tx - diagonal).abs() < 1e-5);
        assert!((tz + diagonal).abs() < 1e-5);
    }

    #[test]
    fn bilinear_samples_the_water_grid_corners_and_center() {
        let values = [0.0, 1.0, 2.0, 3.0];
        assert_eq!(bilinear(&values, 2, 0.0, 0.0), 0.0);
        assert_eq!(bilinear(&values, 2, 1.0, 1.0), 3.0);
        assert!((bilinear(&values, 2, 0.5, 0.5) - 1.5).abs() < 1e-6);
    }

    #[test]
    fn ground_overlay_packs_texture_and_weight_without_touching_the_variant() {
        let packed = pack_ground_overlay(129, 255);
        assert_eq!(packed & 0xff, 0);
        assert_eq!((packed >> 8) & 0xff, 129);
        assert_eq!((packed >> 16) & 0xff, 255);
        let with_grass = packed | 3;
        assert_eq!(with_grass & 0xff, 3);
        assert_eq!((with_grass >> 8) & 0xff, 129);
    }

    #[test]
    fn terrain_sculpting_is_deterministic_and_preserves_water() {
        let size = 64usize;
        let mut heights = vec![0.0f32; size * size];
        let mut water = vec![0.0f32; size * size];
        water[20 * size + 20] = 1.0;
        heights[20 * size + 20] = -37.0;

        let mut first = heights.clone();
        sculpt_terrain(&mut first, size, 0, 0, &water, None);
        let mut second = heights.clone();
        sculpt_terrain(&mut second, size, 0, 0, &water, None);
        assert_eq!(first, second);
        assert_eq!(first[20 * size + 20], -37.0);

        // The floor is calm: the broad swell rolls, but no noise hills.
        let dry: Vec<f32> = first
            .iter()
            .enumerate()
            .filter(|(i, _)| water[*i] == 0.0)
            .map(|(_, h)| *h)
            .collect();
        let min = dry.iter().copied().fold(f32::MAX, f32::min);
        let max = dry.iter().copied().fold(f32::MIN, f32::max);
        assert!(max - min > 5.0, "floor lost its swell: {}", max - min);
        assert!(max - min < 220.0, "floor too hilly: {}", max - min);
        assert!(first.iter().all(|height| height.is_finite()));
    }

    #[test]
    fn terrain_sculpting_compresses_tile_bumps() {
        let size = 48usize;
        let water = vec![0.0f32; size * size];
        let mut bumpy = vec![0.0f32; size * size];
        for (i, h) in bumpy.iter_mut().enumerate() {
            *h = if (i / size + i % size) % 2 == 0 {
                80.0
            } else {
                -80.0
            };
        }
        let mut flat = vec![0.0f32; size * size];
        sculpt_terrain(&mut bumpy, size, 0, 0, &water, None);
        sculpt_terrain(&mut flat, size, 0, 0, &water, None);
        // The relaxation leaves the outermost ring alone; judge the inside.
        let worst = (2..size - 2)
            .flat_map(|y| (2..size - 2).map(move |x| y * size + x))
            .map(|i| (bumpy[i] - flat[i]).abs())
            .fold(0.0f32, f32::max);
        assert!(worst < 30.0, "checkerboard survived: {worst}");
    }

    #[test]
    fn value_noise_is_continuous_across_lattice_lines() {
        for k in 0..64 {
            let x = k as f32 * 0.73 - 20.0;
            for edge in [3.0f32, -2.0, 17.0] {
                let below = value_noise(edge - 1e-4, x);
                let above = value_noise(edge + 1e-4, x);
                assert!(
                    (below - above).abs() < 1e-2,
                    "jump at x={edge}: {below} vs {above}"
                );
            }
        }
    }

    #[test]
    fn plateaus_are_terraced_and_respect_clearance() {
        let mut levels = [0usize; 3];
        for gy in 0..200 {
            for gx in 0..200 {
                let wx = gx as f32 * 96.0;
                let wz = gy as f32 * 96.0;
                assert_eq!(plateau_height(wx, wz, 0.0), 0.0);
                let h = plateau_height(wx, wz, 1.0);
                assert!(h.is_finite() && h > -60.0 && h < TERRACE_STEP * 2.0 + 60.0);
                // Most ground sits on a level, not on a riser.
                for (level, count) in levels.iter_mut().enumerate() {
                    if (h - level as f32 * TERRACE_STEP).abs() < 12.0 {
                        *count += 1;
                    }
                }
            }
        }
        let on_level: usize = levels.iter().sum();
        assert!(on_level > 200 * 200 * 3 / 4, "too much riser: {on_level}");
        assert!(levels[1] > 0, "no plateaus at all");
    }

    #[test]
    fn bicubic_and_bilinear_hit_grid_points() {
        let size = 6usize;
        let values: Vec<f32> = (0..size * size).map(|i| (i * 7 % 11) as f32).collect();
        for gy in 0..size {
            for gx in 0..size {
                let v = values[gy * size + gx];
                assert!((sample_bicubic(&values, size, gx as f32, gy as f32) - v).abs() < 1e-4);
                assert!((sample_bilinear(&values, size, gx as f32, gy as f32) - v).abs() < 1e-4);
            }
        }
    }

    #[test]
    fn height_grid_honours_its_sample_step() {
        let grid = HeightGrid {
            origin_tile_x: 10,
            origin_tile_y: 20,
            step: 64.0,
            size: 3,
            heights: vec![0.0, 10.0, 20.0, 0.0, 10.0, 20.0, 0.0, 10.0, 20.0],
        };
        let x = 10.0 * 128.0;
        let z = 20.0 * 128.0;
        assert!((grid.height_at(x + 64.0, z + 32.0) - 10.0).abs() < 1e-4);
        assert!((grid.height_at(x + 96.0, z) - 15.0).abs() < 1e-4);
        assert!(grid.contains(x + 100.0, z + 100.0));
        assert!(!grid.contains(x + 130.0, z));
    }

    #[test]
    fn castle_settlement_mask_levels_bases_and_wall_bands() {
        let castle = CastleSpec {
            bounds: (0.0, 0.0, 120.0, 120.0),
            wall_model: 16918,
            wall_scale: 1.0,
            tower_model: 1412,
            tower_scale: 1.0,
            gate_model: 3996,
            gate_scale: 1.0,
            platform_model: None,
            statue_model: None,
            tower_spacing: 0.25,
            bases: vec![CastleBase {
                team: 0,
                center: (18.0, 102.0),
                size: 28.0,
                corner: CastleCorner::BottomLeft,
            }],
        };
        assert_eq!(castle_settlement_mask(0.0, 60.0, Some(&castle)), 0.0);
        assert_eq!(castle_settlement_mask(18.0, 102.0, Some(&castle)), 0.0);
        assert_eq!(castle_settlement_mask(60.0, 60.0, Some(&castle)), 1.0);
        assert_eq!(castle_settlement_mask(200.0, 60.0, Some(&castle)), 1.0);
        assert_eq!(castle_settlement_mask(0.0, 0.0, None), 1.0);
    }

    #[test]
    fn atmosphere_defaults_are_sane() {
        let atmosphere = Atmosphere::default();
        assert!(atmosphere.fog_density > 0.0);
        assert!(atmosphere.fog_height > 0.0);
        assert!(atmosphere.sun_dir[1] > 0.0);
        assert!(atmosphere.sun_color.iter().all(|c| (0.0..=1.0).contains(c)));
    }

    #[test]
    fn castle_layout_has_gates_towers_and_blocked_walls() {
        let castle = CastleSpec {
            bounds: (0.0, 0.0, 120.0, 120.0),
            wall_model: 16918,
            wall_scale: 1.0,
            tower_model: 1412,
            tower_scale: 1.0,
            gate_model: 3996,
            gate_scale: 1.0,
            platform_model: Some(15082),
            statue_model: Some(1527),
            tower_spacing: 0.25,
            bases: vec![
                CastleBase {
                    team: 0,
                    center: (18.0, 102.0),
                    size: 28.0,
                    corner: CastleCorner::BottomLeft,
                },
                CastleBase {
                    team: 1,
                    center: (102.0, 18.0),
                    size: 28.0,
                    corner: CastleCorner::TopRight,
                },
            ],
        };
        let pieces = castle.layout();
        assert!(
            pieces
                .iter()
                .filter(|p| p.kind == CastlePieceKind::Wall)
                .count()
                > 300
        );
        assert!(
            pieces
                .iter()
                .filter(|p| p.kind == CastlePieceKind::Tower)
                .count()
                >= 12
        );
        assert_eq!(
            pieces
                .iter()
                .filter(|p| p.kind == CastlePieceKind::Gate)
                .count(),
            4
        );
        assert_eq!(
            pieces
                .iter()
                .filter(|p| p.kind == CastlePieceKind::Platform)
                .count(),
            2
        );
        assert_eq!(
            pieces
                .iter()
                .filter(|p| p.kind == CastlePieceKind::Statue)
                .count(),
            8
        );
        assert!(pieces.iter().any(|p| p.block));

        // The bottom-left courtyard's north and east gates must be open.
        for (x, z) in [(32.0, 102.0), (18.0, 88.0)] {
            assert!(
                !pieces.iter().any(|p| {
                    p.kind == CastlePieceKind::Wall
                        && (p.tile[0] - x).abs() < 0.1
                        && (p.tile[1] - z).abs() < 0.1
                }),
                "wall blocks gate at {x},{z}"
            );
        }

        // Each courtyard wears its god's colours; the open perimeter does not.
        for p in &pieces {
            if matches!(p.kind, CastlePieceKind::Gate | CastlePieceKind::Platform) {
                assert!(p.team.is_some(), "{:?} at {:?} has no team", p.kind, p.tile);
            }
        }
        let team_of = |x: f32, z: f32| {
            pieces
                .iter()
                .find(|p| (p.tile[0] - x).abs() < 0.1 && (p.tile[1] - z).abs() < 0.1)
                .map(|p| p.team)
        };
        assert_eq!(team_of(60.0, 0.0), Some(None), "mid perimeter is neutral");
        assert_eq!(
            team_of(32.0, 95.0),
            Some(Some(0)),
            "blue courtyard's inner wall"
        );
        assert_eq!(
            team_of(88.0, 10.0),
            Some(Some(1)),
            "red courtyard's inner wall"
        );
        assert_ne!(
            CastlePalette::for_team(Some(0)),
            CastlePalette::for_team(Some(1))
        );
        assert_eq!(CastlePalette::for_team(None), CastlePalette::NEUTRAL);
    }
}
