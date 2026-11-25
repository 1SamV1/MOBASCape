//! Arena set dressing from `map.json`'s `props` block (Phase 5).
//!
//! Every prop is a cache model. Three shapes of placement:
//! - `place`: one model at one tile (statues, banners, base pieces);
//! - `lines`: models strung along a polyline, or along both sides of every
//!   lane at an offset, with gaps left as jungle entrances (cliff rocks);
//! - `scatter`: a count of models dropped at random inside a rectangle
//!   (jungle ferns, stumps, flowers).
//!
//! Lines and scatter are seeded, so every match gets the same arena, and they
//! keep off lanes, structures, base courtyards and the river. A prop with a
//! `block` footprint blocks movement and sight; flora usually does not.

use std::collections::{HashMap, HashSet};

use glam::{Vec2, Vec3};
use rift_cache::{Cache, Model};
use rift_sim::Grid;
use serde::Deserialize;

use super::{
    CasterStyle, HeightGrid, MapLayout, MeshData, TEX_FLOWERS, TEX_LEAVES, TEX_STONE, TEX_WOOD,
    append_model_caster, append_model_scaled_tinted, cached_model, hash01, push_blob_quad,
};

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PropsJson {
    #[serde(default)]
    pub place: Vec<PlaceJson>,
    #[serde(default)]
    pub lines: Vec<LineJson>,
    #[serde(default)]
    pub scatter: Vec<ScatterJson>,
}

/// One model at one tile.
#[derive(Debug, Clone, Deserialize)]
pub struct PlaceJson {
    pub model: u32,
    /// World tile `[x, y]` (the prop sits on the tile centre).
    pub at: [f32; 2],
    /// Yaw in degrees.
    #[serde(default)]
    pub yaw: f32,
    #[serde(default = "one")]
    pub scale: f32,
    /// Square footprint in tiles that blocks movement and sight (0 = none).
    #[serde(default)]
    pub block: i32,
    #[serde(default = "white")]
    pub tint: [f32; 3],
    /// Texture pool for untextured faces: stone (default), wood, leaves,
    /// flowers, or plain to keep the cache colours.
    #[serde(default)]
    pub texture: Option<String>,
}

/// Models strung along a path.
#[derive(Debug, Clone, Deserialize)]
pub struct LineJson {
    pub models: Vec<u32>,
    /// A polyline of world tiles. Ignored when `lane_offset` is set.
    #[serde(default)]
    pub points: Vec<[f32; 2]>,
    /// Instead of `points`, run along both sides of every lane this many
    /// tiles out.
    #[serde(default)]
    pub lane_offset: Option<f32>,
    /// Tiles between models.
    #[serde(default = "default_spacing")]
    pub spacing: f32,
    /// Random sideways and along-path offset, in tiles.
    #[serde(default)]
    pub jitter: f32,
    /// Fraction of the path left open, in runs, as jungle entrances.
    #[serde(default)]
    pub gaps: f32,
    #[serde(default = "default_scale_range")]
    pub scale: [f32; 2],
    #[serde(default)]
    pub block: i32,
    #[serde(default = "white")]
    pub tint: [f32; 3],
    #[serde(default)]
    pub texture: Option<String>,
    /// Keep this many tiles clear of any lane centre line.
    #[serde(default = "default_lane_clear")]
    pub lane_clear: f32,
    #[serde(default)]
    pub seed: u32,
}

/// Models dropped at random inside a rectangle.
#[derive(Debug, Clone, Deserialize)]
pub struct ScatterJson {
    pub models: Vec<u32>,
    /// World tiles `[x0, y0, x1, y1]`.
    pub rect: [f32; 4],
    pub count: usize,
    #[serde(default = "default_scale_range")]
    pub scale: [f32; 2],
    #[serde(default)]
    pub block: i32,
    #[serde(default = "white")]
    pub tint: [f32; 3],
    #[serde(default)]
    pub texture: Option<String>,
    #[serde(default = "default_lane_clear")]
    pub lane_clear: f32,
    #[serde(default)]
    pub seed: u32,
}

fn one() -> f32 {
    1.0
}
fn white() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}
fn default_spacing() -> f32 {
    3.0
}
fn default_scale_range() -> [f32; 2] {
    [1.0, 1.0]
}
fn default_lane_clear() -> f32 {
    5.5
}

impl PropsJson {
    /// Every model id the block names, for validation.
    #[cfg(test)]
    pub fn models(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self
            .place
            .iter()
            .map(|p| p.model)
            .chain(self.lines.iter().flat_map(|l| l.models.iter().copied()))
            .chain(self.scatter.iter().flat_map(|s| s.models.iter().copied()))
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }
}

fn texture_pool(name: Option<&str>) -> &'static [u32] {
    match name {
        Some("wood") => TEX_WOOD,
        Some("leaves") => TEX_LEAVES,
        Some("flowers") => TEX_FLOWERS,
        _ => TEX_STONE,
    }
}

/// Where the props go, and what may not be covered.
pub struct PropWorld<'a> {
    pub cache: &'a Cache,
    pub map: &'a MapLayout,
    pub height: &'a HeightGrid,
    pub structures: &'a [Vec3],
    pub river_blocked: &'a [(i32, i32)],
    pub origin: (i32, i32),
    /// Per-tile river surface height and coverage (grid `size` x `size`).
    pub water_level: &'a [f32],
    pub water_mask: &'a [f32],
    pub size: usize,
}

impl PropWorld<'_> {
    /// Highest water surface within `radius` tiles of a world point.
    fn water_near(&self, x: f32, z: f32, radius: i32) -> Option<f32> {
        let cx = (x / 128.0).floor() as i32 - self.origin.0;
        let cy = (z / 128.0).floor() as i32 - self.origin.1;
        let mut best: Option<f32> = None;
        for gy in cy - radius..=cy + radius {
            for gx in cx - radius..=cx + radius {
                if gx < 0 || gy < 0 || gx as usize >= self.size || gy as usize >= self.size {
                    continue;
                }
                let idx = gy as usize * self.size + gx as usize;
                if self.water_mask.get(idx).is_some_and(|&m| m > 0.03) {
                    let level = self.water_level[idx];
                    best = Some(best.map_or(level, |b: f32| b.max(level)));
                }
            }
        }
        best
    }
}

/// The meshes and grids props are written into.
pub struct PropSink<'a> {
    pub model_cache: &'a mut HashMap<u32, Model>,
    pub model_mesh: &'a mut MeshData,
    pub shadow_mesh: &'a mut MeshData,
    pub shadow_caster_mesh: &'a mut MeshData,
    pub collision_grid: &'a mut Grid,
    pub sight_grid: &'a mut Grid,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PropCounts {
    pub placed: usize,
    pub lined: usize,
    pub scattered: usize,
}

struct Drop<'a> {
    model: u32,
    pos: Vec3,
    yaw: f32,
    scale: f32,
    block: i32,
    tint: [f32; 3],
    texture: Option<&'a str>,
    seed: u32,
}

/// Lay out every prop. Returns how many of each shape were placed.
pub fn place_props(
    props: &PropsJson,
    world: &PropWorld<'_>,
    sink: &mut PropSink<'_>,
) -> PropCounts {
    let river: HashSet<(i32, i32)> = world.river_blocked.iter().copied().collect();
    let keep_clear = world
        .map
        .castle
        .as_ref()
        .map(|c| c.keep_clear())
        .unwrap_or_default();
    let landmarks: Vec<Vec2> = props
        .place
        .iter()
        .map(|p| Vec2::new((p.at[0] + 0.5) * 128.0, (p.at[1] + 0.5) * 128.0))
        .collect();
    let open = |pos: Vec3, lane_clear: f32| -> bool {
        if !world.height.contains(pos.x, pos.z) {
            return false;
        }
        // Authored landmarks (statues, banners) keep a clear plaza.
        if landmarks
            .iter()
            .any(|l| l.distance(Vec2::new(pos.x, pos.z)) < 6.0 * 128.0)
        {
            return false;
        }
        if world.map.distance_to_lane(pos.x, pos.z) < lane_clear * 128.0 {
            return false;
        }
        if world
            .structures
            .iter()
            .any(|s| s.distance(pos) < 7.0 * 128.0)
        {
            return false;
        }
        // Jungle camps keep room to fight and leash.
        let flat = Vec2::new(pos.x, pos.z);
        if world
            .map
            .camps
            .iter()
            .any(|c| Vec2::new(c.pos.x, c.pos.z).distance(flat) < 5.0 * 128.0)
        {
            return false;
        }
        let tx = (pos.x / 128.0).floor() as i32;
        let ty = (pos.z / 128.0).floor() as i32;
        if world.map.in_biome(tx, ty) {
            return false;
        }
        let (fx, fz) = (pos.x / 128.0, pos.z / 128.0);
        if keep_clear
            .iter()
            .any(|r| fx >= r.0 && fx <= r.2 && fz >= r.1 && fz <= r.3)
        {
            return false;
        }
        !river.contains(&(tx - world.origin.0, ty - world.origin.1))
    };

    let mut counts = PropCounts::default();
    for (index, place) in props.place.iter().enumerate() {
        let pos = tile_centre(place.at[0], place.at[1]);
        if !world.height.contains(pos.x, pos.z) {
            continue;
        }
        let dropped = drop_prop(
            world,
            sink,
            Drop {
                model: place.model,
                pos,
                yaw: place.yaw.to_radians(),
                scale: place.scale,
                block: place.block,
                tint: place.tint,
                texture: place.texture.as_deref(),
                seed: index as u32 * 0x9E37_79B9,
            },
        );
        counts.placed += dropped as usize;
    }

    for (line_index, line) in props.lines.iter().enumerate() {
        if line.models.is_empty() || line.spacing <= 0.0 {
            continue;
        }
        let mut paths: Vec<Vec<Vec2>> = Vec::new();
        match line.lane_offset {
            Some(offset) => {
                for lane in &world.map.lanes {
                    let centre: Vec<Vec2> =
                        lane.waypoints.iter().map(|w| Vec2::new(w.x, w.z)).collect();
                    for side in [-1.0f32, 1.0] {
                        paths.push(offset_polyline(&centre, offset * 128.0 * side));
                    }
                }
            }
            None => paths.push(
                line.points
                    .iter()
                    .map(|p| Vec2::new((p[0] + 0.5) * 128.0, (p[1] + 0.5) * 128.0))
                    .collect(),
            ),
        }
        let mut slot = 0u32;
        for (path_index, path) in paths.iter().enumerate() {
            let base_seed = line
                .seed
                .wrapping_add(line_index as u32 * 0x85EB_CA6B)
                .wrapping_add(path_index as u32 * 0xC2B2_AE35);
            for (point, dir) in walk_polyline(path, line.spacing * 128.0) {
                slot += 1;
                let seed = base_seed ^ slot.wrapping_mul(0x27D4_EB2F);
                // Gaps come in runs of four slots so they read as openings.
                let run = base_seed ^ (slot / 4).wrapping_mul(0x1656_67B1);
                if hash01(run) < line.gaps {
                    continue;
                }
                let perp = Vec2::new(-dir.y, dir.x);
                let jitter = line.jitter * 128.0;
                let p = point
                    + perp * (hash01(seed ^ 0x51) - 0.5) * 2.0 * jitter
                    + dir * (hash01(seed ^ 0xA3) - 0.5) * jitter;
                let pos = Vec3::new(p.x, 0.0, p.y);
                if !open(pos, line.lane_clear) {
                    continue;
                }
                let model = line.models
                    [(hash01(seed ^ 0x77) * line.models.len() as f32) as usize % line.models.len()];
                let dropped = drop_prop(
                    world,
                    sink,
                    Drop {
                        model,
                        pos,
                        yaw: hash01(seed ^ 0x3C) * std::f32::consts::TAU,
                        scale: lerp(line.scale, hash01(seed ^ 0x19)),
                        block: line.block,
                        tint: line.tint,
                        texture: line.texture.as_deref(),
                        seed,
                    },
                );
                counts.lined += dropped as usize;
            }
        }
    }

    for (scatter_index, scatter) in props.scatter.iter().enumerate() {
        if scatter.models.is_empty() {
            continue;
        }
        let [x0, y0, x1, y1] = scatter.rect;
        for i in 0..scatter.count {
            let seed = scatter
                .seed
                .wrapping_add(scatter_index as u32 * 0x632B_E5AB)
                .wrapping_add(i as u32 * 0x9E37_79B9);
            let tx = x0 + (x1 - x0) * hash01(seed ^ 0x11);
            let ty = y0 + (y1 - y0) * hash01(seed ^ 0x22);
            let pos = Vec3::new(tx * 128.0, 0.0, ty * 128.0);
            if !open(pos, scatter.lane_clear) {
                continue;
            }
            let model = scatter.models[(hash01(seed ^ 0x33) * scatter.models.len() as f32)
                as usize
                % scatter.models.len()];
            let dropped = drop_prop(
                world,
                sink,
                Drop {
                    model,
                    pos,
                    yaw: hash01(seed ^ 0x44) * std::f32::consts::TAU,
                    scale: lerp(scatter.scale, hash01(seed ^ 0x55)),
                    block: scatter.block,
                    tint: scatter.tint,
                    texture: scatter.texture.as_deref(),
                    seed,
                },
            );
            counts.scattered += dropped as usize;
        }
    }
    counts
}

fn drop_prop(world: &PropWorld<'_>, sink: &mut PropSink<'_>, drop: Drop<'_>) -> bool {
    let Ok(model) = cached_model(world.cache, sink.model_cache, drop.model) else {
        tracing::warn!("props: model {} failed to load", drop.model);
        return false;
    };
    let base = Vec3::new(
        drop.pos.x,
        world.height.height_at(drop.pos.x, drop.pos.z),
        drop.pos.z,
    );
    append_model_scaled_tinted(
        sink.model_mesh,
        model,
        base,
        drop.yaw,
        drop.scale,
        drop.tint,
        if drop.texture == Some("plain") {
            super::TEX_PLAIN
        } else {
            super::pick_texture(texture_pool(drop.texture), drop.seed)
        },
        false,
    );
    let (lo, hi) = model.bounds();
    tracing::debug!(
        "prop {} at {:.0},{:.0} y {:.0} bounds {:?} {:?} mesh {}",
        drop.model,
        base.x / 128.0,
        base.z / 128.0,
        base.y,
        lo,
        hi,
        sink.model_mesh.vertices.len()
    );
    let footprint = ((hi[0] - lo[0]).max(hi[2] - lo[2]).max(64) as f32) * drop.scale;
    if hi[1] - lo[1] > 40 {
        append_model_caster(
            sink.shadow_caster_mesh,
            model,
            base,
            drop.yaw,
            drop.scale,
            CasterStyle::Building,
        );
    }
    push_blob_quad(
        sink.shadow_mesh,
        base,
        (footprint * 0.6).clamp(60.0, 420.0),
        0.2,
    );
    if drop.block > 0 {
        let gx = (drop.pos.x / 128.0).floor() as i32 - world.origin.0;
        let gy = (drop.pos.z / 128.0).floor() as i32 - world.origin.1;
        let half = drop.block / 2;
        sink.collision_grid
            .block_rect(gx - half, gy - half, drop.block, drop.block);
        sink.sight_grid
            .block_rect(gx - half, gy - half, drop.block, drop.block);
    }
    true
}

/// Lay the bridge model in rows across every river crossing, lined up with
/// the lane. Planks are walkable: they never touch collision.
pub fn place_bridge_planks(
    bridge: super::BridgeModelJson,
    world: &PropWorld<'_>,
    sink: &mut PropSink<'_>,
) -> usize {
    let Some(river) = world.map.river.as_ref() else {
        return 0;
    };
    let Ok(model) = cached_model(world.cache, sink.model_cache, bridge.model) else {
        tracing::warn!("bridge model {} failed to load", bridge.model);
        return 0;
    };
    // Cache bridge pieces are authored at their plane height (699's deck
    // top is 961 units up, legs down to 810): bring the top face to just
    // over the ground so heroes walk on the planks.
    let (lo, _) = model.bounds();
    let drop_to_deck = lo[1] as f32 * bridge.scale;
    let mut count = 0;
    for &(tx, ty) in &river.bridges {
        let bx = tx * 128.0;
        let bz = ty * 128.0;
        let Some((dx, dz)) = super::nearest_lane_tangent(&world.map.lanes, bx, bz) else {
            continue;
        };
        let along = Vec2::new(dx, dz);
        let across = Vec2::new(-dz, dx);
        let yaw = dx.atan2(dz) + bridge.yaw.to_radians();
        let half_length = super::BRIDGE_HALF_LENGTH;
        // The crossing's bed is cut below the banks; the deck spans just
        // over the surrounding water, so it never drowns.
        let ground = world.height.height_at(bx, bz) + 8.0;
        let deck_y = world
            .water_near(bx, bz, 6)
            .map_or(ground, |water| ground.max(water + 5.0));
        let step = (bridge.spacing * 128.0).max(32.0);
        let rows = ((half_length * 2.0) / step).floor() as i32;
        let width = super::BRIDGE_HALF_WIDTH * 2.0;
        for row in 0..=rows {
            let t = -half_length + row as f32 * step;
            for column in 0..bridge.across {
                let c = (column as f32 + 0.5) / bridge.across as f32 - 0.5;
                let p = Vec2::new(bx, bz) + along * t + across * (c * width);
                append_model_scaled_tinted(
                    sink.model_mesh,
                    model,
                    Vec3::new(p.x, deck_y + drop_to_deck, p.y),
                    yaw,
                    bridge.scale,
                    [1.0, 1.0, 1.0],
                    super::TEX_PLAIN,
                    false,
                );
                count += 1;
            }
        }
    }
    count
}

fn tile_centre(x: f32, y: f32) -> Vec3 {
    Vec3::new((x + 0.5) * 128.0, 0.0, (y + 0.5) * 128.0)
}

fn lerp(range: [f32; 2], t: f32) -> f32 {
    range[0] + (range[1] - range[0]) * t
}

/// Shift a polyline sideways by `offset` (positive = left of travel).
pub fn offset_polyline(points: &[Vec2], offset: f32) -> Vec<Vec2> {
    let n = points.len();
    (0..n)
        .map(|i| {
            let prev = points[i.saturating_sub(1)];
            let next = points[(i + 1).min(n - 1)];
            let dir = (next - prev).normalize_or_zero();
            points[i] + Vec2::new(-dir.y, dir.x) * offset
        })
        .collect()
}

/// Points every `spacing` along a polyline, with the travel direction.
pub fn walk_polyline(points: &[Vec2], spacing: f32) -> Vec<(Vec2, Vec2)> {
    let mut out = Vec::new();
    let mut carry = 0.0;
    for pair in points.windows(2) {
        let delta = pair[1] - pair[0];
        let length = delta.length();
        if length < 1e-3 {
            continue;
        }
        let dir = delta / length;
        let mut t = carry;
        while t < length {
            out.push((pair[0] + dir * t, dir));
            t += spacing;
        }
        carry = t - length;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walk_polyline_spaces_points_across_corners() {
        let path = [
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
        ];
        let points = walk_polyline(&path, 4.0);
        let xs: Vec<(f32, f32)> = points.iter().map(|(p, _)| (p.x, p.y)).collect();
        assert_eq!(
            xs,
            vec![(0.0, 0.0), (4.0, 0.0), (8.0, 0.0), (10.0, 2.0), (10.0, 6.0)]
        );
    }

    #[test]
    fn offset_polyline_moves_left_of_travel() {
        let path = [Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0)];
        let left = offset_polyline(&path, 2.0);
        assert_eq!(left, vec![Vec2::new(0.0, 2.0), Vec2::new(10.0, 2.0)]);
    }

    #[test]
    fn shipped_map_dresses_bases_bridges_cliffs_and_jungle() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let text = std::fs::read_to_string(root.join("data/maps/karamja_rift/map.json"))
            .expect("map.json");
        let json: serde_json::Value = serde_json::from_str(&text).expect("map json");
        let props: PropsJson = serde_json::from_value(json["props"].clone()).expect("props block");
        // Both teams' god statues, cliff lines with blocking footprints, and
        // non-blocking flora.
        for statue in [9146, 1400] {
            assert!(props.place.iter().any(|p| p.model == statue), "{statue}");
        }
        assert!(props.lines.iter().all(|l| l.block > 0 && l.gaps > 0.0));
        assert!(props.scatter.iter().all(|s| s.block == 0 && s.count > 0));
        let bridge: crate::scene::BridgeModelJson =
            serde_json::from_value(json["bridge_model"].clone()).expect("bridge model");
        assert_eq!(bridge.model, 699);

        // Every model id exists in the pinned cache, when it is present.
        let cache_path = root.join(".cache/osrs-235/cache");
        if !cache_path.join("main_file_cache.dat2").exists() {
            return;
        }
        let cache = Cache::open(&cache_path).expect("the pinned cache opens");
        for id in props.models().into_iter().chain([bridge.model]) {
            let group = cache.group(7, id).unwrap_or_else(|_| panic!("model {id}"));
            Model::load(id, &group.data).unwrap_or_else(|_| panic!("model {id} decodes"));
        }
    }

    #[test]
    fn props_block_parses_with_defaults() {
        let props: PropsJson = serde_json::from_str(
            r#"{
                "place": [{ "model": 9146, "at": [10, 20] }],
                "lines": [{ "models": [19997], "lane_offset": 8 }],
                "scatter": [{ "models": [12932, 1685], "rect": [0, 0, 5, 5], "count": 3 }]
            }"#,
        )
        .unwrap();
        assert_eq!(props.place[0].scale, 1.0);
        assert_eq!(props.place[0].block, 0);
        assert_eq!(props.lines[0].spacing, 3.0);
        assert_eq!(props.scatter[0].lane_clear, 5.5);
        assert_eq!(props.models(), vec![1685, 9146, 12932, 19997]);
    }
}
