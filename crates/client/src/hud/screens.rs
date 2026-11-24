//! Front-end screens: the main menu and the hero-select spotlight.
//!
//! The look borrows from the early RuneScape front end: a brazier of pixel
//! flames on each side of a gold title, carved stone buttons on an OSRS
//! panel, and pre-rendered-style character art lit warm from one side.

use super::{Hud, OSRS_YELLOW, UiVertex};
use crate::hero_select::{self, HeroSelect, PICK_SECONDS};
use crate::menu::{self, BUTTONS, MainMenu, MenuAction};
use crate::scene::srgb_to_linear;

const GOLD: [u8; 3] = [245, 214, 137];
const STONE: [u8; 3] = [58, 52, 44];
const STONE_HOVER: [u8; 3] = [86, 74, 54];
const BEVEL_LIGHT: [u8; 3] = [124, 110, 84];
const BEVEL_DARK: [u8; 3] = [18, 16, 12];
const MUTED: [u8; 3] = [178, 172, 158];

fn linear(color: [u8; 3], alpha: f32) -> [f32; 4] {
    [
        srgb_to_linear(color[0] as f32 / 255.0),
        srgb_to_linear(color[1] as f32 / 255.0),
        srgb_to_linear(color[2] as f32 / 255.0),
        alpha,
    ]
}

impl Hud {
    /// Push one untextured vertex at window pixel `(x, y)`.
    fn plain_vertex(&mut self, width: f32, height: f32, x: f32, y: f32, color: [f32; 4]) -> u32 {
        let uv = self.atlas.white_uv;
        let index = self.vertices.len() as u32;
        self.vertices.push(UiVertex {
            position: [2.0 * x / width - 1.0, 1.0 - 2.0 * y / height, 0.0],
            uv: [(uv[0] + uv[2]) * 0.5, (uv[1] + uv[3]) * 0.5],
            color,
        });
        index
    }

    /// A filled ellipse that fades from `alpha` at the centre to clear at
    /// the rim (stage floors and glows).
    #[allow(clippy::too_many_arguments)]
    fn soft_ellipse(
        &mut self,
        width: f32,
        height: f32,
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        color: [u8; 3],
        alpha: f32,
    ) {
        let segments = 40;
        let centre = self.plain_vertex(width, height, cx, cy, linear(color, alpha));
        let rim = linear(color, 0.0);
        let first = self.vertices.len() as u32;
        for i in 0..=segments {
            let a = std::f32::consts::TAU * i as f32 / segments as f32;
            self.plain_vertex(width, height, cx + a.cos() * rx, cy + a.sin() * ry, rim);
        }
        for i in 0..segments {
            self.indices
                .extend_from_slice(&[centre, first + i, first + i + 1]);
        }
    }

    /// A trapezoid with a vertical alpha gradient: a light beam.
    #[allow(clippy::too_many_arguments)]
    fn beam(
        &mut self,
        width: f32,
        height: f32,
        top: (f32, f32, f32),
        bottom: (f32, f32, f32),
        color: [u8; 3],
        alpha_top: f32,
        alpha_bottom: f32,
    ) {
        let (tx0, tx1, ty) = top;
        let (bx0, bx1, by) = bottom;
        let a = self.plain_vertex(width, height, tx0, ty, linear(color, alpha_top));
        let b = self.plain_vertex(width, height, tx1, ty, linear(color, alpha_top));
        let c = self.plain_vertex(width, height, bx0, by, linear(color, alpha_bottom));
        let d = self.plain_vertex(width, height, bx1, by, linear(color, alpha_bottom));
        self.indices.extend_from_slice(&[a, b, c, c, b, d]);
    }

    /// A carved stone button: bevelled edges, gold rim when hot.
    #[allow(clippy::too_many_arguments)]
    fn stone_button(
        &mut self,
        width: f32,
        height: f32,
        rect: (f32, f32, f32, f32),
        hot: bool,
        glow: f32,
    ) {
        let (x, y, w, h) = rect;
        if hot {
            self.quad_alpha(
                width,
                height,
                x - 4.0,
                y - 4.0,
                w + 8.0,
                h + 8.0,
                [236, 168, 64],
                0.18 + 0.12 * glow,
            );
        }
        self.quad(
            width,
            height,
            x,
            y,
            w,
            h,
            if hot { STONE_HOVER } else { STONE },
        );
        self.quad(width, height, x, y, w, 2.0, BEVEL_LIGHT);
        self.quad(width, height, x, y, 2.0, h, BEVEL_LIGHT);
        self.quad(width, height, x, y + h - 2.0, w, 2.0, BEVEL_DARK);
        self.quad(width, height, x + w - 2.0, y, 2.0, h, BEVEL_DARK);
        if hot {
            self.rect_outline(
                width,
                height,
                x - 1.0,
                y - 1.0,
                w + 2.0,
                h + 2.0,
                1.0,
                GOLD,
                0.9,
            );
        }
    }

    /// A brazier of rising pixel flames, centred at `(cx, base)`.
    fn brazier(&mut self, width: f32, height: f32, cx: f32, base: f32, time: f32, seed: u32) {
        let scale = (height / 720.0).clamp(0.8, 1.6);
        // The bowl and its stand.
        self.quad(
            width,
            height,
            cx - 6.0 * scale,
            base,
            12.0 * scale,
            46.0 * scale,
            [34, 30, 26],
        );
        self.quad(
            width,
            height,
            cx - 26.0 * scale,
            base - 6.0 * scale,
            52.0 * scale,
            14.0 * scale,
            [52, 46, 40],
        );
        self.quad(
            width,
            height,
            cx - 26.0 * scale,
            base - 6.0 * scale,
            52.0 * scale,
            2.0,
            [104, 92, 70],
        );
        self.soft_ellipse(
            width,
            height,
            cx,
            base - 30.0 * scale,
            70.0 * scale,
            60.0 * scale,
            [255, 150, 50],
            0.20,
        );
        let count = 70u32;
        for i in 0..count {
            let life = 0.7 + menu::hash01(seed, i) * 0.6;
            let t = ((time + menu::hash01(i, seed ^ 0xA5) * life) % life) / life;
            let spread = (menu::hash01(seed ^ 7, i) - 0.5) * 34.0 * scale;
            let sway = (time * 5.0 + i as f32).sin() * 4.0 * scale * t;
            let x = cx + spread * (1.0 - t * 0.6) + sway;
            let y = base - 8.0 * scale - t * 92.0 * scale;
            let size = (12.0 - t * 9.0) * scale;
            let color = if t < 0.25 {
                [255, 236, 140]
            } else if t < 0.55 {
                [255, 168, 48]
            } else {
                [214, 64, 22]
            };
            let alpha = (1.0 - t).powf(0.8) * 0.9;
            self.quad_alpha(
                width,
                height,
                x - size * 0.5,
                y - size * 0.5,
                size,
                size,
                color,
                alpha,
            );
        }
    }

    /// The main menu over the slowly orbiting arena.
    pub fn build_main_menu(&mut self, width: f32, height: f32, menu: &MainMenu, version: &str) {
        self.vertices.clear();
        self.indices.clear();
        let t = menu.time;

        // Darken the world and frame it like an old login screen.
        self.quad_alpha(width, height, 0.0, 0.0, width, height, [6, 6, 10], 0.38);
        for i in 0..6 {
            let band = height * 0.05;
            let a = 0.55 - i as f32 * 0.09;
            self.quad_alpha(
                width,
                height,
                0.0,
                i as f32 * band,
                width,
                band,
                [0, 0, 0],
                a,
            );
            self.quad_alpha(
                width,
                height,
                0.0,
                height - (i + 1) as f32 * band,
                width,
                band,
                [0, 0, 0],
                a,
            );
        }

        // Title with a flickering firelight glow and a brazier either side.
        let title_scale = (height / 720.0 * 4.6).clamp(3.4, 7.0);
        let title_y = height * 0.06;
        let flicker = 0.75 + 0.25 * ((t * 7.3).sin() * 0.5 + (t * 3.1).sin() * 0.5);
        for (dx, dy) in [(-3.0, 0.0), (3.0, 0.0), (0.0, -3.0), (0.0, 3.0)] {
            let w = self.text_width("MOBASCAPE", title_scale);
            let x = (width - w) * 0.5 + dx;
            self.text(
                width,
                height,
                x,
                title_y + dy,
                title_scale,
                "MOBASCAPE",
                [(170.0 * flicker) as u8, (64.0 * flicker) as u8, 12],
            );
        }
        self.text_centered(
            width,
            height,
            width * 0.5,
            title_y,
            title_scale,
            "MOBASCAPE",
            GOLD,
        );
        let title_w = self.text_width("MOBASCAPE", title_scale);
        let title_h = title_scale * 15.5;
        self.text_centered(
            width,
            height,
            width * 0.5,
            title_y + title_h + 8.0,
            1.1,
            "5V5 HERO ARENA",
            [222, 196, 150],
        );
        let side = title_w * 0.5 + 70.0 * (height / 720.0).clamp(0.8, 1.6);
        let base = title_y + title_h + 22.0;
        self.brazier(width, height, width * 0.5 - side, base, t, 11);
        self.brazier(width, height, width * 0.5 + side, base, t + 0.37, 29);

        // The stone tablet and its buttons.
        let (tx, ty, tw, th) = menu::tablet_rect(width, height);
        self.hud_panel(width, height, tx, ty, tw, th);
        self.text_centered(
            width,
            height,
            tx + tw * 0.5,
            ty + 18.0,
            1.15,
            "SELECT A GAME MODE",
            OSRS_YELLOW,
        );
        self.quad_alpha(
            width,
            height,
            tx + 40.0,
            ty + 40.0,
            tw - 80.0,
            1.0,
            [188, 154, 86],
            0.6,
        );
        let glow = (t * 3.0).sin() * 0.5 + 0.5;
        for (index, rect) in menu::button_rects(width, height).into_iter().enumerate() {
            let hot = menu.hover == Some(index) || (menu.hover.is_none() && menu.focus == index);
            self.stone_button(width, height, rect, hot, glow);
            let (x, y, w, h) = rect;
            match BUTTONS[index] {
                MenuAction::Play(mode) => {
                    let label = if hot { GOLD } else { [232, 224, 204] };
                    self.text(width, height, x + 16.0, y + 8.0, 1.3, mode.label(), label);
                    self.text(
                        width,
                        height,
                        x + 16.0,
                        y + h - 19.0,
                        0.74,
                        mode.blurb(),
                        MUTED,
                    );
                    if mode.needs_server() {
                        let tag = "OFFLINE";
                        let tw = self.text_width(tag, 0.72) + 10.0;
                        self.quad_alpha(
                            width,
                            height,
                            x + w - tw - 10.0,
                            y + 9.0,
                            tw,
                            15.0,
                            [96, 34, 24],
                            0.9,
                        );
                        self.text(
                            width,
                            height,
                            x + w - tw - 5.0,
                            y + 11.0,
                            0.72,
                            tag,
                            [255, 170, 120],
                        );
                    }
                }
                MenuAction::Quit => {
                    self.text_centered(
                        width,
                        height,
                        x + w * 0.5,
                        y + h * 0.5 - 7.0,
                        1.0,
                        "QUIT",
                        if hot { GOLD } else { MUTED },
                    );
                }
            }
        }

        // Footer.
        self.text_centered(
            width,
            height,
            width * 0.5,
            height - 40.0,
            0.8,
            "ARROW KEYS + ENTER OR CLICK   //   ESC TO QUIT",
            [150, 146, 134],
        );
        self.text_centered(
            width,
            height,
            width * 0.5,
            height - 22.0,
            0.72,
            &format!("MOBASCAPE PRE-ALPHA {version}   -   NOT AFFILIATED WITH JAGEX"),
            [110, 106, 98],
        );
    }

    /// Full-screen champion picker: grouped roster, animated spotlight,
    /// countdown and ability detail.
    pub fn build_hero_select(
        &mut self,
        width: f32,
        height: f32,
        select: &HeroSelect,
        mouse: (f64, f64),
    ) {
        self.vertices.clear();
        self.indices.clear();
        self.quad_alpha(width, height, 0.0, 0.0, width, height, [7, 9, 13], 0.80);

        // Title and mode.
        self.text_centered(
            width,
            height,
            width * 0.5,
            height * 0.028,
            2.6,
            "CHOOSE YOUR HERO",
            GOLD,
        );
        let mode = select.mode;
        let chip = mode.label();
        let cw = self.text_width(chip, 0.95) + 18.0;
        self.quad_alpha(width, height, 14.0, 14.0, cw, 22.0, [74, 58, 34], 0.92);
        self.rect_outline(width, height, 14.0, 14.0, cw, 22.0, 1.0, GOLD, 0.8);
        self.text(width, height, 23.0, 18.0, 0.95, chip, [255, 236, 196]);
        if mode.needs_server() {
            self.text(
                width,
                height,
                14.0,
                42.0,
                0.74,
                "SERVERS OFFLINE: BOTS FILL EMPTY SLOTS",
                [255, 170, 120],
            );
        }

        self.countdown_dial(width, height, select);

        // Left column: type tabs and the grouped roster.
        let (lx, ly, lw, lh) = hero_select::roster_column(width, height);
        self.hud_panel(width, height, lx, ly, lw, lh);
        for (index, &(x, y, w, h)) in hero_select::filter_rects(width, height).iter().enumerate() {
            let active = index == select.filter;
            self.quad(
                width,
                height,
                x,
                y,
                w,
                h,
                if active { [150, 62, 62] } else { [38, 38, 44] },
            );
            self.rect_outline(
                width,
                height,
                x,
                y,
                w,
                h,
                1.0,
                if active { [236, 190, 96] } else { [86, 82, 74] },
                0.85,
            );
            self.text_centered(
                width,
                height,
                x + w * 0.5,
                y + h * 0.5 - 7.0,
                0.9,
                hero_select::FILTERS[index],
                if active {
                    [255, 235, 190]
                } else {
                    [176, 176, 172]
                },
            );
        }
        let classes = select.visible_classes();
        let (cards, headers) = hero_select::roster_layout(width, height, &classes);
        for (label, y) in &headers {
            let color = Self::class_color(label);
            self.text(width, height, lx + 10.0, y + 3.0, 0.9, label, color);
            let tw = self.text_width(label, 0.9);
            self.quad_alpha(
                width,
                height,
                lx + 18.0 + tw,
                y + 10.0,
                lw - tw - 30.0,
                1.0,
                color,
                0.45,
            );
        }
        for (slot, &(x, y, cw, ch)) in cards.iter().enumerate() {
            let Some(&index) = select.visible.get(slot) else {
                continue;
            };
            let Some(champion) = select.champions.get(index) else {
                continue;
            };
            let selected = index == select.selected;
            let hovered = select.hover == Some(slot);
            if selected {
                let pulse = 0.20 + 0.10 * (select.time * 2.6).sin().abs();
                self.quad_alpha(
                    width,
                    height,
                    x - 3.0,
                    y - 3.0,
                    cw + 6.0,
                    ch + 6.0,
                    [236, 188, 92],
                    pulse,
                );
            }
            self.quad(
                width,
                height,
                x,
                y,
                cw,
                ch,
                if selected { [74, 58, 34] } else { [32, 32, 38] },
            );
            self.rect_outline(
                width,
                height,
                x + 1.0,
                y + 1.0,
                cw - 2.0,
                ch - 2.0,
                if selected { 2.0 } else { 1.0 },
                if selected {
                    [244, 206, 118]
                } else if hovered {
                    [186, 182, 166]
                } else {
                    [70, 66, 60]
                },
                0.9,
            );
            let portrait = cw - 8.0;
            self.quad_alpha(
                width,
                height,
                x + 4.0,
                y + 4.0,
                portrait,
                portrait,
                [18, 21, 28],
                0.95,
            );
            if let Some(uv) = self.atlas.hero_uv(&champion.id) {
                self.sprite(
                    width,
                    height,
                    x + 4.0,
                    y + 4.0,
                    portrait,
                    portrait,
                    uv,
                    [255, 255, 255],
                    1.0,
                );
            }
            let chars = ((cw / 7.0) as usize).max(4);
            let name: String = champion.name.chars().take(chars).collect();
            self.text_centered(
                width,
                height,
                x + cw * 0.5,
                y + portrait + 6.0,
                0.68,
                &name,
                [238, 232, 214],
            );
            // Taken by a bot: dimmed, stamped, and not selectable.
            if select.is_picked(index) {
                self.quad_alpha(width, height, x, y, cw, ch, [6, 7, 10], 0.68);
                let band = 16.0;
                let by = y + (portrait - band) * 0.5 + 4.0;
                self.quad_alpha(width, height, x, by, cw, band, [96, 30, 30], 0.92);
                self.text_centered(
                    width,
                    height,
                    x + cw * 0.5,
                    by + 2.0,
                    0.7,
                    "PICKED",
                    [255, 214, 196],
                );
            }
        }

        self.spotlight_stage(width, height, select, mouse);
        self.team_columns(width, height, select);
        self.hero_detail(width, height, select);
    }

    /// Both teams' rows down the edges of the stage: portrait once a hero
    /// is locked, a pulsing "..." while still choosing, your row showing
    /// the highlighted hero.
    fn team_columns(&mut self, width: f32, height: f32, select: &HeroSelect) {
        use hero_select::Team;
        for (team, slots, tint, label) in [
            (Team::Blue, &select.blue, [88, 150, 255], "YOUR TEAM"),
            (Team::Red, &select.red, [236, 96, 96], "ENEMY"),
        ] {
            let rects = hero_select::team_slot_rects(width, height, team);
            if let Some(&(x, y, w, _)) = rects.first() {
                self.text_centered(width, height, x + w * 0.5, y - 17.0, 0.66, label, tint);
            }
            for (row, (&(x, y, w, h), slot)) in rects.iter().zip(slots.iter()).enumerate() {
                let you = team == Team::Blue && row == 0;
                let shown = if you {
                    Some(select.selected)
                } else {
                    slot.champion
                };
                self.quad_alpha(width, height, x, y, w, h, [14, 16, 22], 0.9);
                self.quad_alpha(width, height, x, y, 3.0, h, tint, 0.95);
                self.rect_outline(
                    width,
                    height,
                    x,
                    y,
                    w,
                    h,
                    1.0,
                    if you { GOLD } else { [70, 66, 60] },
                    0.9,
                );
                let side = (w - 8.0).min(h - 16.0).max(8.0);
                let px = x + (w - side) * 0.5 + 1.0;
                let py = y + 3.0;
                match shown.and_then(|i| select.champions.get(i)) {
                    Some(champion) => {
                        if let Some(uv) = self.atlas.hero_uv(&champion.id) {
                            // Your row is still a choice until you lock in.
                            let alpha = if you {
                                0.65 + 0.25 * (select.time * 3.0).sin().abs()
                            } else {
                                1.0
                            };
                            self.sprite(
                                width,
                                height,
                                px,
                                py,
                                side,
                                side,
                                uv,
                                [255, 255, 255],
                                alpha,
                            );
                        }
                    }
                    None => {
                        let dots = 1 + ((select.time * 2.0) as usize + row) % 3;
                        self.text_centered(
                            width,
                            height,
                            x + w * 0.5,
                            py + side * 0.5 - 6.0,
                            0.9,
                            &".".repeat(dots),
                            [150, 146, 136],
                        );
                    }
                }
                self.text_centered(
                    width,
                    height,
                    x + w * 0.5,
                    y + h - 12.0,
                    0.58,
                    &slot.name,
                    if you { GOLD } else { [200, 196, 184] },
                );
            }
        }
    }

    /// The pick timer: a draining gold ring with the seconds left, red and
    /// pulsing in the last five seconds.
    fn countdown_dial(&mut self, width: f32, height: f32, select: &HeroSelect) {
        let (x, y, w, h) = hero_select::timer_rect(width, height);
        let (cx, cy, r) = (x + w * 0.5, y + h * 0.5, w * 0.5);
        let left = select.seconds_left();
        let urgent = left <= 5;
        let pulse = if urgent {
            0.5 + 0.5 * (select.countdown * std::f32::consts::TAU).cos()
        } else {
            0.0
        };
        self.cooldown_wedge(width, height, cx, cy, r + 3.0, 1.0, [10, 10, 14], 0.92);
        let ring = if urgent {
            [230, 70, 50]
        } else {
            [236, 190, 96]
        };
        self.cooldown_wedge(
            width,
            height,
            cx,
            cy,
            r,
            select.countdown / PICK_SECONDS,
            ring,
            0.85,
        );
        self.cooldown_wedge(width, height, cx, cy, r - 6.0, 1.0, [16, 16, 20], 1.0);
        let scale = 1.9 + pulse * 0.4;
        let text = left.to_string();
        let th = 13.0 * scale;
        self.text_centered(
            width,
            height,
            cx,
            cy - th * 0.5,
            scale,
            &text,
            if urgent {
                [255, 120, 100]
            } else {
                [255, 240, 205]
            },
        );
        self.text_centered(
            width,
            height,
            cx,
            y + h + 4.0,
            0.74,
            "SECONDS TO PICK",
            MUTED,
        );
    }

    /// The centre stage: a light beam, a floor glow and the animated hero.
    fn spotlight_stage(&mut self, width: f32, height: f32, select: &HeroSelect, mouse: (f64, f64)) {
        let (sx, sy, sw, sh) = hero_select::spotlight_rect(width, height);
        let Some(champion) = select.selected_champion() else {
            return;
        };
        let class = Self::class_color(&champion.class);
        let cx = sx + sw * 0.5;
        let floor = sy + sh * 0.93;
        // Beam from above, floor pool, class-coloured rim light.
        self.beam(
            width,
            height,
            (cx - sw * 0.10, cx + sw * 0.10, sy - 30.0),
            (cx - sw * 0.38, cx + sw * 0.38, floor),
            [255, 236, 190],
            0.16,
            0.03,
        );
        self.soft_ellipse(
            width,
            height,
            cx,
            floor,
            sw * 0.40,
            sh * 0.07,
            [255, 226, 160],
            0.30,
        );
        self.soft_ellipse(width, height, cx, floor, sw * 0.26, sh * 0.045, class, 0.35);
        // The hero, fitted to the stage at the slot's 3:4 aspect, feet on
        // the floor line (the slot puts them at 90% of its height).
        let aspect = 384.0 / 512.0;
        let mut h = sh * 1.02;
        let mut w = h * aspect;
        if w > sw {
            w = sw;
            h = w / aspect;
        }
        let top = floor - h * 0.90;
        let uv = self.atlas.spotlight_uv();
        self.sprite(
            width,
            height,
            cx - w * 0.5,
            top,
            w,
            h,
            uv,
            [255, 255, 255],
            1.0,
        );

        // Name plate and lock in.
        let name_y = sy + sh + 6.0;
        self.text_centered(
            width,
            height,
            cx,
            name_y,
            1.9,
            &champion.name,
            [248, 236, 204],
        );
        self.text_centered(
            width,
            height,
            cx,
            name_y + 30.0,
            0.95,
            &format!("{}  -  {}", champion.role, champion.class),
            class,
        );
        let columns = (sw / 7.0) as usize;
        if let Some(line) = hero_select::wrap_text(&champion.tagline, columns.max(20)).first() {
            self.text_centered(width, height, cx, name_y + 48.0, 0.74, line, MUTED);
        }
        let rect = hero_select::confirm_rect(width, height);
        let hot = hero_select::confirm_at(width, height, mouse);
        let (bx, by, bw, bh) = rect;
        self.quad(
            width,
            height,
            bx,
            by,
            bw,
            bh,
            if hot { [190, 60, 60] } else { [140, 44, 44] },
        );
        self.rect_outline(width, height, bx, by, bw, bh, 1.0, [236, 190, 96], 0.9);
        self.text_centered(
            width,
            height,
            bx + bw * 0.5,
            by + bh * 0.5 - 8.0,
            1.15,
            "LOCK IN",
            [255, 238, 206],
        );
    }

    /// Right column: stats, weapon and the four abilities.
    fn hero_detail(&mut self, width: f32, height: f32, select: &HeroSelect) {
        let (dx, dy, dw, dh) = hero_select::detail_rect(width, height);
        self.hud_panel(width, height, dx, dy, dw, dh);
        let Some(champion) = select.selected_champion() else {
            return;
        };
        self.text(
            width,
            height,
            dx + 12.0,
            dy + 10.0,
            1.0,
            "HERO STATS",
            OSRS_YELLOW,
        );
        let range = if champion.ranged_auto {
            "RANGED"
        } else {
            "MELEE"
        };
        let stats = [
            ("HEALTH", format!("{:.0}", champion.health)),
            ("MANA", format!("{:.0}", champion.mana)),
            ("DAMAGE", format!("{:.0}", champion.damage)),
            ("ARMOUR", format!("{:.0}", champion.armor)),
            ("SPEED", format!("{:.0}", champion.speed)),
            ("ATTACK", range.to_string()),
        ];
        let col_w = (dw - 24.0) / 3.0;
        for (i, (label, value)) in stats.iter().enumerate() {
            let x = dx + 12.0 + (i % 3) as f32 * col_w;
            let y = dy + 30.0 + (i / 3) as f32 * 32.0;
            self.text(width, height, x, y, 0.72, label, [160, 158, 150]);
            self.text(width, height, x, y + 13.0, 0.95, value, [236, 236, 232]);
        }
        let mut row_y = dy + 98.0;
        if let Some(weapon) = champion.weapon.as_deref() {
            let label = weapon.replace('_', " ").to_ascii_uppercase();
            self.text(
                width,
                height,
                dx + 12.0,
                row_y,
                0.78,
                &format!("WIELDS: {label}"),
                [214, 190, 140],
            );
            row_y += 18.0;
        }
        self.quad_alpha(
            width,
            height,
            dx + 12.0,
            row_y,
            dw - 24.0,
            1.0,
            [188, 154, 86],
            0.5,
        );
        row_y += 8.0;
        self.text(
            width,
            height,
            dx + 12.0,
            row_y,
            1.0,
            "ABILITIES",
            OSRS_YELLOW,
        );
        row_y += 20.0;
        let icon = 30.0;
        let columns = ((dw - icon - 36.0) / 6.6).max(18.0) as usize;
        let room = (dy + dh - row_y - 8.0) / champion.abilities.len().max(1) as f32;
        let lines = if room >= 64.0 { 3 } else { 2 };
        for (slot, ability) in champion.abilities.iter().enumerate() {
            let role = crate::ui_live::ability_icon_role(&champion.name, slot);
            self.quad_alpha(
                width,
                height,
                dx + 12.0,
                row_y,
                icon,
                icon,
                [8, 10, 14],
                0.9,
            );
            if let Some(uv) = self.atlas.ui_uv(&role) {
                self.sprite(
                    width,
                    height,
                    dx + 13.0,
                    row_y + 1.0,
                    icon - 2.0,
                    icon - 2.0,
                    uv,
                    [255, 255, 255],
                    1.0,
                );
            }
            self.text_centered(
                width,
                height,
                dx + 12.0 + icon * 0.5,
                row_y + icon + 2.0,
                0.72,
                &ability.key,
                [246, 224, 170],
            );
            let tx = dx + 20.0 + icon;
            self.text(
                width,
                height,
                tx,
                row_y,
                0.9,
                &ability.name,
                [232, 220, 186],
            );
            for (line, text) in hero_select::wrap_text(&ability.description, columns)
                .into_iter()
                .take(lines)
                .enumerate()
            {
                self.text(
                    width,
                    height,
                    tx,
                    row_y + 15.0 + line as f32 * 12.0,
                    0.72,
                    &text,
                    [186, 188, 196],
                );
            }
            row_y += room.clamp(46.0, 84.0);
        }
    }
}
