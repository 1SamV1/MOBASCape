use std::sync::Arc;

use glam::Vec2;

use crate::animated::{AnimatedScene, HitsplatKind, UnitKind};
use crate::font;
use crate::hero_select::{self, HeroSelect};
use crate::items::{
    CATEGORY_LABELS, ItemConfig, ItemShop, shop_button_rect, shop_buy_rect, shop_card_rect,
    shop_category_rect, shop_close_rect, shop_detail_rect, shop_sell_rect, shop_window_rect,
};
use crate::scene::CampBuff;
use crate::scene::srgb_to_linear;
use crate::ui_assets::UiAtlas;

mod screens;

/// OSRS interface colours. The RuneScape client draws all of its text with a
/// crisp pixel font and a one pixel black shadow in these palette colours.
pub const OSRS_WHITE: [u8; 3] = [255, 255, 255];
pub const OSRS_YELLOW: [u8; 3] = [255, 255, 0];
pub const OSRS_GOLD: [u8; 3] = [255, 196, 0];

/// The HUD text is drawn at this multiple of the authored sizes so it stays
/// readable on a 720p window.
const TEXT_SCALE: f32 = 1.16;

/// Shared HUD type scale. Cards use one title size, one body size and one
/// caption size so the interface reads as a system rather than a pile of
/// one-off labels.
const HUD_TITLE_SCALE: f32 = 0.98;
const HUD_BODY_SCALE: f32 = 0.82;
const HUD_SMALL_SCALE: f32 = 0.70;

/// Extra multiplier for the baked OSRS interface font. The atlas glyphs are
/// authored at 16px, so this maps the authored HUD scale to whole pixels for
/// crisp nearest-filtered text.
const FONT_SCALE: f32 = 1.0 / TEXT_SCALE;

/// Cache hitmarks are 24x23. They are drawn at 2x so the pixel art stays
/// crisp and the splat reads at the default camera distance.
const HITMARK_SCALE: f32 = 2.0;
const HITMARK_GAP: f32 = 4.0;

/// Information about the unit under the cursor, shown as a Dota style
/// tooltip next to the mouse.
pub struct HoverInfo {
    pub name: String,
    pub level: u32,
    pub health: f32,
    pub max_health: f32,
    pub team: u8,
    pub kind: UnitKind,
    pub description: &'static str,
}

#[derive(Clone)]
pub struct SpectatorInfo {
    pub name: String,
    pub level: u32,
    pub health: f32,
    pub max_health: f32,
    pub mana: f32,
    pub max_mana: f32,
    pub team: u8,
}

/// A screen-space unit plate projected from the world by the client.
#[derive(Clone)]
pub struct UnitFrame {
    pub x: f32,
    pub y: f32,
    pub name: String,
    pub level: u32,
    pub health: f32,
    pub max_health: f32,
    pub mana: f32,
    pub max_mana: f32,
    pub team: u8,
    pub hero: bool,
    /// Neutral jungle creature; gets its own gold/buff name colour.
    pub camp: bool,
    pub structure: bool,
    /// A solid overhead-name colour chosen from team and creature type.
    pub name_color: [u8; 3],
    /// Overhead names are larger for champions than for jungle creatures.
    pub name_scale: f32,
    /// Player-killer skull over the name.
    pub pk: bool,
    /// A short warning line such as "NO SAFE" between the skull and name.
    pub tag: Option<String>,
    pub width: f32,
}

/// A projected OSRS hitmark, ready for the HUD to draw.
#[derive(Clone)]
pub struct Hitsplat {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub kind: HitsplatKind,
    /// Fades out over the last fraction of a second.
    pub alpha: f32,
    /// Column slot when several hitmarks share one unit.
    pub stack: u8,
    /// Cool tint for another champion's hit; `None` keeps the cache colour.
    pub tint: Option<[u8; 3]>,
}

/// Solid overhead-name colours: team heroes are blue/red, jungle camps take
/// their buff colour, and structures stay neutral bone white.
pub fn unit_name_color(team: u8, hero: bool, camp: bool, buff: Option<CampBuff>) -> [u8; 3] {
    if hero {
        return if team == 0 {
            [125, 195, 255]
        } else {
            [255, 105, 95]
        };
    }
    if camp {
        return match buff {
            Some(CampBuff::Might) => [255, 150, 80],
            Some(CampBuff::Swift) => [120, 235, 160],
            None => [245, 205, 85],
        };
    }
    [225, 220, 205]
}

/// How far a plate reaches above its anchor, for the overlap nudge.
fn plate_extent(frame: &UnitFrame) -> f32 {
    let mut extent = 24.0 + frame.name_scale.max(1.0) * 12.0;
    if frame.tag.is_some() {
        extent += 16.0;
    }
    if frame.pk {
        extent += 29.0;
    }
    extent
}

/// Sort projected plates and push overlapping ones upward. Both the legacy
/// HUD and the assembled `--ui-live` interface use this, so a champion and a
/// jungle camp at the same spot never print their names on top of each other.
pub fn nudge_unit_frames(mut frames: Vec<UnitFrame>) -> Vec<UnitFrame> {
    frames.sort_by(|a, b| {
        (a.team != 0)
            .cmp(&(b.team != 0))
            .then(a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });
    for i in 1..frames.len() {
        let mut y = frames[i].y;
        let x = frames[i].x;
        let w = frames[i].width;
        let extent = plate_extent(&frames[i]);
        for _ in 0..3 {
            let mut moved = false;
            for other in &frames[..i] {
                let dx = (x - other.x).abs();
                let dy = (y - other.y).abs();
                let needed = (extent + plate_extent(other)) * 0.5;
                if dx < (w + other.width) * 0.5 && dy < needed {
                    y -= extent * 0.8;
                    moved = true;
                }
            }
            if !moved {
                break;
            }
        }
        frames[i].y = y;
    }
    frames
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

pub struct PlayerHud {
    pub health: f32,
    pub max_health: f32,
    pub mana: f32,
    pub max_mana: f32,
    pub xp: f32,
    pub xp_next: f32,
    pub cooldowns: [f32; 4],
    /// Maximum cooldown per slot, so the HUD fill bar matches the kit.
    pub cooldown_maxima: [f32; 4],
    pub armor: f32,
    pub attack_damage: f32,
    /// Ranged level for the OSRS-style stats panel.
    pub ranged: f32,
    /// Attack interval, shown as Run-Attack.
    pub attack_interval: f32,
    /// Ability power, shown as Ranged-Bonus.
    pub ability_power: f32,
    /// Both teams' champions with live health for the roster panel.
    pub roster: Vec<HeroMarker>,
    pub shield: f32,
    pub shield_timer: f32,
    pub buff: Option<(f32, f32, [u8; 3])>,
    pub buff_label: Option<&'static str>,
    pub level: u32,
    pub gold: u32,
    pub shop_open: bool,
    pub shop_hover: Option<usize>,
    pub shop_selected: Option<usize>,
    pub champion_name: String,
    pub minions: usize,
    pub visible_enemies: usize,
    pub towers: (usize, usize),
    pub inhibitors: (usize, usize),
    pub items_owned: usize,
    pub events: Vec<String>,
    /// Team scoreboard: kills and deaths per team.
    pub kills: (u32, u32),
    pub deaths: (u32, u32),
    /// The local player's own scoreboard line.
    pub player_kills: u32,
    pub player_deaths: u32,
    pub player_assists: u32,
    pub player_cs: u32,
    /// Show the engineer stat sheet and visibility counters.
    pub debug_hud: bool,
    /// Seconds since the match started.
    pub match_time: f32,
    /// Unit under the cursor.
    pub hover: Option<HoverInfo>,
    /// Name of the biome pocket the hero is standing in.
    pub biome: Option<String>,
    /// Team heroes for the minimap and the hero profile card.
    pub heroes: Vec<HeroMarker>,
    /// Screen-space floating labels: `(x, y, text, colour, scale)`.
    pub damage_numbers: Vec<(f32, f32, String, [u8; 3], f32)>,
    /// Screen-space OSRS hitmarks for combat damage, status ticks and heals.
    pub hitsplats: Vec<Hitsplat>,
    /// Screen-space name/health plates for champions, structures and camps.
    pub unit_frames: Vec<UnitFrame>,
    /// 0..1 progress of a committed player attack, if one is winding up.
    pub cast_progress: Option<f32>,
    /// Red screen-edge flash while the player is taking damage.
    pub damage_flash: f32,
    pub spectator: Option<SpectatorInfo>,
    /// Seconds until the local player respawns, when dead.
    pub respawn_remaining: Option<f32>,
    /// Centre-screen announcement, e.g. "YOU HAVE SLAIN A MINION!".
    pub announcement: Option<String>,
    /// Recent kill messages shown under the roster.
    pub kill_feed: Vec<String>,
    /// Number of purchases waiting in the storage chest.
    pub delivery_pending: usize,
    /// 0 = idle, 1 = flying to the hero, 2 = returning.
    pub delivery_state: u8,
    /// Item ids currently in transit.
    pub pending_ids: Vec<String>,
}

/// One team hero shown on the minimap and in the hero profile card.
#[derive(Clone)]
pub struct HeroMarker {
    pub name: String,
    pub team: u8,
    pub pos: (f32, f32),
    pub alive: bool,
    pub health: f32,
    pub max_health: f32,
    pub mana: f32,
    pub max_mana: f32,
    pub level: u32,
}

pub struct Hud {
    pub vertices: Vec<UiVertex>,
    pub indices: Vec<u32>,
    pub atlas: Arc<UiAtlas>,
}

/// Format seconds as `M:SS`, like the match clock in the scoreboard.
pub fn format_clock(seconds: f32) -> String {
    let total = seconds.max(0.0) as u32;
    format!("{}:{:02}", total / 60, total % 60)
}

/// Screen-space rectangle of the minimap: `(x, y, width, height)`.
pub fn minimap_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let size = 190.0;
    (width - size - 18.0, height - size - 18.0, size, size)
}

/// Convert a cursor position over the minimap into world `(x, z)`.
/// Returns `None` when the cursor is outside the minimap.
pub fn minimap_world_at(
    width: f32,
    height: f32,
    cursor: (f64, f64),
    bounds: (Vec2, Vec2),
) -> Option<(f32, f32)> {
    let (x, y, size, _) = minimap_rect(width, height);
    let mx = cursor.0 as f32;
    let my = cursor.1 as f32;
    if mx < x || mx > x + size || my < y || my > y + size {
        return None;
    }
    let nx = ((mx - x) / size).clamp(0.0, 1.0);
    let nz = 1.0 - ((my - y) / size).clamp(0.0, 1.0);
    Some((
        bounds.0.x + nx * (bounds.1.x - bounds.0.x),
        bounds.0.y + nz * (bounds.1.y - bounds.0.y),
    ))
}

impl Hud {
    pub fn new(atlas: Arc<UiAtlas>) -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            atlas,
        }
    }

    /// Draw text with the default interface font and a drop shadow.
    #[allow(clippy::too_many_arguments)]
    pub fn text(
        &mut self,
        width: f32,
        height: f32,
        x: f32,
        y: f32,
        scale: f32,
        text: &str,
        color: [u8; 3],
    ) {
        self.text_font(None, width, height, x, y, scale, text, color);
    }

    /// Draw text with a named cache font (for example `p12` for overhead
    /// names or `b12` for hitmark numbers), with a drop shadow.
    #[allow(clippy::too_many_arguments)]
    pub fn text_named(
        &mut self,
        font_name: &str,
        width: f32,
        height: f32,
        x: f32,
        y: f32,
        scale: f32,
        text: &str,
        color: [u8; 3],
    ) {
        self.text_font(Some(font_name), width, height, x, y, scale, text, color);
    }

    /// Draw one pass of a named font without a shadow. The hitmark renderer
    /// uses this for the black offset pass and the white number pass so both
    /// can fade together.
    #[allow(clippy::too_many_arguments)]
    pub fn text_named_raw(
        &mut self,
        font_name: &str,
        width: f32,
        height: f32,
        x: f32,
        y: f32,
        scale: f32,
        text: &str,
        color: [u8; 3],
        alpha: f32,
    ) {
        self.text_raw(
            Some(font_name),
            width,
            height,
            x,
            y,
            scale,
            text,
            color,
            alpha,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn text_font(
        &mut self,
        font_name: Option<&str>,
        width: f32,
        height: f32,
        x: f32,
        y: f32,
        scale: f32,
        text: &str,
        color: [u8; 3],
    ) {
        let shadow = (scale * TEXT_SCALE * FONT_SCALE).round().max(1.0);
        self.text_raw(
            font_name,
            width,
            height,
            x + shadow,
            y + shadow,
            scale,
            text,
            [0, 0, 0],
            1.0,
        );
        self.text_raw(font_name, width, height, x, y, scale, text, color, 1.0);
    }

    #[allow(clippy::too_many_arguments)]
    fn text_raw(
        &mut self,
        font_name: Option<&str>,
        width: f32,
        height: f32,
        x: f32,
        y: f32,
        scale: f32,
        text: &str,
        color: [u8; 3],
        alpha: f32,
    ) {
        let quads = {
            let atlas = match font_name {
                Some(name) => self.atlas.font_named(name),
                None => self.atlas.font(),
            };
            atlas.map(|atlas| {
                // Real RuneScape interface glyphs: one textured quad per
                // glyph. Pixel glyphs must land on whole pixels: non-integer
                // nearest scaling drops rows/columns and breaks letters.
                let scale = (scale * TEXT_SCALE * FONT_SCALE).round().max(1.0);
                let mut quads = Vec::with_capacity(text.len());
                let mut cx = x;
                for ch in text.chars() {
                    let Some(glyph) = atlas.glyph(ch) else {
                        cx += atlas.size * 0.5 * scale;
                        continue;
                    };
                    if ch != ' ' && glyph.w > 0.0 && glyph.h > 0.0 {
                        quads.push((
                            (cx + glyph.left * scale).round(),
                            (y + glyph.top * scale).round(),
                            glyph.w * scale,
                            glyph.h * scale,
                            glyph.uv,
                        ));
                    }
                    cx += glyph.advance * scale;
                }
                quads
            })
        };
        if let Some(quads) = quads {
            for (qx, qy, qw, qh, uv) in quads {
                self.sprite(width, height, qx, qy, qw, qh, uv, color, alpha);
            }
            return;
        }

        // Fallback: the built-in 5x7 pixel font.
        let scale = scale * TEXT_SCALE;
        let mut cx = x;
        for ch in text.chars() {
            let rows = font::glyph(ch);
            for (row, bits) in rows.iter().enumerate() {
                let mut col = 0;
                while col < font::GLYPH_W {
                    if bits & (1 << (font::GLYPH_W - 1 - col)) == 0 {
                        col += 1;
                        continue;
                    }
                    let start = col;
                    while col < font::GLYPH_W && bits & (1 << (font::GLYPH_W - 1 - col)) != 0 {
                        col += 1;
                    }
                    self.quad_alpha(
                        width,
                        height,
                        cx + start as f32 * scale,
                        y + row as f32 * scale,
                        (col - start) as f32 * scale,
                        scale,
                        color,
                        alpha,
                    );
                }
            }
            cx += font::ADVANCE as f32 * scale;
        }
    }

    /// Width of a string in screen pixels for the default font.
    pub fn text_width(&self, text: &str, scale: f32) -> f32 {
        if let Some(atlas) = self.atlas.font() {
            let snapped = (scale * TEXT_SCALE * FONT_SCALE).round().max(1.0);
            atlas.measure(text, snapped)
        } else {
            font::text_width(text, scale * TEXT_SCALE)
        }
    }

    /// Width of a string in screen pixels for a named cache font.
    pub fn text_width_named(&self, font_name: &str, text: &str, scale: f32) -> f32 {
        if let Some(atlas) = self.atlas.font_named(font_name) {
            let snapped = (scale * TEXT_SCALE * FONT_SCALE).round().max(1.0);
            atlas.measure(text, snapped)
        } else {
            font::text_width(text, scale * TEXT_SCALE)
        }
    }

    /// Top-to-baseline height of a named font, in pixels.
    pub fn font_ascent_named(&self, font_name: &str) -> f32 {
        self.atlas
            .font_named(font_name)
            .map_or(font::GLYPH_H as f32, |font| font.size)
    }

    /// Visual centre of a digit, measured from the top of the text line.
    /// Used to centre hitmark numbers inside the splat.
    pub fn font_digit_center_named(&self, font_name: &str) -> f32 {
        self.atlas
            .font_named(font_name)
            .and_then(|font| font.glyph('0'))
            .map_or(font::GLYPH_H as f32 * 0.5, |glyph| {
                glyph.top + glyph.h * 0.5
            })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn text_centered(
        &mut self,
        width: f32,
        height: f32,
        center_x: f32,
        y: f32,
        scale: f32,
        text: &str,
        color: [u8; 3],
    ) {
        let text_width = self.text_width(text, scale);
        self.text(
            width,
            height,
            center_x - text_width * 0.5,
            y,
            scale,
            text,
            color,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn text_centered_named(
        &mut self,
        font_name: &str,
        width: f32,
        height: f32,
        center_x: f32,
        y: f32,
        scale: f32,
        text: &str,
        color: [u8; 3],
    ) {
        let text_width = self.text_width_named(font_name, text, scale);
        self.text_named(
            font_name,
            width,
            height,
            center_x - text_width * 0.5,
            y,
            scale,
            text,
            color,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn build_minimap(
        &mut self,
        width: f32,
        height: f32,
        scene: &AnimatedScene,
        camera_pos: (f32, f32),
        camera_distance: f32,
        camera_pitch: f32,
        heroes: &[HeroMarker],
    ) {
        let (x, y, size, _) = minimap_rect(width, height);
        self.quad(
            width,
            height,
            x - 3.0,
            y - 3.0,
            size + 6.0,
            size + 6.0,
            [70, 75, 85],
        );
        self.quad(
            width,
            height,
            x - 1.0,
            y - 1.0,
            size + 2.0,
            size + 2.0,
            [16, 16, 20],
        );

        // The baked terrain map covers the whole rift; enemy positions are
        // still hidden by only drawing units that are currently visible.
        if let Some(uv) = self.atlas.minimap_uv() {
            self.sprite(width, height, x, y, size, size, uv, [255, 255, 255], 1.0);
        }

        // Level/XP/gold strip along the bottom edge of the minimap, matching
        // the reference client's compact economy readout.
        {
            let stats = scene.player_stats();
            let strip_h = 16.0;
            let sy = y + size - strip_h;
            self.quad(width, height, x, sy, size, strip_h, [10, 10, 14]);
            self.quad(
                width,
                height,
                x + 4.0,
                sy + 11.0,
                size - 8.0,
                4.0,
                [34, 28, 44],
            );
            self.quad(
                width,
                height,
                x + 4.0,
                sy + 11.0,
                (size - 8.0) * (stats.xp / stats.xp_next.max(1.0)).clamp(0.0, 1.0),
                4.0,
                [160, 96, 220],
            );
            self.text(
                width,
                height,
                x + 5.0,
                sy + 1.0,
                0.75,
                &format!("LV {}", stats.level),
                [235, 225, 150],
            );
            let gold = format!("{}G", stats.gold);
            let gw = self.text_width(&gold, 0.75);
            self.text(
                width,
                height,
                x + size - gw - 5.0,
                sy + 1.0,
                0.75,
                &gold,
                OSRS_GOLD,
            );
        }

        let (min, max) = scene.minimap_bounds;
        let to_map = |p: glam::Vec3| {
            // North (small z) is up, matching the camera's view direction.
            let nx = ((p.x - min.x) / (max.x - min.x)).clamp(0.0, 1.0);
            let nz = ((p.z - min.y) / (max.y - min.y)).clamp(0.0, 1.0);
            (x + nx * size, y + nz * size)
        };

        // Team heroes: a white ringed marker so you can see who is where.
        for hero in heroes {
            if hero.team != 0 || !hero.alive {
                continue;
            }
            let (hx, hy) = to_map(glam::Vec3::new(hero.pos.0, 0.0, hero.pos.1));
            self.quad(width, height, hx - 5.0, hy - 5.0, 10.0, 10.0, [10, 10, 12]);
            self.quad(width, height, hx - 4.0, hy - 4.0, 8.0, 8.0, [250, 250, 250]);
            self.quad(width, height, hx - 2.5, hy - 2.5, 5.0, 5.0, [70, 210, 120]);
        }

        // Ping marker on the last enemy the player hit.
        if let Some((pos, _timer)) = &scene.ping {
            let (px, py) = to_map(*pos);
            self.quad(width, height, px - 1.5, py - 7.0, 3.0, 14.0, [235, 70, 60]);
            self.quad(width, height, px - 7.0, py - 1.5, 14.0, 3.0, [235, 70, 60]);
        }

        // Camera viewport rectangle, Dota minimap style. Approximate the
        // ground footprint of the 50 degree perspective camera so the box
        // matches what the player can actually see.
        {
            let half_w = camera_distance * 0.466 * (width / height.max(1.0));
            let half_h = camera_distance * 0.466 / camera_pitch.sin().max(0.2);
            let corners = [
                glam::Vec3::new(camera_pos.0 - half_w, 0.0, camera_pos.1 - half_h),
                glam::Vec3::new(camera_pos.0 + half_w, 0.0, camera_pos.1 + half_h),
            ];
            let (cx0, cy0) = to_map(corners[0]);
            let (cx1, cy1) = to_map(corners[1]);
            let rx = (cx1 - cx0).abs();
            let ry = (cy1 - cy0).abs();
            let px = cx0.min(cx1);
            let py = cy0.min(cy1);
            for (ox, oy, w, h) in [
                (px, py, rx, 1.0),
                (px, py + ry, rx, 1.0),
                (px, py, 1.0, ry),
                (px + rx, py, 1.0, ry),
            ] {
                self.quad(width, height, ox, oy, w, h, [235, 235, 235]);
            }
        }

        for unit in &scene.units {
            if !unit.visible {
                continue;
            }
            let (ux, uy) = to_map(unit.pos);
            if unit.health <= 0.0 {
                if unit.kind.is_structure() {
                    self.quad(width, height, ux - 2.5, uy - 2.5, 5.0, 5.0, [45, 45, 48]);
                }
                continue;
            }
            match unit.kind {
                UnitKind::Nexus => {
                    let color = if unit.team == 0 {
                        [80, 150, 255]
                    } else {
                        [220, 65, 60]
                    };
                    self.quad(width, height, ux - 4.0, uy - 4.0, 8.0, 8.0, color);
                }
                UnitKind::Inhibitor => {
                    let color = if unit.team == 0 {
                        [80, 150, 255]
                    } else {
                        [220, 65, 60]
                    };
                    self.quad(width, height, ux - 3.0, uy - 3.0, 6.0, 6.0, color);
                }
                UnitKind::Tower => {
                    let color = if unit.team == 0 {
                        [80, 150, 255]
                    } else {
                        [220, 65, 60]
                    };
                    self.quad(width, height, ux - 2.0, uy - 2.0, 4.0, 4.0, color);
                }
                UnitKind::Camp => {
                    self.quad(width, height, ux - 2.0, uy - 2.0, 4.0, 4.0, [230, 180, 70]);
                }
                UnitKind::Player => {
                    self.quad(width, height, ux - 3.0, uy - 3.0, 6.0, 6.0, [235, 235, 235]);
                }
                UnitKind::MeleeMinion | UnitKind::RangedMinion => {
                    let color = if unit.team == 0 {
                        [90, 160, 255]
                    } else {
                        [235, 90, 80]
                    };
                    if unit.super_minion {
                        self.quad(width, height, ux - 2.5, uy - 2.5, 5.0, 5.0, [255, 215, 90]);
                        self.quad(width, height, ux - 1.0, uy - 1.0, 2.0, 2.0, color);
                    } else {
                        self.quad(width, height, ux - 1.0, uy - 1.0, 2.0, 2.0, color);
                    }
                }
                UnitKind::Dummy => {
                    self.quad(width, height, ux - 1.5, uy - 1.5, 3.0, 3.0, [220, 180, 70]);
                }
                UnitKind::Watchtower => {
                    let color = if unit.team == 0 {
                        [120, 200, 255]
                    } else {
                        [255, 150, 120]
                    };
                    self.quad(width, height, ux - 2.0, uy - 2.0, 4.0, 4.0, color);
                }
                UnitKind::StorageChest => {
                    self.quad(width, height, ux - 2.0, uy - 2.0, 4.0, 4.0, [255, 205, 80]);
                }
                UnitKind::DeliveryGuard => {
                    self.quad(width, height, ux - 2.0, uy - 2.0, 4.0, 4.0, [250, 240, 180]);
                }
                UnitKind::Ambient => {}
                UnitKind::EnemyChampion => {
                    self.quad(width, height, ux - 3.0, uy - 3.0, 6.0, 6.0, [235, 90, 80]);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn build(
        &mut self,
        width: f32,
        height: f32,
        player: &PlayerHud,
        shop: &ItemShop,
        nexus0: (f32, f32),
        nexus1: (f32, f32),
        wave_timer: f32,
        wave_interval: f32,
        match_over: Option<u8>,
        mouse: (f64, f64),
    ) {
        self.vertices.clear();
        self.indices.clear();
        let spectating = player.spectator.is_some();
        let dead = player.respawn_remaining.is_some();
        let hide_player_hud = spectating || dead;
        let mouse_x = mouse.0 as f32;
        let mouse_y = mouse.1 as f32;

        // Scoreboard strip. The reference client keeps only the match
        // essentials on the top edge: team kills, clock, personal K/D/A and
        // creep score. Everything else is contextual.
        let strip_w = 760.0;
        let strip_x = (width - strip_w) * 0.5;
        self.hud_panel(width, height, strip_x, 0.0, strip_w, 34.0);
        self.quad(
            width,
            height,
            strip_x + 8.0,
            33.0,
            strip_w - 16.0,
            1.0,
            [86, 74, 50],
        );

        self.icon(
            width,
            height,
            strip_x + 12.0,
            8.0,
            18.0,
            "icon_swords",
            [255, 255, 255],
        );
        self.text(
            width,
            height,
            strip_x + 36.0,
            10.0,
            1.2,
            &format!("BLUE {}", player.kills.0),
            [110, 175, 255],
        );

        self.text_centered(
            width,
            height,
            strip_x + 330.0,
            9.0,
            1.45,
            &format_clock(player.match_time),
            [245, 225, 120],
        );

        self.icon(
            width,
            height,
            strip_x + 470.0,
            8.0,
            18.0,
            "icon_skull",
            [255, 255, 255],
        );
        self.text(
            width,
            height,
            strip_x + 494.0,
            10.0,
            1.2,
            &format!("RED {}", player.kills.1),
            [255, 120, 110],
        );

        self.text(
            width,
            height,
            strip_x + 604.0,
            11.0,
            1.0,
            &format!(
                "KDA {}/{}/{}",
                player.player_kills, player.player_deaths, player.player_assists
            ),
            [235, 235, 235],
        );
        self.text(
            width,
            height,
            strip_x + 706.0,
            11.0,
            1.0,
            &format!("CS {}", player.player_cs),
            [235, 225, 150],
        );

        if let Some(spectator) = &player.spectator {
            let bar_w = 560.0;
            let bar_x = (width - bar_w) * 0.5;
            self.hud_panel(width, height, bar_x, 38.0, bar_w, 54.0);
            let team = if spectator.team == 0 {
                [110, 185, 255]
            } else {
                [255, 120, 110]
            };
            self.text(
                width,
                height,
                bar_x + 10.0,
                42.0,
                0.8,
                &format!(
                    "SPECTATING {}  ·  LV {}  ·  {}  ·  Z/X SWITCH  ·  V EXIT",
                    spectator.name.to_uppercase(),
                    spectator.level,
                    if spectator.team == 0 { "BLUE" } else { "RED" }
                ),
                team,
            );
            self.quad(width, height, bar_x + 10.0, 59.0, 300.0, 6.0, [16, 16, 20]);
            self.quad(
                width,
                height,
                bar_x + 10.0,
                59.0,
                300.0 * (spectator.health / spectator.max_health.max(1.0)).clamp(0.0, 1.0),
                6.0,
                [70, 200, 105],
            );
            self.quad(width, height, bar_x + 10.0, 68.0, 300.0, 4.0, [16, 16, 22]);
            self.quad(
                width,
                height,
                bar_x + 10.0,
                68.0,
                300.0 * (spectator.mana / spectator.max_mana.max(1.0)).clamp(0.0, 1.0),
                4.0,
                [70, 120, 235],
            );
            self.text(
                width,
                height,
                bar_x + 320.0,
                58.0,
                0.70,
                &format!(
                    "HP {:.0}/{:.0}   MP {:.0}/{:.0}",
                    spectator.health, spectator.max_health, spectator.mana, spectator.max_mana
                ),
                [220, 220, 215],
            );
        }

        if !hide_player_hud {
            // Hero profile card above the minimap: portrait, name, level and
            // health so you can see who the marker on the map is.
            if let Some(hero) = player.heroes.first() {
                let (mx, my, msize, _) = minimap_rect(width, height);
                let card_h = 58.0;
                let card_y = (my - card_h - 8.0).max(120.0);
                self.hud_panel(width, height, mx, card_y, msize, card_h);
                let portrait = 42.0;
                self.quad(
                    width,
                    height,
                    mx + 8.0,
                    card_y + 8.0,
                    portrait,
                    portrait,
                    [70, 64, 44],
                );
                self.quad(
                    width,
                    height,
                    mx + 10.0,
                    card_y + 10.0,
                    portrait - 4.0,
                    portrait - 4.0,
                    [40, 92, 52],
                );
                let initial = hero
                    .name
                    .chars()
                    .next()
                    .map(|c| c.to_ascii_uppercase())
                    .unwrap_or('?');
                self.text_centered(
                    width,
                    height,
                    mx + 8.0 + portrait * 0.5,
                    card_y + 18.0,
                    2.4,
                    &initial.to_string(),
                    OSRS_YELLOW,
                );
                self.text(
                    width,
                    height,
                    mx + 58.0,
                    card_y + 7.0,
                    1.15,
                    &player.champion_name.to_uppercase(),
                    OSRS_WHITE,
                );
                self.text(
                    width,
                    height,
                    mx + 58.0,
                    card_y + 20.0,
                    0.95,
                    &format!(
                        "LV {}  {}/6  {}G",
                        player.level, player.items_owned, player.gold
                    ),
                    OSRS_YELLOW,
                );
                self.panel(
                    width,
                    height,
                    mx + 58.0,
                    card_y + 33.0,
                    124.0,
                    9.0,
                    (hero.health / hero.max_health.max(1.0)).clamp(0.0, 1.0),
                    [70, 190, 95],
                );
                self.panel(
                    width,
                    height,
                    mx + 58.0,
                    card_y + 44.0,
                    124.0,
                    7.0,
                    (hero.mana / hero.max_mana.max(1.0)).clamp(0.0, 1.0),
                    [70, 120, 235],
                );
            }
        }
        // Dota style unit tooltip that follows the cursor.
        if let Some(hover) = &player.hover {
            let title = if hover.kind == UnitKind::Player {
                hover.name.clone()
            } else {
                format!("{} LV{}", hover.name, hover.level)
            };
            let lines: [&str; 2] = [
                &format!(
                    "HP {:.0} / {:.0}   {}",
                    hover.health,
                    hover.max_health,
                    if hover.team == 0 {
                        "ALLIED"
                    } else if hover.team == 1 {
                        "ENEMY"
                    } else {
                        "NEUTRAL"
                    }
                ),
                hover.description,
            ];
            let box_w = 268.0;
            let box_h = 58.0;
            let bx = (mouse.0 as f32 + 20.0).min(width - box_w - 10.0).max(6.0);
            let by = (mouse.1 as f32 + 20.0).min(height - box_h - 10.0).max(6.0);
            self.hud_panel(width, height, bx, by, box_w, box_h);
            self.text(width, height, bx + 8.0, by + 6.0, 1.35, &title, OSRS_YELLOW);
            self.text(
                width,
                height,
                bx + 8.0,
                by + 22.0,
                1.05,
                &lines[0].to_uppercase(),
                OSRS_WHITE,
            );
            self.text(
                width,
                height,
                bx + 8.0,
                by + 36.0,
                1.0,
                &lines[1].to_uppercase(),
                [220, 215, 190],
            );
        }

        if let Some(winner) = match_over {
            let color = if winner == 0 {
                [80, 150, 255]
            } else {
                [220, 65, 60]
            };
            self.quad(width, height, 0.0, 0.0, width, 5.0, color);
            self.quad(width, height, 0.0, height - 5.0, width, 5.0, color);
            self.quad(width, height, 0.0, 0.0, 5.0, height, color);
            self.quad(width, height, width - 5.0, 0.0, 5.0, height, color);
            self.text_centered(
                width,
                height,
                width * 0.5,
                height * 0.34,
                5.0,
                if winner == 0 { "VICTORY" } else { "DEFEAT" },
                color,
            );
        }

        if let Some(remaining) = player.respawn_remaining {
            self.quad_alpha(width, height, 0.0, 0.0, width, height, [0, 0, 0], 0.30);
            self.text_centered(
                width,
                height,
                width * 0.5,
                height * 0.30,
                2.8,
                "YOU HAVE BEEN SLAIN",
                [255, 90, 80],
            );
            self.text_centered(
                width,
                height,
                width * 0.5,
                height * 0.30 + 44.0,
                1.35,
                &format!("RESPAWNING IN {:.0}", remaining.ceil()),
                [245, 235, 220],
            );
            self.text_centered(
                width,
                height,
                width * 0.5,
                height * 0.30 + 68.0,
                0.8,
                "THE FOUNTAIN AWAITS",
                [190, 190, 200],
            );
        }

        if !hide_player_hud {
            // Four ability boxes, bottom-centre.
            let box_size = 58.0;
            let gap = 8.0;
            let total = box_size * 4.0 + gap * 3.0;
            let x0 = (width - total) * 0.5;
            let y0 = height - 96.0;
            let colors = [
                [255, 150, 40],
                [90, 220, 255],
                [90, 255, 140],
                [255, 90, 60],
            ];
            let maxima = player.cooldown_maxima;
            for (i, color) in colors.iter().enumerate() {
                let bx = x0 + i as f32 * (box_size + gap);
                self.slot_frame(width, height, bx, y0, box_size, box_size);
                let cooldown = player.cooldowns[i];
                // A thin class-coloured strip only when the slot is ready; the
                // old full colour block read as a placeholder.
                self.quad(
                    width,
                    height,
                    bx + 4.0,
                    y0 + box_size - 5.0,
                    box_size - 8.0,
                    2.0,
                    if cooldown <= 0.0 {
                        *color
                    } else {
                        [44, 44, 48]
                    },
                );
                let icon_key = crate::ui_live::ability_icon_role(&player.champion_name, i);
                self.icon(
                    width,
                    height,
                    bx + 14.0,
                    y0 + 14.0,
                    box_size - 28.0,
                    &icon_key,
                    [255, 255, 255],
                );
                // Clockwise radial sweep, the genre-standard cooldown read.
                if cooldown > 0.0 {
                    let frac = (cooldown / maxima[i]).clamp(0.0, 1.0);
                    self.cooldown_wedge(
                        width,
                        height,
                        bx + box_size * 0.5,
                        y0 + box_size * 0.5,
                        box_size * 0.5,
                        frac,
                        [10, 12, 18],
                        0.74,
                    );
                }
                self.text(
                    width,
                    height,
                    bx + 4.0,
                    y0 + 3.0,
                    1.2,
                    ["Q", "W", "E", "R"][i],
                    *color,
                );
                if cooldown > 0.0 {
                    self.text_centered(
                        width,
                        height,
                        bx + box_size * 0.5,
                        y0 + 40.0,
                        1.1,
                        &format!("{:.0}", cooldown.ceil()),
                        [220, 220, 220],
                    );
                }
            }
        }
        // Enemy team card on the right edge. Five fixed slots keep the card
        // stable before a full 5v5 roster exists.
        let tx = width - 238.0;
        let card_h = 162.0;
        self.hud_panel(width, height, tx, 4.0, 220.0, card_h);
        self.text(
            width,
            height,
            tx + 8.0,
            10.0,
            HUD_TITLE_SCALE,
            "ENEMY TEAM",
            [255, 120, 110],
        );
        self.card_rule(width, height, tx, 4.0, 220.0);
        let enemies: Vec<&HeroMarker> =
            player.roster.iter().filter(|hero| hero.team == 1).collect();
        for slot in 0..5usize {
            let y = 26.0 + slot as f32 * 26.0;
            self.quad(width, height, tx + 8.0, y, 22.0, 22.0, [58, 36, 38]);
            self.quad(width, height, tx + 9.0, y + 1.0, 20.0, 20.0, [118, 58, 56]);
            if let Some(hero) = enemies.get(slot) {
                let initial = hero
                    .name
                    .chars()
                    .next()
                    .map(|c| c.to_ascii_uppercase())
                    .unwrap_or('?');
                self.text_centered(
                    width,
                    height,
                    tx + 19.0,
                    y + 5.0,
                    1.1,
                    &initial.to_string(),
                    [255, 225, 210],
                );
                let name: String = hero.name.chars().take(13).collect();
                self.text(
                    width,
                    height,
                    tx + 36.0,
                    y + 2.0,
                    0.85,
                    &name,
                    [240, 235, 230],
                );
                self.text(
                    width,
                    height,
                    tx + 190.0,
                    y + 2.0,
                    HUD_SMALL_SCALE,
                    &format!("LV{}", hero.level),
                    [220, 210, 190],
                );
                let hp = (hero.health / hero.max_health.max(1.0)).clamp(0.0, 1.0);
                self.panel(
                    width,
                    height,
                    tx + 36.0,
                    y + 13.0,
                    154.0,
                    6.0,
                    hp,
                    if hero.alive {
                        [200, 70, 62]
                    } else {
                        [80, 50, 50]
                    },
                );
                let mp = (hero.mana / hero.max_mana.max(1.0)).clamp(0.0, 1.0);
                self.panel(
                    width,
                    height,
                    tx + 36.0,
                    y + 20.0,
                    154.0,
                    4.0,
                    mp,
                    [70, 120, 235],
                );
            } else {
                self.text_centered(width, height, tx + 19.0, y + 5.0, 1.0, "?", [120, 105, 105]);
                self.text(
                    width,
                    height,
                    tx + 36.0,
                    y + 3.0,
                    0.8,
                    "OPEN SLOT",
                    [105, 100, 100],
                );
                self.panel(
                    width,
                    height,
                    tx + 36.0,
                    y + 15.0,
                    154.0,
                    4.0,
                    0.0,
                    [48, 48, 52],
                );
            }
        }

        // Centre-screen announcement, like the reference client's kill shout.
        // The death overlay already owns the screen when the player is dead.
        if !dead {
            if let Some(announcement) = player.announcement.as_deref() {
                self.text_centered(
                    width,
                    height,
                    width * 0.5,
                    height * 0.30,
                    1.4,
                    announcement,
                    OSRS_YELLOW,
                );
            }
        }

        // Kill feed under the roster panel.
        for (index, line) in player.kill_feed.iter().rev().take(4).enumerate() {
            self.text(
                width,
                height,
                width - 230.0,
                172.0 + index as f32 * 13.0,
                0.85,
                line,
                [225, 230, 240],
            );
        }

        // Screen-space unit plates: name, level, HP and (for champions) mana.
        // These replace the old world-space bars on champions and structures,
        // which were too small to read at the reference camera distance.
        let frames = nudge_unit_frames(player.unit_frames.clone());
        for frame in &frames {
            let w = frame.width;
            let x = (frame.x - w * 0.5).clamp(4.0, (width - w - 4.0).max(4.0));
            let y = (frame.y - 30.0).clamp(64.0, (height - 140.0).max(64.0));
            let bar_h = if frame.hero {
                8.0
            } else if frame.structure {
                6.0
            } else {
                5.0
            };
            // A level chip to the right of the bar shifts the bar left by
            // half the chip so the whole plate still reads as centred.
            let show_level = frame.hero || frame.camp;
            let level_text = frame.level.to_string();
            let level_width = self.text_width_named("p12", &level_text, 1.0);
            let level_box_w = (level_width + 7.0).max(16.0);
            let level_box_h = 14.0;
            let shift = if show_level {
                (level_box_w + 4.0) * 0.5
            } else {
                0.0
            };
            let bar_x = (x - shift).clamp(4.0, (width - w - 4.0).max(4.0));

            // Big, solid OSRS overhead name with a crisp black shadow. No
            // dark plate behind it; the name sits directly on the world.
            let name: String = frame.name.chars().take(22).collect();
            let name_scale = frame.name_scale.max(1.0);
            let name_top = y - 7.0 - self.font_ascent_named("p12") * name_scale;
            self.text_centered_named(
                "p12",
                width,
                height,
                bar_x + w * 0.5,
                name_top,
                name_scale,
                &name,
                frame.name_color,
            );
            // "NO SAFE" and the PK skull, like the reference client's
            // warning label above a flagged player.
            let center = bar_x + w * 0.5;
            if let Some(tag) = frame.tag.as_deref() {
                let tag_top = name_top - 14.0;
                self.text_centered_named(
                    "b12",
                    width,
                    height,
                    center,
                    tag_top,
                    1.0,
                    tag,
                    [255, 255, 0],
                );
                if frame.pk {
                    if let Some(skull) = self.atlas.hitmark("skull") {
                        let size = 25.0;
                        self.sprite(
                            width,
                            height,
                            center - size * 0.5,
                            tag_top - size - 4.0,
                            size,
                            size,
                            skull.uv,
                            [255, 255, 255],
                            1.0,
                        );
                    }
                }
            }

            self.quad(width, height, bar_x, y, w, bar_h, [16, 16, 20]);
            self.quad(
                width,
                height,
                bar_x,
                y,
                w * (frame.health / frame.max_health.max(1.0)).clamp(0.0, 1.0),
                bar_h,
                if frame.team == 0 {
                    [70, 200, 105]
                } else if frame.team == 2 {
                    [220, 165, 60]
                } else {
                    [210, 60, 55]
                },
            );
            if frame.hero && frame.max_mana > 0.0 {
                self.quad(width, height, bar_x, y + bar_h + 1.0, w, 3.0, [16, 16, 22]);
                self.quad(
                    width,
                    height,
                    bar_x,
                    y + bar_h + 1.0,
                    w * (frame.mana / frame.max_mana.max(1.0)).clamp(0.0, 1.0),
                    3.0,
                    [70, 120, 235],
                );
            }
            if show_level {
                let cx = bar_x + w + 4.0;
                let cy = y + bar_h * 0.5 - level_box_h * 0.5;
                self.quad_alpha(
                    width,
                    height,
                    cx,
                    cy,
                    level_box_w,
                    level_box_h,
                    [8, 9, 12],
                    0.78,
                );
                self.quad(width, height, cx, cy, level_box_w, 1.0, [96, 86, 58]);
                self.text_centered_named(
                    "p12",
                    width,
                    height,
                    cx + level_box_w * 0.5,
                    cy + 1.0,
                    1.0,
                    &level_text,
                    [235, 225, 150],
                );
            }
        }

        // OSRS hitmarks: the exact cache splat for the damage kind, with
        // the bold cache font centred over it. Hits on one unit stack down
        // the body, so a combo reads at a glance.
        for hit in &player.hitsplats {
            let sprite = self.atlas.hitmark(hit.kind.sprite_name());
            let (sw, sh) = sprite
                .map(|slot| (slot.width * HITMARK_SCALE, slot.height * HITMARK_SCALE))
                .unwrap_or((24.0 * HITMARK_SCALE, 23.0 * HITMARK_SCALE));
            let cx = hit.x.round();
            let cy = hit.y.round() + hit.stack as f32 * (sh + HITMARK_GAP);
            let sx = (cx - sw * 0.5).round();
            let sy = (cy - sh * 0.5).round();
            if let Some(slot) = sprite {
                self.sprite(
                    width,
                    height,
                    sx,
                    sy,
                    sw,
                    sh,
                    slot.uv,
                    hit.tint.unwrap_or([255, 255, 255]),
                    hit.alpha,
                );
            } else {
                self.quad_alpha(
                    width,
                    height,
                    sx,
                    sy,
                    sw,
                    sh,
                    hit.kind.fallback_color(),
                    hit.alpha * 0.92,
                );
            }

            // OSRS uses a bold 12px font in the splat. Two-digit hits get
            // the full 3x treatment; longer numbers shrink so they stay
            // legible instead of running past the sprite.
            let digits = hit.text.chars().filter(|c| c.is_ascii_digit()).count();
            let text_scale = if digits <= 2 {
                3.0
            } else if digits <= 4 {
                2.0
            } else {
                1.0
            };
            let text_width = self.text_width_named("b12", &hit.text, text_scale);
            let text_x = (cx - text_width * 0.5).round();
            let text_y = (cy - self.font_digit_center_named("b12") * text_scale).round();
            let shadow = (text_scale * 0.5).round().max(1.0);
            self.text_named_raw(
                "b12",
                width,
                height,
                text_x + shadow,
                text_y + shadow,
                text_scale,
                &hit.text,
                [0, 0, 0],
                hit.alpha,
            );
            self.text_named_raw(
                "b12", width, height, text_x, text_y, text_scale, &hit.text, OSRS_WHITE, hit.alpha,
            );
        }

        // Plain floating labels (gold pickups and developer text). Large
        // labels pop larger; a one-pixel shadow keeps them readable.
        for (x, y, text, color, scale) in &player.damage_numbers {
            self.text(
                width,
                height,
                *x + 1.0,
                *y + 1.0,
                *scale,
                text,
                [12, 12, 16],
            );
            self.text(width, height, *x, *y, *scale, text, *color);
        }

        // Committed-attack cast bar above the bottom cluster.
        if let Some(progress) = player.cast_progress {
            let cw = 210.0;
            let cx = width * 0.5 - cw * 0.5;
            let cy = height - 182.0;
            self.quad_alpha(
                width,
                height,
                cx - 3.0,
                cy - 3.0,
                cw + 6.0,
                14.0,
                [8, 9, 12],
                0.7,
            );
            self.quad(width, height, cx, cy, cw, 8.0, [30, 26, 20]);
            self.quad(
                width,
                height,
                cx,
                cy,
                cw * progress.clamp(0.0, 1.0),
                8.0,
                [250, 160, 55],
            );
            self.text_centered(
                width,
                height,
                cx + cw * 0.5,
                cy - 1.0,
                0.65,
                "CAST",
                [255, 225, 170],
            );
        }

        // Red edge vignette on damage, so a hit is felt even when the camera
        // is looking away from the health bar.
        if player.damage_flash > 0.01 {
            let alpha = (player.damage_flash * 0.55).clamp(0.0, 0.55);
            self.quad_alpha(width, height, 0.0, 0.0, width, 4.0, [220, 30, 25], alpha);
            self.quad_alpha(
                width,
                height,
                0.0,
                height - 4.0,
                width,
                4.0,
                [220, 30, 25],
                alpha,
            );
            self.quad_alpha(width, height, 0.0, 0.0, 4.0, height, [220, 30, 25], alpha);
            self.quad_alpha(
                width,
                height,
                width - 4.0,
                0.0,
                4.0,
                height,
                [220, 30, 25],
                alpha,
            );
        }

        // Current biome pocket label.
        if let Some(label) = player.biome.as_deref() {
            self.text_centered(width, height, width * 0.5, 96.0, 1.3, label, OSRS_YELLOW);
        }

        // Objectives card: team comparison and nexus health, like the single
        // macro panel in the reference client.
        let ox = 22.0;
        let oy = 112.0;
        let ow = 230.0;
        self.hud_panel(width, height, ox, oy, ow, 160.0);
        self.text(
            width,
            height,
            ox + 8.0,
            oy + 6.0,
            HUD_TITLE_SCALE,
            "OBJECTIVES",
            OSRS_GOLD,
        );
        self.card_rule(width, height, ox, oy, ow);
        self.text(
            width,
            height,
            ox + 8.0,
            oy + 22.0,
            0.75,
            "BLUE",
            [110, 175, 255],
        );
        self.text(
            width,
            height,
            ox + 132.0,
            oy + 22.0,
            0.75,
            "RED",
            [255, 120, 110],
        );
        let rows: [(&str, u32, u32); 4] = [
            ("KILLS", player.kills.0, player.kills.1),
            ("DEATHS", player.deaths.0, player.deaths.1),
            ("TOWERS", player.towers.0 as u32, player.towers.1 as u32),
            (
                "INHIBITORS",
                player.inhibitors.0 as u32,
                player.inhibitors.1 as u32,
            ),
        ];
        for (index, (label, blue, red)) in rows.iter().enumerate() {
            let y = oy + 34.0 + index as f32 * 12.0;
            self.text(
                width,
                height,
                ox + 8.0,
                y,
                HUD_BODY_SCALE,
                label,
                [205, 205, 205],
            );
            self.text(
                width,
                height,
                ox + 108.0,
                y,
                0.8,
                &blue.to_string(),
                [150, 200, 255],
            );
            self.text(
                width,
                height,
                ox + 188.0,
                y,
                0.8,
                &red.to_string(),
                [255, 170, 160],
            );
        }
        // Nexus health bars.
        self.text(
            width,
            height,
            ox + 8.0,
            oy + 86.0,
            0.8,
            "NEXUS",
            OSRS_YELLOW,
        );
        self.panel(
            width,
            height,
            ox + 66.0,
            oy + 88.0,
            150.0,
            6.0,
            (nexus0.0 / nexus0.1.max(1.0)).clamp(0.0, 1.0),
            [80, 150, 255],
        );
        self.panel(
            width,
            height,
            ox + 66.0,
            oy + 97.0,
            150.0,
            6.0,
            (nexus1.0 / nexus1.1.max(1.0)).clamp(0.0, 1.0),
            [220, 65, 60],
        );
        self.text(
            width,
            height,
            ox + 8.0,
            oy + 106.0,
            0.7,
            &format!("{:.0}", nexus0.0),
            [150, 200, 255],
        );
        self.text(
            width,
            height,
            ox + 188.0,
            oy + 106.0,
            0.7,
            &format!("{:.0}", nexus1.0),
            [255, 170, 160],
        );
        let buff_line = player
            .buff
            .map(|(remaining, _, _)| {
                format!(
                    "BUFF {} {:.0}S",
                    player.buff_label.unwrap_or("ACTIVE"),
                    remaining
                )
            })
            .unwrap_or_else(|| "BUFF NONE".to_string());
        self.text(
            width,
            height,
            ox + 8.0,
            oy + 126.0,
            0.75,
            &format!("WAVE {wave_timer:.0}S   {buff_line}"),
            [220, 190, 130],
        );
        let wave_fill = if wave_interval > 0.0 {
            (1.0 - wave_timer / wave_interval).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.quad(
            width,
            height,
            ox + 8.0,
            oy + 138.0,
            ow - 16.0,
            3.0,
            [38, 34, 30],
        );
        self.quad(
            width,
            height,
            ox + 8.0,
            oy + 138.0,
            (ow - 16.0) * wave_fill,
            3.0,
            [220, 190, 60],
        );
        self.text(
            width,
            height,
            ox + 8.0,
            oy + 146.0,
            0.7,
            &format!(
                "VISIBLE {}   MINIONS {}",
                player.visible_enemies, player.minions
            ),
            [150, 160, 170],
        );

        // Engineer stat sheet, opt-in. The reference client never shows this
        // during a normal match.
        if player.debug_hud {
            let sx = 22.0;
            let sy = oy + 176.0;
            self.hud_panel(width, height, sx, sy, 230.0, 142.0);
            self.text(
                width,
                height,
                sx + 8.0,
                sy + 6.0,
                HUD_TITLE_SCALE,
                "STATS",
                OSRS_GOLD,
            );
            self.card_rule(width, height, sx, sy, 230.0);
            let stat_rows = [
                ("ATTACK", player.attack_damage),
                ("STRENGTH", player.attack_damage),
                ("RANGED", player.ranged),
                ("PRAYER", player.mana),
                ("HITPOINTS", player.health),
            ];
            for (index, (label, value)) in stat_rows.iter().enumerate() {
                let y = sy + 20.0 + index as f32 * 13.0;
                self.text(width, height, sx + 8.0, y, 0.85, label, [205, 205, 205]);
                self.text(
                    width,
                    height,
                    sx + 150.0,
                    y,
                    0.85,
                    &format!("{value:.0}"),
                    [235, 235, 235],
                );
            }
            self.text(
                width,
                height,
                sx + 8.0,
                sy + 88.0,
                0.85,
                "BONUSES",
                OSRS_YELLOW,
            );
            let bonus_rows = [
                ("RUN-ATTACK", player.attack_interval),
                ("RANGED-BONUS", player.ability_power),
                ("MELEE-DEF", player.armor),
                ("RANGED-DEF", player.armor),
            ];
            for (index, (label, value)) in bonus_rows.iter().enumerate() {
                let y = sy + 102.0 + index as f32 * 12.0;
                self.text(width, height, sx + 8.0, y, 0.85, label, [205, 205, 205]);
                self.text(
                    width,
                    height,
                    sx + 150.0,
                    y,
                    0.85,
                    &format!("{value:.0}"),
                    [235, 235, 235],
                );
            }
        }

        // Event log, bottom-left.
        let lx = 18.0;
        let ly = height - 170.0;
        self.hud_panel(width, height, lx, ly, 340.0, 120.0);
        self.text(
            width,
            height,
            lx + 8.0,
            ly + 6.0,
            HUD_TITLE_SCALE,
            "CHAT",
            OSRS_GOLD,
        );
        self.card_rule(width, height, lx, ly, 340.0);
        if player.events.is_empty() {
            self.text(
                width,
                height,
                lx + 8.0,
                ly + 26.0,
                0.85,
                "NO EVENTS YET",
                [180, 185, 195],
            );
        } else {
            for (i, event) in player.events.iter().take(5).enumerate() {
                let line: String = event.chars().take(46).collect();
                // OSRS chat colouring: combat in red, waves in orange, hints in
                // yellow and everything else in plain white.
                let color = if event.starts_with("ENEMY") {
                    [235, 90, 80]
                } else if event.starts_with("WAVE") {
                    [255, 180, 80]
                } else if event.starts_with("PRESS") {
                    [245, 220, 120]
                } else if event.starts_with("FOUNTAIN") {
                    [120, 230, 150]
                } else {
                    [205, 210, 220]
                };
                self.text(
                    width,
                    height,
                    lx + 8.0,
                    ly + 26.0 + i as f32 * 15.0,
                    HUD_BODY_SCALE,
                    &line,
                    color,
                );
            }
        }

        if !hide_player_hud {
            // Buff indicator sits just above the bottom health bars; the
            // reference client keeps the top-left corner clear for the scoreboard.
            if let Some((remaining, duration, color)) = player.buff {
                let fill = (remaining / duration.max(0.1)).clamp(0.0, 1.0);
                let bx = width * 0.5 - 130.0;
                self.panel(width, height, bx, height - 168.0, 260.0, 10.0, fill, color);
                if let Some(label) = player.buff_label {
                    self.text(
                        width,
                        height,
                        bx + 6.0,
                        height - 167.0,
                        0.8,
                        &format!("{label} {remaining:.0}S"),
                        [240, 240, 240],
                    );
                }
            }

            // Health/mana bars above the ability row, with an OSRS-style orb.
            let cx = width * 0.5 - 130.0;
            self.orb(
                width,
                height,
                width * 0.5 - 168.0,
                height - 96.0,
                15.0,
                [70, 120, 235],
            );
            self.orb(
                width,
                height,
                width * 0.5 - 168.0,
                height - 96.0,
                11.0,
                [180, 210, 255],
            );
            self.panel(
                width,
                height,
                cx,
                height - 148.0,
                260.0,
                14.0,
                (player.health / player.max_health.max(1.0)).clamp(0.0, 1.0),
                [70, 190, 95],
            );
            self.text(
                width,
                height,
                cx + 6.0,
                height - 147.0,
                0.85,
                &format!("HP {:.0}/{:.0}", player.health, player.max_health),
                [245, 245, 245],
            );
            self.panel(
                width,
                height,
                cx,
                height - 130.0,
                260.0,
                12.0,
                (player.mana / player.max_mana.max(1.0)).clamp(0.0, 1.0),
                [70, 120, 235],
            );
            self.text(
                width,
                height,
                cx + 6.0,
                height - 129.0,
                0.8,
                &format!("MP {:.0}/{:.0}", player.mana, player.max_mana),
                [235, 242, 255],
            );

            // Shield overlay rides on top of the health bar; XP is a thin strip
            // under mana so the bottom cluster stays compact.
            if player.shield > 0.0 && player.shield_timer > 0.0 {
                let fraction = (player.shield / 120.0).clamp(0.0, 1.0);
                self.quad(
                    width,
                    height,
                    cx,
                    height - 148.0,
                    260.0 * fraction,
                    3.0,
                    [110, 225, 255],
                );
            }
            self.quad(width, height, cx, height - 116.0, 260.0, 3.0, [34, 30, 42]);
            self.quad(
                width,
                height,
                cx,
                height - 116.0,
                260.0 * (player.xp / player.xp_next.max(1.0)).clamp(0.0, 1.0),
                3.0,
                [160, 96, 220],
            );

            // Delivery guard button, bottom-right next to the shop button.
            {
                let (bx, by, bw, bh) = crate::items::delivery_button_rect(width, height);
                let ready = player.delivery_pending > 0 && player.delivery_state == 0;
                let hovered =
                    mouse_x >= bx && mouse_x <= bx + bw && mouse_y >= by && mouse_y <= by + bh;
                let label = match player.delivery_state {
                    1 => "GUARD IN FLIGHT".to_string(),
                    2 => "GUARD RETURNING".to_string(),
                    _ => format!("DELIVER ({})", player.delivery_pending),
                };
                self.quad(
                    width,
                    height,
                    bx - 2.0,
                    by - 2.0,
                    bw + 4.0,
                    bh + 4.0,
                    [8, 8, 10],
                );
                self.quad(
                    width,
                    height,
                    bx - 1.0,
                    by - 1.0,
                    bw + 2.0,
                    bh + 2.0,
                    if hovered && ready {
                        [150, 132, 82]
                    } else {
                        [82, 76, 58]
                    },
                );
                self.quad(
                    width,
                    height,
                    bx,
                    by,
                    bw,
                    bh,
                    if ready {
                        if hovered { [64, 98, 56] } else { [48, 74, 42] }
                    } else {
                        [28, 28, 32]
                    },
                );
                if hovered && ready {
                    self.rect_outline(width, height, bx, by, bw, bh, 1.0, [236, 200, 110], 0.8);
                }
                self.text_centered(
                    width,
                    height,
                    bx + bw * 0.5,
                    by + 6.0,
                    1.0,
                    &label,
                    if ready {
                        [210, 255, 180]
                    } else {
                        [140, 140, 140]
                    },
                );
            }

            // Inventory and shop button, bottom-right.
            let inv_x = width - 454.0;
            let inv_y = height - 96.0;
            self.draw_inventory_slots(width, height, inv_x, inv_y, 34.0, 6.0, shop);
            let (bx, by, bw, bh) = shop_button_rect(width, height);
            let shop_hovered =
                mouse_x >= bx && mouse_x <= bx + bw && mouse_y >= by && mouse_y <= by + bh;
            self.quad(
                width,
                height,
                bx - 2.0,
                by - 2.0,
                bw + 4.0,
                bh + 4.0,
                [8, 8, 10],
            );
            self.quad(
                width,
                height,
                bx - 1.0,
                by - 1.0,
                bw + 2.0,
                bh + 2.0,
                if shop_hovered {
                    [152, 134, 84]
                } else {
                    [82, 76, 58]
                },
            );
            if let Some(uv) = self.atlas.ui_uv("tab") {
                self.sprite(
                    width,
                    height,
                    bx,
                    by,
                    bw,
                    bh,
                    uv,
                    if shop_hovered {
                        [255, 252, 230]
                    } else {
                        [248, 232, 190]
                    },
                    1.0,
                );
                self.quad_alpha(
                    width,
                    height,
                    bx + 3.0,
                    by + 3.0,
                    bw - 6.0,
                    bh - 6.0,
                    if shop_hovered {
                        [40, 38, 28]
                    } else {
                        [24, 24, 28]
                    },
                    0.82,
                );
            } else {
                self.quad(
                    width,
                    height,
                    bx,
                    by,
                    bw,
                    bh,
                    if shop_hovered {
                        [58, 52, 36]
                    } else {
                        [30, 28, 22]
                    },
                );
            }
            self.text_centered(
                width,
                height,
                bx + bw * 0.5,
                by + 7.0,
                0.9,
                if player.shop_open {
                    "CLOSE [B]"
                } else {
                    "SHOP [B]"
                },
                if shop_hovered {
                    [255, 248, 220]
                } else {
                    [245, 220, 150]
                },
            );
        }
        // Crosshair at the cursor.
        if player.shop_open {
            return;
        }
        let mx = mouse.0 as f32;
        let my = mouse.1 as f32;
        self.quad(
            width,
            height,
            mx - 8.0,
            my - 1.0,
            16.0,
            2.0,
            [235, 235, 235],
        );
        self.quad(
            width,
            height,
            mx - 1.0,
            my - 8.0,
            2.0,
            16.0,
            [235, 235, 235],
        );
    }

    /// Colour used for a combat class badge: melee red, ranged blue, magic green.
    fn class_color(class: &str) -> [u8; 3] {
        if class.eq_ignore_ascii_case("RANGED") {
            [96, 150, 235]
        } else if class.eq_ignore_ascii_case("MAGIC") {
            [110, 205, 130]
        } else {
            [225, 105, 95]
        }
    }

    /// The 5v5 lobby shown between the hero pick and the match: two rows of
    /// five hero cards, a team tab on each side and the red loading button.
    pub fn build_lobby(&mut self, width: f32, height: f32, select: &HeroSelect, mouse: (f64, f64)) {
        self.vertices.clear();
        self.indices.clear();
        self.quad_alpha(width, height, 0.0, 0.0, width, height, [7, 9, 13], 0.88);

        self.text_centered(
            width,
            height,
            width * 0.5,
            height * 0.045,
            2.4,
            "KARAMJA RIFT",
            [245, 214, 137],
        );
        self.text_centered(
            width,
            height,
            width * 0.5,
            height * 0.045 + 38.0,
            0.95,
            "5V5 RANKED - BOTH TEAMS LOCKED IN",
            [178, 184, 196],
        );
        let rule_w = (width * 0.62).min(820.0);
        self.quad_alpha(
            width,
            height,
            (width - rule_w) * 0.5,
            height * 0.045 + 60.0,
            rule_w,
            1.0,
            [188, 154, 86],
            0.7,
        );

        // Team tabs down each side.
        let (left_tab, right_tab) = hero_select::lobby_side_tabs(width, height);
        for (rect, label, color) in [
            (left_tab, "RADIANT", [96, 150, 235]),
            (right_tab, "DIRE", [235, 105, 95]),
        ] {
            self.quad(width, height, rect.0, rect.1, rect.2, rect.3, color);
            self.text_centered(
                width,
                height,
                rect.0 + rect.2 * 0.5,
                rect.1 + rect.3 * 0.5 - 6.0,
                0.95,
                label,
                [250, 250, 250],
            );
        }

        let (radiant, dire) = hero_select::lobby_teams(&select.champions, select.selected);
        let cards = hero_select::lobby_card_rects(width, height);
        for (index, &(x, y, cw, ch)) in cards.iter().enumerate() {
            let team = index / 5;
            let slot = index % 5;
            let champion_index = if team == 0 {
                radiant.get(slot)
            } else {
                dire.get(slot)
            };
            let Some(&champion_index) = champion_index else {
                continue;
            };
            let Some(champion) = select.champions.get(champion_index) else {
                continue;
            };
            let picked = champion_index == select.selected;
            let accent = if team == 0 {
                [110, 175, 255]
            } else {
                [255, 120, 110]
            };
            self.quad(
                width,
                height,
                x,
                y,
                cw,
                ch,
                if picked { [86, 66, 36] } else { [34, 34, 40] },
            );
            self.rect_outline(
                width,
                height,
                x + 1.0,
                y + 1.0,
                cw - 2.0,
                ch - 2.0,
                if picked { 2.0 } else { 1.0 },
                if picked {
                    [244, 206, 118]
                } else {
                    [82, 80, 74]
                },
                0.9,
            );
            // Team stripe.
            self.quad(width, height, x + 4.0, y + 4.0, 6.0, ch - 8.0, accent);

            let portrait = (cw * 0.52).min(ch * 0.60);
            let px = x + (cw - portrait) * 0.5;
            let py = y + 10.0;
            self.quad_alpha(
                width,
                height,
                px,
                py,
                portrait,
                portrait,
                [18, 21, 28],
                0.95,
            );
            if let Some(uv) = self.atlas.hero_uv(&champion.id) {
                self.sprite(
                    width,
                    height,
                    px,
                    py,
                    portrait,
                    portrait,
                    uv,
                    [255, 255, 255],
                    1.0,
                );
            }
            let name: String = champion.name.chars().take(13).collect();
            self.text_centered(
                width,
                height,
                x + cw * 0.5,
                py + portrait + 4.0,
                0.78,
                &name,
                [238, 232, 214],
            );
            self.text_centered(
                width,
                height,
                x + cw * 0.5,
                py + portrait + 19.0,
                0.7,
                &champion.class,
                Self::class_color(&champion.class),
            );
            self.text_centered(
                width,
                height,
                x + cw * 0.5,
                py + portrait + 33.0,
                0.68,
                &format!(
                    "HP {}  MP {}  RNG {}",
                    champion.health as i32, champion.mana as i32, champion.ranged as i32
                ),
                [176, 178, 186],
            );
        }

        // The red loading button.
        let (bx, by, bw, bh) = hero_select::lobby_button_rect(width, height);
        let hovered = hero_select::lobby_button_at(width, height, mouse);
        self.quad(
            width,
            height,
            bx,
            by,
            bw,
            bh,
            if hovered {
                [190, 60, 60]
            } else {
                [150, 44, 44]
            },
        );
        self.rect_outline(width, height, bx, by, bw, bh, 1.0, [236, 190, 96], 0.9);
        self.text_centered(
            width,
            height,
            bx + bw * 0.5,
            by + bh * 0.5 - 7.0,
            1.0,
            "LOADING - PLEASE WAIT...",
            [255, 238, 206],
        );
    }

    pub fn build_shop(&mut self, width: f32, height: f32, player: &PlayerHud, shop: &ItemShop) {
        if !player.shop_open {
            return;
        }
        let (wx, wy, ww, wh) = shop_window_rect(width, height);
        let affordable = shop.affordable(player.gold);

        // Modal dimmer and window frame.
        self.quad_alpha(width, height, 0.0, 0.0, width, height, [0, 0, 0], 0.42);
        if let Some(uv) = self.atlas.ui_uv("panel") {
            self.panel_sprite(width, height, wx, wy, ww, wh, uv, 14.0, [92, 84, 72]);
        } else {
            self.quad(
                width,
                height,
                wx - 2.0,
                wy - 2.0,
                ww + 4.0,
                wh + 4.0,
                [96, 82, 50],
            );
        }
        self.quad_alpha(
            width,
            height,
            wx + 6.0,
            wy + 6.0,
            ww - 12.0,
            wh - 12.0,
            [18, 20, 26],
            0.94,
        );
        if let Some(uv) = self.atlas.ui_uv("tab") {
            self.sprite(
                width,
                height,
                wx + 2.0,
                wy + 2.0,
                ww - 4.0,
                32.0,
                uv,
                [255, 255, 255],
                1.0,
            );
            self.quad_alpha(
                width,
                height,
                wx + 5.0,
                wy + 5.0,
                ww - 10.0,
                26.0,
                [18, 20, 26],
                0.88,
            );
        } else {
            self.quad(
                width,
                height,
                wx + 2.0,
                wy + 2.0,
                ww - 4.0,
                32.0,
                [34, 31, 24],
            );
        }
        self.quad(
            width,
            height,
            wx + 2.0,
            wy + 34.0,
            ww - 4.0,
            2.0,
            [120, 100, 60],
        );

        self.text_centered(
            width,
            height,
            wx + ww * 0.5,
            wy + 10.0,
            1.6,
            "FOUNTAIN SHOP",
            [245, 220, 160],
        );
        let gold_text = format!("{}G", player.gold);
        let gold_width = self.text_width(&gold_text, 1.1);
        self.text(
            width,
            height,
            wx + ww - 160.0 - gold_width,
            wy + 12.0,
            1.1,
            &gold_text,
            [245, 205, 70],
        );
        let (cx, cy, cw, ch) = shop_close_rect(width, height);
        self.quad(width, height, cx, cy, cw, ch, [110, 55, 50]);
        self.quad(
            width,
            height,
            cx + 2.0,
            cy + 2.0,
            cw - 4.0,
            ch - 4.0,
            [40, 20, 20],
        );
        self.text_centered(
            width,
            height,
            cx + cw * 0.5,
            cy + 6.0,
            1.2,
            "X",
            [255, 220, 210],
        );

        // Category sidebar.
        for (index, label) in CATEGORY_LABELS.iter().enumerate() {
            let (x, y, w, h) = shop_category_rect(width, height, index);
            let selected = shop.category == index;
            if let Some(uv) = self.atlas.ui_uv("tab") {
                self.sprite(
                    width,
                    height,
                    x,
                    y,
                    w,
                    h,
                    uv,
                    if selected {
                        [255, 230, 160]
                    } else {
                        [220, 210, 190]
                    },
                    1.0,
                );
                self.quad_alpha(
                    width,
                    height,
                    x + 3.0,
                    y + 3.0,
                    w - 6.0,
                    h - 6.0,
                    [22, 22, 26],
                    0.86,
                );
            } else {
                self.quad(
                    width,
                    height,
                    x,
                    y,
                    w,
                    h,
                    if selected { [78, 64, 34] } else { [28, 30, 36] },
                );
            }
            if selected {
                self.quad(width, height, x, y, 3.0, h, [225, 185, 90]);
            }
            self.text(
                width,
                height,
                x + 9.0,
                y + 7.0,
                0.9,
                label,
                if selected {
                    [250, 225, 160]
                } else {
                    [185, 190, 200]
                },
            );
        }

        // Item cards.
        let filtered = shop.filtered_items();
        for (slot, item_index) in filtered.iter().enumerate().take(6) {
            let (x, y, w, h) = shop_card_rect(width, height, slot);
            let item = &shop.items[*item_index];
            let hover = player.shop_hover == Some(*item_index);
            let selected = player.shop_selected == Some(*item_index);
            let can_buy = affordable[*item_index];
            let border = if selected {
                [230, 190, 90]
            } else if hover {
                [120, 200, 255]
            } else {
                [82, 76, 58]
            };
            self.quad(width, height, x, y, w, h, border);
            self.quad(
                width,
                height,
                x + 2.0,
                y + 2.0,
                w - 4.0,
                h - 4.0,
                if hover || selected {
                    [38, 40, 48]
                } else {
                    [24, 26, 32]
                },
            );
            let icon_alpha = if can_buy { 1.0 } else { 0.45 };
            self.draw_item_icon(
                width,
                height,
                x + (w - 46.0) * 0.5,
                y + 8.0,
                46.0,
                item,
                icon_alpha,
            );
            let name: String = item.name.chars().take(16).collect();
            self.text_centered(
                width,
                height,
                x + w * 0.5,
                y + 60.0,
                0.8,
                &name,
                if can_buy {
                    [235, 235, 235]
                } else {
                    [135, 135, 145]
                },
            );
            self.text_centered(
                width,
                height,
                x + w * 0.5,
                y + 74.0,
                0.9,
                &format!("{}G", item.cost),
                if can_buy {
                    [245, 205, 70]
                } else {
                    [170, 95, 95]
                },
            );
            let owned = shop.owned_count(*item_index);
            if owned > 0 {
                self.quad(
                    width,
                    height,
                    x + w - 16.0,
                    y + 6.0,
                    9.0,
                    9.0,
                    [80, 255, 120],
                );
                if owned > 1 {
                    self.text(
                        width,
                        height,
                        x + w - 26.0,
                        y + 17.0,
                        0.7,
                        &format!("x{owned}"),
                        [170, 255, 190],
                    );
                }
            }
        }

        // Detail pane.
        let (dx, dy, dw, dh) = shop_detail_rect(width, height);
        self.quad(width, height, dx, dy, dw, dh, [82, 76, 58]);
        self.quad(
            width,
            height,
            dx + 2.0,
            dy + 2.0,
            dw - 4.0,
            dh - 4.0,
            [18, 20, 25],
        );
        let detail_index = player.shop_hover.or(player.shop_selected).or(shop.selected);
        if let Some(index) = detail_index {
            if let Some(item) = shop.items.get(index) {
                let can_buy = affordable[index] && shop.has_free_slot();
                let name: String = item.name.chars().take(20).collect();
                self.text(
                    width,
                    height,
                    dx + 10.0,
                    dy + 8.0,
                    1.0,
                    &name,
                    [240, 235, 220],
                );
                self.text(
                    width,
                    height,
                    dx + 10.0,
                    dy + 22.0,
                    0.9,
                    &format!("{}G", item.cost),
                    [245, 205, 70],
                );
                self.draw_item_icon(width, height, dx + 10.0, dy + 40.0, 52.0, item, 1.0);

                let mut lines = Vec::new();
                if item.damage > 0.0 {
                    lines.push(format!("+{:.0} DAMAGE", item.damage));
                }
                if item.health > 0.0 {
                    lines.push(format!("+{:.0} HEALTH", item.health));
                }
                if item.mana > 0.0 {
                    lines.push(format!("+{:.0} MANA", item.mana));
                }
                if item.speed > 0.0 {
                    lines.push(format!("+{:.0} SPEED", item.speed));
                }
                if item.ability > 0.0 {
                    lines.push(format!("+{:.0}% ABILITY", item.ability));
                }
                if lines.is_empty() {
                    lines.push("NO BONUS".to_string());
                }
                for (i, line) in lines.iter().take(5).enumerate() {
                    self.text(
                        width,
                        height,
                        dx + 10.0,
                        dy + 100.0 + i as f32 * 12.0,
                        0.85,
                        line,
                        [200, 220, 240],
                    );
                }

                let owned = shop.owned_count(index);
                let in_transit = player
                    .pending_ids
                    .iter()
                    .any(|id| shop.items.get(index).is_some_and(|item| &item.id == id));
                let status = if in_transit && owned == 0 {
                    "IN TRANSIT"
                } else if owned > 0 {
                    "OWNED"
                } else if !affordable[index] {
                    "NEED GOLD"
                } else if !shop.has_free_slot() {
                    "INVENTORY FULL"
                } else {
                    "IN STOCK"
                };
                self.text(
                    width,
                    height,
                    dx + 10.0,
                    dy + 166.0,
                    0.8,
                    status,
                    if can_buy {
                        [160, 235, 170]
                    } else {
                        [230, 130, 120]
                    },
                );

                let (bx, by, bw, bh) = shop_buy_rect(width, height);
                if let Some(uv) = self.atlas.ui_uv("tab") {
                    self.sprite(
                        width,
                        height,
                        bx,
                        by,
                        bw,
                        bh,
                        uv,
                        if can_buy {
                            [150, 255, 170]
                        } else {
                            [150, 150, 150]
                        },
                        if can_buy { 0.95 } else { 0.55 },
                    );
                    self.quad_alpha(
                        width,
                        height,
                        bx + 4.0,
                        by + 4.0,
                        bw - 8.0,
                        bh - 8.0,
                        [18, 20, 24],
                        0.82,
                    );
                } else {
                    self.quad(
                        width,
                        height,
                        bx,
                        by,
                        bw,
                        bh,
                        if can_buy { [52, 120, 58] } else { [55, 55, 62] },
                    );
                }
                self.text_centered(
                    width,
                    height,
                    bx + bw * 0.5,
                    by + 9.0,
                    0.9,
                    &format!("BUY  {}G", item.cost),
                    [245, 245, 245],
                );

                let (sx, sy, sw, sh) = shop_sell_rect(width, height);
                let can_sell = owned > 0;
                if let Some(uv) = self
                    .atlas
                    .ui_uv(if can_sell { "button_red" } else { "button" })
                {
                    self.sprite(
                        width,
                        height,
                        sx,
                        sy,
                        sw,
                        sh,
                        uv,
                        if can_sell {
                            [255, 200, 140]
                        } else {
                            [140, 140, 140]
                        },
                        if can_sell { 0.95 } else { 0.50 },
                    );
                    self.quad_alpha(
                        width,
                        height,
                        sx + 4.0,
                        sy + 3.0,
                        sw - 8.0,
                        sh - 6.0,
                        [18, 20, 24],
                        0.82,
                    );
                } else {
                    self.quad(
                        width,
                        height,
                        sx,
                        sy,
                        sw,
                        sh,
                        if can_sell {
                            [120, 72, 34]
                        } else {
                            [48, 48, 54]
                        },
                    );
                }
                let sell_text = if can_sell {
                    format!("SELL  +{}G", item.cost / 2)
                } else {
                    "SELL".to_string()
                };
                self.text_centered(
                    width,
                    height,
                    sx + sw * 0.5,
                    sy + 7.0,
                    0.8,
                    &sell_text,
                    if can_sell {
                        [255, 225, 180]
                    } else {
                        [130, 130, 140]
                    },
                );
            }
        }

        // Inventory row and footer.
        self.text(
            width,
            height,
            wx + 166.0,
            wy + 282.0,
            0.9,
            "INVENTORY",
            [220, 200, 150],
        );
        self.draw_inventory_slots(width, height, wx + 166.0, wy + 300.0, 38.0, 6.0, shop);
        self.text(
            width,
            height,
            wx + 520.0,
            wy + 282.0,
            0.9,
            &format!("GOLD {}G", player.gold),
            [245, 205, 70],
        );
        self.text_centered(
            width,
            height,
            wx + ww * 0.5,
            wy + wh - 18.0,
            0.8,
            "CLICK ITEM TO SELECT  ·  BUY  ·  B/ESC TO CLOSE",
            [160, 160, 170],
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_item_icon(
        &mut self,
        width: f32,
        height: f32,
        x: f32,
        y: f32,
        size: f32,
        item: &ItemConfig,
        alpha: f32,
    ) {
        if let Some(uv) = self.atlas.item_uv(&item.id) {
            self.sprite(width, height, x, y, size, size, uv, [255, 255, 255], alpha);
            return;
        }
        self.quad_alpha(width, height, x, y, size, size, [12, 14, 18], 0.92 * alpha);
        let c = item.color;
        let light = [
            c[0].saturating_add(45),
            c[1].saturating_add(45),
            c[2].saturating_add(45),
        ];
        let dark = [c[0] / 2, c[1] / 2, c[2] / 2];
        let q = |hud: &mut Hud, fx: f32, fy: f32, fw: f32, fh: f32, col: [u8; 3]| {
            hud.quad_alpha(
                width,
                height,
                x + fx * size,
                y + fy * size,
                fw * size,
                fh * size,
                col,
                alpha,
            );
        };
        match item.id.as_str() {
            "bronze_sword" => {
                q(self, 0.46, 0.12, 0.08, 0.50, light);
                q(self, 0.30, 0.58, 0.40, 0.08, c);
                q(self, 0.46, 0.66, 0.08, 0.18, dark);
            }
            "iron_plate" => {
                q(self, 0.34, 0.26, 0.32, 0.42, c);
                q(self, 0.22, 0.22, 0.16, 0.18, light);
                q(self, 0.62, 0.22, 0.16, 0.18, light);
                q(self, 0.46, 0.26, 0.08, 0.10, dark);
            }
            "fury" => {
                q(self, 0.30, 0.20, 0.08, 0.18, light);
                q(self, 0.62, 0.20, 0.08, 0.18, light);
                q(self, 0.38, 0.34, 0.24, 0.10, c);
                q(self, 0.44, 0.46, 0.12, 0.18, light);
            }
            "berserker" => {
                q(self, 0.28, 0.28, 0.44, 0.44, c);
                q(self, 0.39, 0.39, 0.22, 0.22, dark);
            }
            "ahrims" => {
                q(self, 0.46, 0.16, 0.08, 0.66, c);
                q(self, 0.36, 0.10, 0.28, 0.28, light);
                q(self, 0.42, 0.16, 0.16, 0.16, dark);
            }
            "boots" => {
                q(self, 0.34, 0.24, 0.22, 0.42, c);
                q(self, 0.34, 0.60, 0.42, 0.16, light);
                q(self, 0.34, 0.72, 0.42, 0.06, dark);
            }
            _ => {
                q(self, 0.28, 0.28, 0.44, 0.44, c);
                if let Some(ch) = item.name.chars().next() {
                    self.text_centered(
                        width,
                        height,
                        x + size * 0.5,
                        y + size * 0.36,
                        1.2,
                        &ch.to_string(),
                        [245, 245, 245],
                    );
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_inventory_slots(
        &mut self,
        width: f32,
        height: f32,
        x: f32,
        y: f32,
        slot: f32,
        gap: f32,
        shop: &ItemShop,
    ) {
        for i in 0..6 {
            let sx = x + i as f32 * (slot + gap);
            self.slot_frame(width, height, sx, y, slot, slot);
            if let Some(index) = shop.owned[i] {
                if let Some(item) = shop.items.get(index) {
                    self.draw_item_icon(width, height, sx + 3.0, y + 3.0, slot - 6.0, item, 1.0);
                    let count = shop.owned_count(index);
                    if count > 1 {
                        self.text(
                            width,
                            height,
                            sx + slot - 12.0,
                            y + slot - 12.0,
                            0.8,
                            &format!("x{count}"),
                            [245, 220, 140],
                        );
                    }
                }
            } else {
                self.quad(
                    width,
                    height,
                    sx + slot * 0.5 - 2.0,
                    y + slot * 0.5 - 2.0,
                    4.0,
                    4.0,
                    [48, 52, 60],
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn panel(
        &mut self,
        width: f32,
        height: f32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        fill: f32,
        color: [u8; 3],
    ) {
        self.quad(width, height, x, y, w, h, [12, 12, 16]);
        self.quad(
            width,
            height,
            x + 2.0,
            y + 2.0,
            (w - 4.0) * fill.clamp(0.0, 1.0),
            h - 4.0,
            color,
        );
    }

    /// OSRS interface panel background. Uses the scraped cache panel sprite
    /// as a nine-slice, with the old flat quads as a fallback.
    pub fn hud_panel(&mut self, width: f32, height: f32, x: f32, y: f32, w: f32, h: f32) {
        if let Some(uv) = self.atlas.ui_uv("panel") {
            self.panel_sprite(width, height, x, y, w, h, uv, 12.0, [56, 53, 48]);
        } else {
            self.quad(width, height, x, y, w, h, [20, 20, 24]);
            self.quad(width, height, x, y, w, 1.0, [82, 76, 58]);
            self.quad(width, height, x, y + h - 1.0, w, 1.0, [82, 76, 58]);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn sprite(
        &mut self,
        width: f32,
        height: f32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        uv: [f32; 4],
        tint: [u8; 3],
        alpha: f32,
    ) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let x0 = 2.0 * x / width - 1.0;
        let x1 = 2.0 * (x + w) / width - 1.0;
        let y0 = 1.0 - 2.0 * y / height;
        let y1 = 1.0 - 2.0 * (y + h) / height;
        let color = [
            srgb_to_linear(tint[0] as f32 / 255.0),
            srgb_to_linear(tint[1] as f32 / 255.0),
            srgb_to_linear(tint[2] as f32 / 255.0),
            alpha,
        ];
        let base = self.vertices.len() as u32;
        self.vertices.push(UiVertex {
            position: [x0, y0, 0.0],
            uv: [uv[0], uv[1]],
            color,
        });
        self.vertices.push(UiVertex {
            position: [x1, y0, 0.0],
            uv: [uv[2], uv[1]],
            color,
        });
        self.vertices.push(UiVertex {
            position: [x0, y1, 0.0],
            uv: [uv[0], uv[3]],
            color,
        });
        self.vertices.push(UiVertex {
            position: [x1, y1, 0.0],
            uv: [uv[2], uv[3]],
            color,
        });
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
    }

    /// Draw a real OSRS interface panel sprite as a nine-slice so the
    /// carved border stays crisp at any size.
    #[allow(clippy::too_many_arguments)]
    pub fn panel_sprite(
        &mut self,
        width: f32,
        height: f32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        uv: [f32; 4],
        border: f32,
        tint: [u8; 3],
    ) {
        let (u0, v0, u1, v1) = (uv[0], uv[1], uv[2], uv[3]);
        let su = (u1 - u0) / 3.0;
        let sv = (v1 - v0) / 3.0;
        let bw = border.min(w * 0.45);
        let bh = border.min(h * 0.45);
        // 3x3 grid of quads; the middle stretches.
        let xs = [0.0, bw, w - bw, w];
        let ys = [0.0, bh, h - bh, h];
        for row in 0..3 {
            for col in 0..3 {
                let qx = x + xs[col];
                let qy = y + ys[row];
                let qw = xs[col + 1] - xs[col];
                let qh = ys[row + 1] - ys[row];
                if qw <= 0.0 || qh <= 0.0 {
                    continue;
                }
                let qu = [
                    u0 + su * col as f32,
                    v0 + sv * row as f32,
                    u0 + su * (col + 1) as f32,
                    v0 + sv * (row + 1) as f32,
                ];
                self.sprite(width, height, qx, qy, qw, qh, qu, tint, 0.93);
            }
        }
    }

    /// Draw a small OSRS icon at a screen position.
    #[allow(clippy::too_many_arguments)]
    pub fn icon(
        &mut self,
        width: f32,
        height: f32,
        x: f32,
        y: f32,
        size: f32,
        key: &str,
        tint: [u8; 3],
    ) {
        if let Some(uv) = self.atlas.ui_uv(key) {
            self.sprite(width, height, x, y, size, size, uv, tint, 1.0);
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Draw a filled pie wedge for ability cooldowns. `fraction` is the
    /// remaining cooldown (1 = just cast, 0 = ready), sweeping clockwise from
    /// the top like a clock face.
    #[allow(clippy::too_many_arguments)]
    fn cooldown_wedge(
        &mut self,
        width: f32,
        height: f32,
        cx: f32,
        cy: f32,
        radius: f32,
        fraction: f32,
        color: [u8; 3],
        alpha: f32,
    ) {
        let fraction = fraction.clamp(0.0, 1.0);
        if fraction <= 0.001 || radius <= 0.0 {
            return;
        }
        let segments = 40usize;
        let start = -std::f32::consts::FRAC_PI_2;
        let sweep = std::f32::consts::TAU * fraction;
        let uv = self.atlas.white_uv;
        let uv_center = [(uv[0] + uv[2]) * 0.5, (uv[1] + uv[3]) * 0.5];
        let tint = [
            srgb_to_linear(color[0] as f32 / 255.0),
            srgb_to_linear(color[1] as f32 / 255.0),
            srgb_to_linear(color[2] as f32 / 255.0),
            alpha,
        ];
        let to_clip = |x: f32, y: f32| [2.0 * x / width - 1.0, 1.0 - 2.0 * y / height, 0.0];
        let base = self.vertices.len() as u32;
        self.vertices.push(UiVertex {
            position: to_clip(cx, cy),
            uv: uv_center,
            color: tint,
        });
        for i in 0..=segments {
            let angle = start + sweep * i as f32 / segments as f32;
            self.vertices.push(UiVertex {
                position: to_clip(cx + angle.cos() * radius, cy + angle.sin() * radius),
                uv: uv_center,
                color: tint,
            });
        }
        for i in 0..segments {
            self.indices
                .extend_from_slice(&[base, base + 1 + i as u32, base + 2 + i as u32]);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn quad_alpha(
        &mut self,
        width: f32,
        height: f32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: [u8; 3],
        alpha: f32,
    ) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let x0 = 2.0 * x / width - 1.0;
        let x1 = 2.0 * (x + w) / width - 1.0;
        let y0 = 1.0 - 2.0 * y / height;
        let y1 = 1.0 - 2.0 * (y + h) / height;

        let base = self.vertices.len() as u32;
        let uv = self.atlas.white_uv;
        let rgba = [
            srgb_to_linear(color[0] as f32 / 255.0),
            srgb_to_linear(color[1] as f32 / 255.0),
            srgb_to_linear(color[2] as f32 / 255.0),
            alpha,
        ];
        self.vertices.push(UiVertex {
            position: [x0, y0, 0.0],
            uv: [uv[0], uv[1]],
            color: rgba,
        });
        self.vertices.push(UiVertex {
            position: [x1, y0, 0.0],
            uv: [uv[0], uv[1]],
            color: rgba,
        });
        self.vertices.push(UiVertex {
            position: [x0, y1, 0.0],
            uv: [uv[0], uv[1]],
            color: rgba,
        });
        self.vertices.push(UiVertex {
            position: [x1, y1, 0.0],
            uv: [uv[0], uv[1]],
            color: rgba,
        });
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
    }

    #[allow(clippy::too_many_arguments)]
    fn quad(&mut self, width: f32, height: f32, x: f32, y: f32, w: f32, h: f32, color: [u8; 3]) {
        self.quad_alpha(width, height, x, y, w, h, color, 1.0);
    }

    /// A filled circle, used for the OSRS-style prayer orb.
    fn orb(&mut self, width: f32, height: f32, cx: f32, cy: f32, radius: f32, color: [u8; 3]) {
        let segments = 18usize;
        let uv = self.atlas.white_uv;
        let rgba = [
            srgb_to_linear(color[0] as f32 / 255.0),
            srgb_to_linear(color[1] as f32 / 255.0),
            srgb_to_linear(color[2] as f32 / 255.0),
            1.0,
        ];
        let base = self.vertices.len() as u32;
        self.vertices.push(UiVertex {
            position: [2.0 * cx / width - 1.0, 1.0 - 2.0 * cy / height, 0.0],
            uv: [uv[0], uv[1]],
            color: rgba,
        });
        for step in 0..=segments {
            let angle = step as f32 / segments as f32 * std::f32::consts::TAU;
            let x = cx + angle.cos() * radius;
            let y = cy + angle.sin() * radius;
            self.vertices.push(UiVertex {
                position: [2.0 * x / width - 1.0, 1.0 - 2.0 * y / height, 0.0],
                uv: [uv[0], uv[1]],
                color: rgba,
            });
        }
        for step in 0..segments {
            self.indices
                .extend_from_slice(&[base, base + 1 + step as u32, base + 2 + step as u32]);
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Shared OSRS-style beveled slot used by abilities and inventory. One
    /// dark inset, a warm outer edge and a top-left highlight read as carved
    /// stone without needing a second sprite.
    fn slot_frame(&mut self, width: f32, height: f32, x: f32, y: f32, w: f32, h: f32) {
        self.quad(
            width,
            height,
            x - 3.0,
            y - 3.0,
            w + 6.0,
            h + 6.0,
            [8, 8, 10],
        );
        self.quad(
            width,
            height,
            x - 1.0,
            y - 1.0,
            w + 2.0,
            h + 2.0,
            [78, 68, 48],
        );
        self.quad(width, height, x, y, w, h, [17, 18, 22]);
        self.quad(width, height, x, y, w, 1.0, [108, 96, 68]);
        self.quad(width, height, x, y, 1.0, h, [90, 80, 58]);
        self.quad(width, height, x, y + h - 1.0, w, 1.0, [10, 10, 12]);
        self.quad(width, height, x + w - 1.0, y, 1.0, h, [10, 10, 12]);
    }

    /// Thin gold rule under a card title.
    fn card_rule(&mut self, width: f32, height: f32, x: f32, y: f32, w: f32) {
        self.quad(
            width,
            height,
            x + 8.0,
            y + 20.0,
            w - 16.0,
            1.0,
            [86, 74, 50],
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn rect_outline(
        &mut self,
        width: f32,
        height: f32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        thickness: f32,
        color: [u8; 3],
        alpha: f32,
    ) {
        self.quad_alpha(width, height, x, y, w, thickness, color, alpha);
        self.quad_alpha(
            width,
            height,
            x,
            y + h - thickness,
            w,
            thickness,
            color,
            alpha,
        );
        self.quad_alpha(width, height, x, y, thickness, h, color, alpha);
        self.quad_alpha(
            width,
            height,
            x + w - thickness,
            y,
            thickness,
            h,
            color,
            alpha,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimap_world_maps_corners_and_center() {
        let bounds = (Vec2::new(0.0, 0.0), Vec2::new(1000.0, 2000.0));
        let (x, y, size, _) = minimap_rect(1280.0, 720.0);
        let (cx, cz) = minimap_world_at(
            1280.0,
            720.0,
            ((x + size * 0.5) as f64, (y + size * 0.5) as f64),
            bounds,
        )
        .expect("center inside minimap");
        assert!((cx - 500.0).abs() < 0.01);
        assert!((cz - 1000.0).abs() < 0.01);

        let (lx, lz) = minimap_world_at(1280.0, 720.0, (x as f64, (y + size) as f64), bounds)
            .expect("bottom-left inside minimap");
        assert!(lx.abs() < 0.01);
        assert!(lz.abs() < 0.01);

        let (rx, rz) = minimap_world_at(1280.0, 720.0, ((x + size) as f64, y as f64), bounds)
            .expect("top-right inside minimap");
        assert!((rx - 1000.0).abs() < 0.01);
        assert!((rz - 2000.0).abs() < 0.01);
    }

    #[test]
    fn minimap_world_rejects_clicks_outside() {
        let bounds = (Vec2::ZERO, Vec2::ONE);
        assert!(minimap_world_at(1280.0, 720.0, (10.0, 10.0), bounds).is_none());
    }

    #[test]
    fn overhead_name_colours_separate_teams_and_camps() {
        let ally = unit_name_color(0, true, false, None);
        let enemy = unit_name_color(1, true, false, None);
        let might = unit_name_color(2, false, true, Some(CampBuff::Might));
        let swift = unit_name_color(2, false, true, Some(CampBuff::Swift));
        let neutral = unit_name_color(2, false, true, None);
        assert!(ally[2] > ally[0], "allies read blue");
        assert!(enemy[0] > enemy[2], "enemies read red");
        assert!(might[0] > might[2], "Might camps read warm");
        assert!(swift[1] > swift[0], "Swift camps read green");
        assert_ne!(neutral, might);
        assert_ne!(neutral, swift);
    }
}
