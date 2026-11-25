//! Front-end screens: the main menu and the hero-select spotlight.
//!
//! The look borrows from the early RuneScape front end: a brazier of pixel
//! flames on each side of a gold title, carved stone buttons on an OSRS
//! panel, and pre-rendered-style character art lit warm from one side.

use super::{Hud, OSRS_YELLOW, UiVertex};
use crate::hero_select::{self, HeroSelect};
use crate::items::ItemShop;
use crate::menu::{self, BUTTONS, MainMenu, MenuAction};
use crate::scene::srgb_to_linear;

/// The least height a passive or ability row gets on the detail
/// parchment: its key chip and one line of text.
const ROW_MIN: f32 = 38.0;

/// Ability icon size on the detail parchment.
const ICON: f32 = 28.0;

/// The least row height that fits the key chip with the icon under it.
const STACKED_ROW: f32 = 25.0 + ICON + 3.0;

/// A one-line row (key chip, small icon, name and cooldowns) for windows
/// too short to give every row a line of description.
const ROW_TIGHT: f32 = 24.0;

/// The popular build row: item icons, the rule under them and a gap.
const BUILD_H: f32 = ICON + 9.0;

/// A hover box: the rect it belongs to and its lines.
type Tooltip = ((f32, f32, f32, f32), Vec<String>);

/// One row on the detail parchment: the passive or an ability.
struct DetailRow {
    key: String,
    icon: Option<[f32; 4]>,
    name: String,
    /// Per-rank cooldowns ("6 / 5.5 / 5 / 4.5 s"); none for the passive.
    cooldowns: Option<String>,
    description: String,
}

const GOLD: [u8; 3] = [245, 214, 137];

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
            "5v5 Hero Arena",
            [222, 196, 150],
        );
        let side = title_w * 0.5 + 70.0 * (height / 720.0).clamp(0.8, 1.6);
        let base = title_y + title_h + 22.0;
        self.brazier(width, height, width * 0.5 - side, base, t, 11);
        self.brazier(width, height, width * 0.5 + side, base, t + 0.37, 29);

        // The stone tablet: a recessed row per game mode with a Play
        // button, and Quit.
        use super::kit::{ORANGE, STONE_QUIET, STONE_TEXT};
        let (tx, ty, tw, _) = menu::tablet_rect(width, height);
        let (_, _, _, th) = menu::tablet_rect(width, height);
        self.stone_frame(width, height, (tx, ty, tw, th));
        self.label_in(
            width,
            height,
            (tx, ty + 14.0, tw, 26.0),
            "b12",
            1.0,
            "Choose a game mode",
            ORANGE,
        );
        let glow = (t * 3.0).sin() * 0.5 + 0.5;
        for (index, rect) in menu::button_rects(width, height).into_iter().enumerate() {
            let hot = menu.hover == Some(index) || (menu.hover.is_none() && menu.focus == index);
            let (x, y, w, h) = rect;
            match BUTTONS[index] {
                MenuAction::Play(mode) => {
                    self.stone_inset(width, height, rect);
                    if hot {
                        self.quad_alpha(
                            width,
                            height,
                            x + 6.0,
                            y + 6.0,
                            w - 12.0,
                            h - 12.0,
                            [255, 214, 120],
                            0.06 + 0.05 * glow,
                        );
                        self.rect_outline(
                            width,
                            height,
                            x + 2.0,
                            y + 2.0,
                            w - 4.0,
                            h - 4.0,
                            1.0,
                            [236, 190, 96],
                            0.8,
                        );
                    }
                    let label = sentence_case(mode.label());
                    self.text_named(
                        "b12",
                        width,
                        height,
                        x + 18.0,
                        y + 12.0,
                        1.0,
                        &label,
                        if hot { OSRS_YELLOW } else { ORANGE },
                    );
                    self.text_named(
                        "p11",
                        width,
                        height,
                        x + 18.0,
                        y + h - 24.0,
                        1.0,
                        mode.blurb(),
                        STONE_QUIET,
                    );
                    let play = (x + w - 104.0, y + (h - 30.0) * 0.5, 88.0, 30.0);
                    self.osrs_button(width, height, play, hot, hot, false);
                    self.label_in(
                        width,
                        height,
                        play,
                        "p12",
                        1.0,
                        "Play",
                        if hot { [255, 255, 255] } else { ORANGE },
                    );
                    if mode.needs_server() {
                        let tag = "Offline";
                        let tw = self.text_width_named("p11", tag, 1.0);
                        self.text_named(
                            "p11",
                            width,
                            height,
                            play.0 - tw - 14.0,
                            y + 12.0,
                            1.0,
                            tag,
                            [255, 80, 60],
                        );
                    }
                }
                MenuAction::Quit => {
                    self.osrs_button(width, height, rect, false, hot, false);
                    self.label_in(
                        width,
                        height,
                        rect,
                        "p12",
                        1.0,
                        "Quit",
                        if hot { STONE_TEXT } else { ORANGE },
                    );
                }
            }
        }

        // Footer.
        let help = "Click a mode, or use the arrow keys and Enter. Esc quits.";
        self.label_in(
            width,
            height,
            (0.0, height - 46.0, width, 16.0),
            "p11",
            1.0,
            help,
            [176, 168, 150],
        );
        let foot = format!("MobaScape pre-alpha {version}  -  not affiliated with Jagex");
        self.label_in(
            width,
            height,
            (0.0, height - 26.0, width, 16.0),
            "p11",
            1.0,
            &foot,
            [130, 124, 112],
        );
    }

    /// Full-screen champion picker in OSRS stone windows: a title bar with
    /// the mode, the timer and Leave; both teams' columns; class tabs over
    /// the roster; the highlighted hero on parchment (the animated spotlight
    /// render, tags, stats and abilities with per-rank cooldowns); and the
    /// bottom bar with Lock in.
    pub fn build_hero_select(
        &mut self,
        width: f32,
        height: f32,
        select: &HeroSelect,
        shop: &ItemShop,
        mouse: (f64, f64),
    ) {
        use super::kit::{ORANGE, STONE_QUIET, STONE_TEXT};
        self.vertices.clear();
        self.indices.clear();
        // The dark gaps between the windows.
        self.quad(width, height, 0.0, 0.0, width, height, [36, 31, 25]);

        // Title bar: mode on the left, title and timer centred, Leave.
        let title = hero_select::title_rect(width, height);
        self.stone_frame(width, height, title);
        let (tx, ty, tw, _) = title;
        let mode = select.mode;
        self.text_named(
            "p12",
            width,
            height,
            tx + 22.0,
            ty + 14.0,
            1.0,
            "Game mode",
            STONE_QUIET,
        );
        let mode_line = format!("{}  -  5v5", sentence_case(mode.label()));
        self.text_named(
            "p12",
            width,
            height,
            tx + 22.0,
            ty + 34.0,
            1.0,
            &mode_line,
            [255, 214, 120],
        );
        if mode.needs_server() {
            self.text_named(
                "p11",
                width,
                height,
                tx + 22.0,
                ty + 52.0,
                1.0,
                "Servers offline: bots fill the empty slots.",
                [255, 150, 110],
            );
        }
        let heading = if select.is_locked() {
            "All heroes locked in".to_string()
        } else {
            format!("{}: choose your hero", sentence_case(mode.label()))
        };
        self.label_in(
            width,
            height,
            (tx, ty + 6.0, tw, 30.0),
            "b12",
            2.0,
            &heading,
            ORANGE,
        );
        if let Some(seconds) = select.starting_in() {
            self.starting_line(width, height, seconds);
        } else {
            self.pick_clock(width, height, select);
        }
        let leave = hero_select::leave_rect(width, height);

        let leave_hot = hero_select::leave_at(width, height, mouse);
        self.osrs_button(width, height, leave, false, leave_hot, false);
        self.label_in(
            width,
            height,
            leave,
            "p12",
            1.0,
            "Leave",
            if leave_hot { STONE_TEXT } else { ORANGE },
        );

        self.team_columns(width, height, select);

        // The tab bar over the roster: your lane call, then the compact
        // class filters at the right end.
        let tabs = hero_select::tabs_rect(width, height);
        self.stone_frame(width, height, tabs);
        let label_y = tabs.1 + tabs.3 * 0.5 - 7.0;
        self.text_named(
            "p12",
            width,
            height,
            tabs.0 + 22.0,
            label_y,
            1.0,
            "Your lane:",
            STONE_QUIET,
        );
        let mine = select.blue.first().and_then(|slot| slot.lane);
        let lane_rects = hero_select::lane_rects(width, height);
        let lane_labels = self.tabs_hold_labels(
            lane_rects[0].2,
            &hero_select::Lane::ALL.map(hero_select::Lane::label),
        );
        for (lane, &rect) in hero_select::Lane::ALL.iter().zip(lane_rects.iter()) {
            let hot = point_in(rect, mouse);
            self.icon_tab(
                width,
                height,
                rect,
                Some(lane.icon()),
                lane.label(),
                lane_labels,
                mine == Some(*lane),
                hot,
            );
        }
        self.text_named(
            "p12",
            width,
            height,
            hero_select::show_label_x(width, height),
            label_y,
            1.0,
            "Show:",
            STONE_QUIET,
        );
        for (index, &rect) in hero_select::filter_rects(width, height).iter().enumerate() {
            let icon = match index {
                1 => Some("stat_attack"),
                2 => Some("stat_ranged"),
                3 => Some("stat_magic"),
                _ => None,
            };
            let label = sentence_case(hero_select::FILTERS[index]);
            let hot = point_in(rect, mouse);
            // Compact tabs: the class icons alone (All keeps its word).
            self.icon_tab(
                width,
                height,
                rect,
                icon,
                &label,
                false,
                index == select.filter,
                hot,
            );
        }

        // The roster grid.
        let roster = hero_select::roster_column(width, height);
        self.stone_frame(width, height, roster);
        let classes = select.visible_classes();
        let (cards, _) = hero_select::roster_layout(width, height, &classes);
        let mut roster_tip: Option<Tooltip> = None;
        for (slot, &(x, y, cw, ch)) in cards.iter().enumerate() {
            let Some(&index) = select.visible.get(slot) else {
                continue;
            };
            let Some(champion) = select.champions.get(index) else {
                continue;
            };
            let selected = index == select.selected;
            let hovered = select.hover == Some(slot);
            let picked = select.is_picked(index);
            // Once everyone is locked in the roster dims around your pick.
            let dimmed = picked || (select.is_locked() && !selected);
            let side = cw;
            let tint = if dimmed {
                [110, 110, 110]
            } else {
                [255, 255, 255]
            };
            self.item_slot(width, height, (x, y, side, side), tint);
            if let Some(uv) = self.atlas.hero_uv(&champion.id) {
                self.sprite(
                    width,
                    height,
                    x + 3.0,
                    y + 3.0,
                    side - 6.0,
                    side - 6.0,
                    uv,
                    tint,
                    if dimmed { 0.55 } else { 1.0 },
                );
            }
            if selected {
                self.rect_outline(width, height, x, y, side, side, 2.0, OSRS_YELLOW, 1.0);
            } else if hovered && !picked {
                self.rect_outline(width, height, x, y, side, side, 1.0, [236, 226, 200], 0.9);
            }
            if picked {
                self.label_in(
                    width,
                    height,
                    (x, y + side * 0.5 - 10.0, side, 20.0),
                    "p11",
                    1.0,
                    "Picked",
                    [255, 60, 40],
                );
            }
            if ch - side < hero_select::CARD_LABEL * 0.5 {
                // Short window: no name line, the cursor names the card.
                if point_in((x, y, cw, ch), mouse) {
                    roster_tip = Some(((x, y, cw, ch), vec![champion.name.clone()]));
                }
                continue;
            }
            let chars = ((cw / 6.2) as usize).max(4);
            let name = clip_name(&champion.name, chars);
            self.label_in(
                width,
                height,
                (x - 4.0, y + side + 1.0, cw + 8.0, ch - side - 1.0),
                "p11",
                1.0,
                &name,
                if dimmed {
                    [140, 132, 118]
                } else if selected {
                    OSRS_YELLOW
                } else {
                    ORANGE
                },
            );
        }
        if let Some((anchor, text)) = roster_tip {
            let bounds = hero_select::roster_column(width, height);
            self.tooltip(width, height, anchor, bounds, &text);
        }

        self.hero_detail(width, height, select, shop, mouse);

        // Bottom bar: the hero's numbers, the note and Lock in.
        let bottom = hero_select::bottom_rect(width, height);
        self.stone_frame(width, height, bottom);
        if let Some(champion) = select.selected_champion() {
            let attack = if champion.class.eq_ignore_ascii_case("MAGIC") {
                "stat_magic"
            } else if champion.ranged_auto {
                "stat_ranged"
            } else {
                "stat_attack"
            };
            let stats = [
                (
                    "stat_hitpoints",
                    "Health",
                    format!("{:.0}", champion.health),
                ),
                ("stat_prayer", "Mana", format!("{:.0}", champion.mana)),
                ("stat_strength", "Damage", format!("{:.0}", champion.damage)),
                ("stat_defence", "Armour", format!("{:.0}", champion.armor)),
                ("stat_agility", "Speed", format!("{:.0}", champion.speed)),
                (attack, "Range", format!("{:.0}", champion.attack_range)),
            ];
            let (bx, by, _, bh) = bottom;
            let (cx, _, _, _) = hero_select::confirm_rect(width, height);
            let room = (cx - bx - 30.0).max(200.0);
            let cell = (room / stats.len() as f32).min(110.0);
            // Too narrow for the words (1024 wide): icon and number only,
            // and the cursor names the stat.
            let widest = stats
                .iter()
                .map(|(_, label, _)| self.text_width_named("p11", label, 1.0))
                .fold(0.0, f32::max);
            let labelled = cell >= 32.0 + widest + 6.0;
            let mut stat_tip: Option<Tooltip> = None;
            for (i, (icon, label, value)) in stats.iter().enumerate() {
                let x = bx + 20.0 + i as f32 * cell;
                let y = by + (bh - 40.0) * 0.5;
                if !labelled && point_in((x, y, cell, 40.0), mouse) {
                    stat_tip = Some(((x, y, cell, 40.0), vec![label.to_string()]));
                }
                if let (Some(uv), Some((iw, ih))) =
                    (self.atlas.ui_uv(icon), self.atlas.ui_size(icon))
                {
                    let k = (26.0 / iw.max(ih)).min(1.2);
                    self.sprite(
                        width,
                        height,
                        x,
                        y + 2.0,
                        iw * k,
                        ih * k,
                        uv,
                        [255, 255, 255],
                        1.0,
                    );
                }
                if labelled {
                    self.text_named("p11", width, height, x + 32.0, y, 1.0, label, STONE_QUIET);
                }
                self.text_named(
                    "p12",
                    width,
                    height,
                    x + 32.0,
                    if labelled { y + 16.0 } else { y + 9.0 },
                    1.0,
                    value,
                    STONE_TEXT,
                );
            }
            if let Some((anchor, text)) = stat_tip {
                self.tooltip(width, height, anchor, (0.0, 0.0, width, height), &text);
            }
        }
        let confirm = hero_select::confirm_rect(width, height);
        let locked = select.is_locked();
        let hot = !locked && hero_select::confirm_at(width, height, mouse);
        // Red to lock in; grey and inert once locked, as in the reference.
        self.osrs_button(width, height, confirm, !locked, hot, false);
        self.label_in(
            width,
            height,
            confirm,
            "b12",
            1.0,
            if locked { "Locked in" } else { "Lock in" },
            if locked {
                STONE_QUIET
            } else if hot {
                [255, 255, 255]
            } else {
                STONE_TEXT
            },
        );
        // The timer note, only where it clears the six stats (it ran over
        // them at 1280x720): the stats sit in cells from the bar's left, the
        // last one about 72px wide.
        let note = if locked {
            "Lane calls stay open until the match loads."
        } else {
            "At zero the highlighted hero is locked in."
        };
        let nw = self.text_width_named("p11", note, 1.0);
        let note_x = confirm.0 - nw - 18.0;
        let bar_x = hero_select::bottom_rect(width, height).0;
        let cell = ((confirm.0 - bar_x - 30.0).max(200.0) / 6.0).min(110.0);
        if note_x >= bar_x + 20.0 + cell * 5.0 + 72.0 {
            self.text_named(
                "p11",
                width,
                height,
                note_x,
                confirm.1 + confirm.3 * 0.5 - 6.0,
                1.0,
                note,
                STONE_QUIET,
            );
        }
        // The spellbook picker opens over everything else.
        self.spell_picker(width, height, select, mouse);
    }

    /// The pick timer under the title, pulsing red in the last five seconds.
    fn pick_clock(&mut self, width: f32, height: f32, select: &HeroSelect) {
        let left = select.seconds_left();
        let urgent = left <= 5;
        let clock = format!("0:{left:02}");
        let pulse = if urgent {
            0.5 + 0.5 * (select.countdown * std::f32::consts::TAU).cos()
        } else {
            1.0
        };
        let clock_colour = if urgent {
            [255, (60.0 + 120.0 * pulse) as u8, 40]
        } else {
            OSRS_YELLOW
        };
        self.label_in(
            width,
            height,
            hero_select::timer_rect(width, height),
            "b12",
            2.0,
            &clock,
            clock_colour,
        );
    }

    /// "Game starting in N" under the title once everyone has locked in,
    /// the number in yellow; "Loading the arena..." on the last frame.
    fn starting_line(&mut self, width: f32, height: f32, seconds: u32) {
        use super::kit::ORANGE;
        let rect = hero_select::timer_rect(width, height);
        if seconds == 0 {
            self.label_in(
                width,
                height,
                rect,
                "b12",
                2.0,
                "Loading the arena...",
                ORANGE,
            );
            return;
        }
        let lead = "Game starting in ";
        let number = seconds.to_string();
        let lw = self.text_width_named("b12", lead, 2.0);
        let nw = self.text_width_named("b12", &number, 2.0);
        let snapped = 2.0f32;
        let th = self.font_ascent_named("b12") * snapped;
        let (x, y, w, h) = rect;
        let left = (x + (w - lw - nw) * 0.5).round();
        let top = (y + (h - th) * 0.5).round();
        self.text_named("b12", width, height, left, top, 2.0, lead, ORANGE);
        self.text_named(
            "b12",
            width,
            height,
            left + lw,
            top,
            2.0,
            &number,
            OSRS_YELLOW,
        );
    }

    /// Whether every tab of `width` holds its icon and the longest of
    /// `labels`, so a row of tabs shows its labels all together or not at
    /// all.
    fn tabs_hold_labels(&self, width: f32, labels: &[&str]) -> bool {
        let widest = labels
            .iter()
            .map(|label| self.text_width_named("p12", label, 1.0))
            .fold(0.0, f32::max);
        26.0 + widest + 14.0 <= width
    }

    /// A notched tab with a cache icon and, when `labels` allows, its label
    /// (a tab without an icon always shows its label).
    #[allow(clippy::too_many_arguments)]
    fn icon_tab(
        &mut self,
        width: f32,
        height: f32,
        rect: (f32, f32, f32, f32),
        icon: Option<&str>,
        label: &str,
        labels: bool,
        active: bool,
        hot: bool,
    ) {
        use super::kit::{ORANGE, STONE_TEXT};
        self.osrs_button(width, height, rect, active, hot, false);
        let (x, y, w, h) = rect;
        let sprite = icon.and_then(|key| Some((self.atlas.ui_uv(key)?, self.atlas.ui_size(key)?)));
        let lw = self.text_width_named("p12", label, 1.0);
        let icon_w = if sprite.is_some() { 26.0 } else { 0.0 };
        let with_label = sprite.is_none() || labels;
        let start = if with_label {
            x + (w - lw - icon_w) * 0.5
        } else {
            x + (w - icon_w + 2.0) * 0.5
        };
        if let Some((uv, (iw, ih))) = sprite {
            let k = ((h - 12.0) / ih).min(1.0);
            self.sprite(
                width,
                height,
                start,
                y + (h - ih * k) * 0.5,
                iw * k,
                ih * k,
                uv,
                [255, 255, 255],
                1.0,
            );
        }
        if with_label {
            self.text_named(
                "p12",
                width,
                height,
                start + icon_w,
                y + h * 0.5 - 7.0,
                1.0,
                label,
                if active { STONE_TEXT } else { ORANGE },
            );
        }
    }

    /// Both teams' windows: the side's name, then five rows with a portrait
    /// slot, the player, the hero (or "Picking...") and the hero's role.
    fn team_columns(&mut self, width: f32, height: f32, select: &HeroSelect) {
        use super::kit::{ORANGE, STONE_QUIET, STONE_TEXT};
        use hero_select::Team;
        for (team, slots, tint, label) in [
            (Team::Blue, &select.blue, [96, 160, 255], "Blue team"),
            (Team::Red, &select.red, [255, 96, 80], "Red team"),
        ] {
            let column = hero_select::team_column(width, height, team);
            self.stone_frame(width, height, column);
            let (cx, cy, cw, _) = column;
            self.label_in(
                width,
                height,
                (cx, cy + 10.0, cw, 26.0),
                "b12",
                1.0,
                label,
                tint,
            );
            let rects = hero_select::team_slot_rects(width, height, team);
            for (row, (&(x, y, w, h), slot)) in rects.iter().zip(slots.iter()).enumerate() {
                let you = team == Team::Blue && row == 0;
                let shown = if you {
                    Some(select.selected)
                } else {
                    slot.champion
                };
                if you {
                    self.quad_alpha(width, height, x, y, w, h, [255, 214, 120], 0.10);
                    self.rect_outline(width, height, x, y, w, h, 1.0, [200, 168, 96], 0.9);
                }
                let side = (h - 14.0).clamp(24.0, 84.0);
                let px = x + 8.0;
                let py = y + (h - side) * 0.5;
                self.item_slot(width, height, (px, py, side, side), [255, 255, 255]);
                let champion = shown.and_then(|i| select.champions.get(i));
                match champion {
                    Some(champion) => {
                        if let Some(uv) = self.atlas.hero_uv(&champion.id) {
                            let alpha = if you && !select.is_locked() {
                                0.7 + 0.3 * (select.time * 3.0).sin().abs()
                            } else {
                                1.0
                            };
                            self.sprite(
                                width,
                                height,
                                px + 3.0,
                                py + 3.0,
                                side - 6.0,
                                side - 6.0,
                                uv,
                                [255, 255, 255],
                                alpha,
                            );
                        }
                    }
                    None => {
                        self.label_in(
                            width,
                            height,
                            (px, py, side, side),
                            "b12",
                            1.0,
                            "?",
                            [120, 112, 100],
                        );
                    }
                }
                let text_x = px + side + 12.0;
                // Lines centred on the portrait: player, hero, the hero's
                // role once there is a hero, and the lane they have called.
                let line = 18.0;
                let lines = if champion.is_some() { 4.0 } else { 3.0 };
                let top = py + side * 0.5 - line * lines * 0.5;
                let name = if you {
                    "You".to_string()
                } else {
                    slot.name.clone()
                };
                self.text_named(
                    "p12",
                    width,
                    height,
                    text_x,
                    top,
                    1.0,
                    &name,
                    if you { OSRS_YELLOW } else { STONE_TEXT },
                );
                let room = ((x + w - text_x) / 6.4) as usize;
                let locked_in = slot.champion.is_some() || (you && select.is_locked());
                let (status, colour) = match (champion, you || slot.champion.is_some()) {
                    (Some(champion), true) if locked_in => {
                        (clip_name(&champion.name, room), [120, 230, 120])
                    }
                    (Some(champion), _) => (
                        format!("{}...", clip_name(&champion.name, room.saturating_sub(3))),
                        STONE_QUIET,
                    ),
                    _ => ("Picking...".to_string(), STONE_QUIET),
                };
                self.text_named(
                    "p12",
                    width,
                    height,
                    text_x,
                    top + line,
                    1.0,
                    &status,
                    colour,
                );
                if let Some(champion) = champion {
                    let role = clip_name(&sentence_case(&champion.role), room);
                    self.text_named(
                        "p11",
                        width,
                        height,
                        text_x,
                        top + line * 2.0 + 2.0,
                        1.0,
                        &role,
                        Self::class_color(&champion.class),
                    );
                }
                let lane_y = top + line * (lines - 1.0) + 2.0;
                match slot.lane {
                    Some(lane) => {
                        let icon = lane.icon();
                        if let (Some(uv), Some((iw, ih))) =
                            (self.atlas.ui_uv(icon), self.atlas.ui_size(icon))
                        {
                            let k = 16.0 / iw.max(ih);
                            self.sprite(
                                width,
                                height,
                                text_x,
                                lane_y - 1.0,
                                iw * k,
                                ih * k,
                                uv,
                                [255, 255, 255],
                                1.0,
                            );
                        }
                        self.text_named(
                            "p12",
                            width,
                            height,
                            text_x + 20.0,
                            lane_y,
                            1.0,
                            lane.label(),
                            ORANGE,
                        );
                    }
                    None => {
                        self.text_named(
                            "p11",
                            width,
                            height,
                            text_x,
                            lane_y + 1.0,
                            1.0,
                            "No lane called",
                            STONE_QUIET,
                        );
                    }
                }
                // The row's battle spells, or the scroll for a hero
                // without them, in the top right corner.
                let icon = 20.0;
                let uvs: Vec<Option<[f32; 4]>> = match select.row_spells(team, row) {
                    hero_select::RowSpells::Spells(ids) => ids
                        .iter()
                        .map(|id| self.atlas.ui_uv(&crate::spells::icon_role(id)))
                        .collect(),
                    hero_select::RowSpells::Scroll => {
                        vec![self.atlas.item_uv(crate::scroll::ICON_KEY)]
                    }
                    hero_select::RowSpells::Nothing => Vec::new(),
                };
                for (k, uv) in uvs.iter().enumerate() {
                    let ix = x + w - 6.0 - (uvs.len() - k) as f32 * (icon + 3.0) + 3.0;
                    self.spell_icon(width, height, (ix, y + 5.0, icon, icon), *uv, false);
                }
            }
        }
    }

    /// The parchment: the animated hero in a recess with its name, class
    /// and role tags under it; then the tagline and the four abilities with
    /// their per-rank cooldowns.
    fn hero_detail(
        &mut self,
        width: f32,
        height: f32,
        select: &HeroSelect,
        shop: &ItemShop,
        mouse: (f64, f64),
    ) {
        use super::kit::{INK, INK_RED};
        let detail = hero_select::detail_rect(width, height);
        self.parchment(width, height, detail);
        let Some(champion) = select.selected_champion() else {
            return;
        };
        let (dx, dy, dw, dh) = detail;
        let (sx, sy, sw, sh) = hero_select::spotlight_rect(width, height);
        let tags = [
            sentence_case(&champion.class),
            sentence_case(&champion.role),
        ];
        let tag_ws: Vec<f32> = tags
            .iter()
            .map(|tag| self.text_width_named("p11", tag, 1.0) + 16.0)
            .collect();
        let tag_w = tag_ws.iter().sum::<f32>() + 6.0 * (tag_ws.len() as f32 - 1.0);
        // The left column fits both the recess and the tag row. On short
        // windows the recess is narrower than "Melee  Hellhound tank", so
        // the column widens and the art centres in it.
        let left = sx;
        let column = sw.max(tag_w);
        let sx = left + (column - sw) * 0.5;
        // The recess and the live spotlight render inside it.
        self.quad(
            width,
            height,
            sx - 3.0,
            sy - 3.0,
            sw + 6.0,
            sh + 6.0,
            [62, 50, 34],
        );
        self.quad(width, height, sx, sy, sw, sh, [26, 22, 16]);
        self.soft_ellipse(
            width,
            height,
            sx + sw * 0.5,
            sy + sh * 0.9,
            sw * 0.42,
            sh * 0.06,
            [255, 226, 160],
            0.25,
        );
        self.sprite(
            width,
            height,
            sx,
            sy,
            sw,
            sh,
            self.atlas.spotlight_uv(),
            [255, 255, 255],
            1.0,
        );
        let name_y = sy + sh + 10.0;
        let centre = (left - 10.0, name_y, column + 20.0, 24.0);
        let name_scale = if self.text_width_named("q8", &champion.name, 2.0) > column + 16.0 {
            1.0
        } else {
            2.0
        };
        self.text_named_centered_raw(
            width,
            height,
            centre,
            "q8",
            name_scale,
            &champion.name,
            INK_RED,
        );
        let mut tag_x = left + (column - tag_w) * 0.5;
        for (tag, &w) in tags.iter().zip(&tag_ws) {
            let rect = (tag_x, name_y + 34.0, w, 20.0);
            self.rect_outline(
                width,
                height,
                rect.0,
                rect.1,
                rect.2,
                rect.3,
                1.0,
                [120, 96, 60],
                0.9,
            );
            self.text_named_centered_raw(width, height, rect, "p11", 1.0, tag, [40, 60, 140]);
            tag_x += w + 6.0;
        }
        let spell_tip = self.spell_slots(width, height, select, mouse);

        // Right side, as in the reference: the popular build, the tagline,
        // then the passive and the four abilities.
        let x = left + column + 28.0;
        let right = dx + dw - 22.0;
        let bottom = dy + dh - 14.0;
        let mut y = dy + 16.0;
        let build = shop.popular_build(&champion.id, &champion.role, &champion.class, 5);
        let mut rows: Vec<DetailRow> = Vec::new();
        if let Some(kit) = champion.kit.as_ref()
            && let Some(passive) = kit.passive.as_ref()
        {
            rows.push(DetailRow {
                key: "Passive".to_string(),
                icon: None,
                name: kit
                    .passive_name
                    .clone()
                    .unwrap_or_else(|| "Passive".to_string()),
                cooldowns: None,
                description: passive.describe(),
            });
        }
        for (slot, ability) in champion.abilities.iter().enumerate() {
            let role = crate::ui_live::ability_icon_role(&champion.name, slot);
            let cooldowns: Vec<String> = ability
                .ranks
                .iter()
                .map(|rank| format_seconds(rank.cooldown))
                .collect();
            rows.push(DetailRow {
                key: ability.key.clone(),
                icon: self.atlas.ui_uv(&role),
                name: crate::ui_live::title_case(&ability.name),
                cooldowns: (!cooldowns.is_empty()).then(|| format!("{} s", cooldowns.join(" / "))),
                description: ability.description.clone(),
            });
        }
        let columns = ((right - x) / 6.2).max(20.0) as usize;
        let tagline = hero_select::wrap_text(&champion.tagline, columns);
        // Every row keeps its key chip and name. The flavour gives way
        // first (tagline, then the weapon), then the descriptions (rows
        // drop to one line and the text moves to a hover tooltip), and the
        // build last.
        // With no flavour left, the build row's own rule heads the rows.
        let flavour = |tag: usize, wields: bool, build: bool| tag > 0 || wields || !build;
        let header = |tag: usize, wields: bool, build: bool| {
            let top = if build { BUILD_H } else { 4.0 };
            if flavour(tag, wields, build) {
                top + tag as f32 * 17.0 + 8.0 + if wields { 18.0 } else { 0.0 } + 8.0
            } else {
                top
            }
        };
        let mut tag_lines = tagline.len().min(2);
        let mut wields = champion.weapon.is_some();
        let mut described = true;
        let mut show_build = build.is_some();
        loop {
            let row_h = if described { ROW_MIN } else { ROW_TIGHT };
            if header(tag_lines, wields, show_build) + rows.len() as f32 * row_h <= bottom - y {
                break;
            }
            if tag_lines > 1 {
                tag_lines -= 1;
            } else if wields {
                wields = false;
            } else if tag_lines > 0 {
                tag_lines -= 1;
            } else if described {
                described = false;
            } else if show_build {
                show_build = false;
            } else {
                break;
            }
        }
        let bounds = (dx, dy, dw, dh);
        let mut tooltip = None;
        if let Some((key, items)) = build.as_ref().filter(|_| show_build) {
            let (below, hovered) =
                self.popular_build_row(width, height, (x, y, right), key, items, mouse);
            y = below;
            tooltip = hovered;
        }
        if flavour(tag_lines, wields, show_build) {
            y = self.detail_flavour(
                width,
                height,
                (x, y, right),
                champion,
                &tagline[..tag_lines],
                wields,
            );
        }
        let row_min = if described { ROW_MIN } else { ROW_TIGHT };
        let room = ((bottom - y) / rows.len().max(1) as f32).max(row_min);
        let lines = if described {
            (((room - 20.0) / 15.0).floor() as usize).clamp(1, 3)
        } else {
            0
        };
        let key_w = (self.text_width_named("b12", "Passive", 1.0) + 14.0).max(54.0);
        // Tall rows stack the ability icon under its key chip; short ones
        // (720p with a build and a passive) set it beside the chip so it
        // never reaches the next row. Every row shares one text column.
        let stacked = room >= STACKED_ROW;
        let icon = if stacked {
            ICON
        } else {
            (room - 8.0).min(ICON)
        };
        let tx = if stacked {
            x + key_w + 12.0
        } else {
            x + key_w + 8.0 + icon + 10.0
        };
        let columns = ((right - tx) / 6.0).max(18.0) as usize;
        for row in &rows {
            self.key_chip(width, height, (x, y + 1.0, key_w, 20.0), &row.key);
            if let Some(uv) = row.icon {
                let (ix, iy) = if stacked {
                    (x + (key_w - icon) * 0.5, y + 25.0)
                } else {
                    (x + key_w + 8.0, y + 1.0)
                };
                self.sprite(width, height, ix, iy, icon, icon, uv, [255, 255, 255], 1.0);
            }
            self.text_named_raw(
                "b12",
                width,
                height,
                tx,
                y + 3.0,
                1.0,
                &row.name,
                INK_RED,
                1.0,
            );
            if let Some(cooldowns) = &row.cooldowns {
                let nx = tx + self.text_width_named("b12", &row.name, 1.0) + 10.0;
                if nx + self.text_width_named("p11", cooldowns, 1.0) <= right {
                    self.text_named_raw(
                        "p11",
                        width,
                        height,
                        nx,
                        y + 5.0,
                        1.0,
                        cooldowns,
                        [40, 60, 140],
                        1.0,
                    );
                }
            }
            let (shown, clipped) = hero_select::clip_lines(&row.description, columns, lines);
            for (line, text) in shown.iter().enumerate() {
                self.text_named_raw(
                    "p12",
                    width,
                    height,
                    tx,
                    y + 22.0 + line as f32 * 15.0,
                    1.0,
                    text,
                    INK,
                    1.0,
                );
            }
            // The full text on hover when the row cut it short.
            if clipped && point_in((x, y, right - x, room), mouse) {
                let wrap = ((right - tx) / 6.0).clamp(24.0, 60.0) as usize;
                let text = hero_select::wrap_text(&row.description, wrap);
                tooltip = Some(((tx, y, right - tx, room.min(22.0)), text));
            }
            y += room;
        }
        if tooltip.is_none() {
            tooltip = spell_tip;
        }
        if let Some((anchor, text)) = tooltip {
            self.tooltip(width, height, anchor, bounds, &text);
        }
    }

    /// One battle spell (or the scroll) in a slot frame: the cache icon
    /// inset, dimmed when `dim`.
    fn spell_icon(
        &mut self,
        width: f32,
        height: f32,
        rect: (f32, f32, f32, f32),
        uv: Option<[f32; 4]>,
        dim: bool,
    ) {
        let tint = if dim {
            [110, 110, 110]
        } else {
            [255, 255, 255]
        };
        self.item_slot(width, height, rect, tint);
        if let Some(uv) = uv {
            let (x, y, w, h) = rect;
            let inset = (w * 0.14).round();
            self.sprite(
                width,
                height,
                x + inset,
                y + inset,
                w - inset * 2.0,
                h - inset * 2.0,
                uv,
                tint,
                if dim { 0.5 } else { 1.0 },
            );
        }
    }

    /// The tooltip lines for a battle spell: name, book and cooldown, then
    /// what it does.
    fn spell_lines(select: &HeroSelect, index: usize, extra: Option<&str>) -> Vec<String> {
        let Some(spell) = select.spell_book.get(index) else {
            return Vec::new();
        };
        let mut lines = vec![format!(
            "{}  -  {} spellbook, {:.0} s",
            crate::ui_live::title_case(&spell.name),
            spell.book,
            spell.cooldown
        )];
        lines.extend(hero_select::wrap_text(&spell.describe(), 52));
        if let Some(extra) = extra {
            lines.push(extra.to_string());
        }
        lines
    }

    /// The D and F battle spell slots under the hero's tags. A non-magic
    /// hero's slots are greyed. Returns the tooltip for a hovered slot.
    fn spell_slots(
        &mut self,
        width: f32,
        height: f32,
        select: &HeroSelect,
        mouse: (f64, f64),
    ) -> Option<Tooltip> {
        let magic = select.selected_is_magic();
        let mut tip = None;
        for (slot, &rect) in hero_select::spell_slot_rects(width, height)
            .iter()
            .enumerate()
        {
            let key = ["D", "F"][slot];
            let index = select.spell_book.index_of(&select.spell_picks[slot]);
            let uv = index.filter(|_| magic).and_then(|i| {
                self.atlas
                    .ui_uv(&crate::spells::icon_role(&select.spell_book.spells[i].id))
            });
            self.spell_icon(width, height, rect, uv, !magic);
            let (x, y, w, h) = rect;
            if select.spell_picker == Some(slot) {
                self.rect_outline(width, height, x, y, w, h, 2.0, OSRS_YELLOW, 1.0);
            } else if magic && point_in(rect, mouse) && !select.is_locked() {
                self.rect_outline(width, height, x, y, w, h, 1.0, [236, 226, 200], 0.9);
            }
            let ink = if magic {
                [236, 226, 200]
            } else {
                [130, 122, 108]
            };
            self.text_named_raw("p11", width, height, x + 3.0, y + 1.0, 1.0, key, ink, 1.0);
            if point_in(rect, mouse) && select.spell_picker.is_none() {
                let lines = match index.filter(|_| magic) {
                    Some(i) => Self::spell_lines(
                        select,
                        i,
                        (!select.is_locked()).then_some("Click to choose another spell"),
                    ),
                    None => vec![
                        format!("{key}: battle spell"),
                        "Magic heroes only.".to_string(),
                        "Every hero carries a teleport scroll (T).".to_string(),
                    ],
                };
                tip = Some((rect, lines));
            }
        }
        tip
    }

    /// The open spellbook picker over the parchment: the spells book by
    /// book, the slot's current spell outlined, the other slot's marked.
    fn spell_picker(&mut self, width: f32, height: f32, select: &HeroSelect, mouse: (f64, f64)) {
        use super::kit::{ORANGE, STONE_QUIET};
        let Some(slot) = select.spell_picker else {
            return;
        };
        let order = select.picker_order();
        let books: Vec<&str> = order
            .iter()
            .map(|&i| select.spell_book.spells[i].book.as_str())
            .collect();
        let (panel, cells, labels) = hero_select::spell_picker_layout(width, height, &books);
        self.stone_frame(width, height, panel);
        let (px, py, pw, _) = panel;
        let title = format!("Choose a battle spell ({})", ["D", "F"][slot]);
        self.label_in(
            width,
            height,
            (px, py + 12.0, pw, 20.0),
            "b12",
            1.0,
            &title,
            ORANGE,
        );
        for (book, rect) in &labels {
            self.label_in(width, height, *rect, "p11", 1.0, book, STONE_QUIET);
        }
        let mut tip = None;
        for (&index, &rect) in order.iter().zip(&cells) {
            let spell = &select.spell_book.spells[index];
            let uv = self.atlas.ui_uv(&crate::spells::icon_role(&spell.id));
            self.spell_icon(width, height, rect, uv, false);
            let (x, y, w, h) = rect;
            let hot = point_in(rect, mouse);
            if select.spell_picks[slot] == spell.id {
                self.rect_outline(width, height, x, y, w, h, 2.0, OSRS_YELLOW, 1.0);
            } else if hot {
                self.rect_outline(width, height, x, y, w, h, 1.0, [236, 226, 200], 0.9);
            }
            let other = 1 - slot;
            if select.spell_picks[other] == spell.id {
                let key = ["D", "F"][other];
                self.text_named_raw(
                    "p11",
                    width,
                    height,
                    x + 3.0,
                    y + 1.0,
                    1.0,
                    key,
                    [236, 226, 200],
                    1.0,
                );
            }
            if hot {
                let extra = if select.spell_picks[other] == spell.id {
                    "In your other slot: picking it swaps them"
                } else {
                    "Click to pick"
                };
                tip = Some((rect, Self::spell_lines(select, index, Some(extra))));
            }
        }
        if let Some((anchor, lines)) = tip {
            self.tooltip(width, height, anchor, (0.0, 0.0, width, height), &lines);
        }
    }

    /// The dark key chip on the parchment ("Q", "Passive").
    fn key_chip(&mut self, width: f32, height: f32, rect: (f32, f32, f32, f32), label: &str) {
        let (x, y, w, h) = rect;
        self.quad(width, height, x, y, w, h, [26, 22, 18]);
        self.rect_outline(width, height, x, y, w, h, 1.0, [90, 76, 56], 1.0);
        self.label_in(width, height, rect, "b12", 1.0, label, [236, 226, 200]);
    }

    /// The tagline, the cosmetic weapon and the rule over the rows.
    /// Returns the y under the rule.
    fn detail_flavour(
        &mut self,
        width: f32,
        height: f32,
        (x, mut y, right): (f32, f32, f32),
        champion: &crate::champion::ChampionConfig,
        tagline: &[String],
        wields: bool,
    ) -> f32 {
        for line in tagline {
            self.text_named_raw("p12", width, height, x, y, 1.0, line, [70, 52, 30], 1.0);
            y += 17.0;
        }
        y += 8.0;
        if let Some(weapon) = champion.weapon.as_deref().filter(|_| wields) {
            let label = sentence_case(&weapon.replace('_', " "));
            self.text_named_raw(
                "p11",
                width,
                height,
                x,
                y,
                1.0,
                &format!("Wields: {label}"),
                [90, 66, 36],
                1.0,
            );
            y += 18.0;
        }
        self.quad_alpha(width, height, x, y, right - x, 1.0, [120, 96, 60], 0.7);
        y + 8.0
    }

    /// The OSRS hover box: pale yellow, black edge and text. It opens under
    /// `anchor`, or over it when there is no room below inside `bounds`.
    fn tooltip(
        &mut self,
        width: f32,
        height: f32,
        anchor: (f32, f32, f32, f32),
        bounds: (f32, f32, f32, f32),
        lines: &[String],
    ) {
        let (ax, ay, _, ah) = anchor;
        let (bx0, by0, bw, bh) = bounds;
        let tw = lines
            .iter()
            .map(|line| self.text_width_named("p11", line, 1.0))
            .fold(0.0, f32::max)
            + 10.0;
        let th = lines.len() as f32 * 14.0 + 6.0;
        let bx = ax.min(bx0 + bw - tw - 4.0).max(bx0 + 4.0);
        let by = if ay + ah + 2.0 + th <= by0 + bh - 4.0 {
            ay + ah + 2.0
        } else {
            (ay - 2.0 - th).max(by0 + 4.0)
        };
        self.quad(width, height, bx, by, tw, th, [255, 255, 160]);
        self.rect_outline(width, height, bx, by, tw, th, 1.0, [0, 0, 0], 1.0);
        for (k, line) in lines.iter().enumerate() {
            let ly = by + 3.0 + k as f32 * 14.0;
            self.text_named_raw(
                "p11",
                width,
                height,
                bx + 5.0,
                ly,
                1.0,
                line,
                [0, 0, 0],
                1.0,
            );
        }
    }

    /// "Popular build", the list it came from ("Melee", "Tank") and the
    /// finished items as cache icons. Returns the y under the row and, for
    /// the item under the cursor, its tooltip (name and price), which the
    /// caller draws last so later text cannot cover it.
    #[allow(clippy::too_many_arguments)]
    fn popular_build_row(
        &mut self,
        width: f32,
        height: f32,
        (x, y, right): (f32, f32, f32),
        key: &str,
        items: &[crate::items::ItemConfig],
        mouse: (f64, f64),
    ) -> (f32, Option<Tooltip>) {
        use super::kit::INK_RED;
        let label = "Popular build";
        self.text_named_raw("b12", width, height, x, y + 7.0, 1.0, label, INK_RED, 1.0);
        let mut ix = x + self.text_width_named("b12", label, 1.0) + 10.0;
        let list = sentence_case(&key.replace('_', " "));
        self.text_named_raw(
            "p11",
            width,
            height,
            ix,
            y + 9.0,
            1.0,
            &list,
            [40, 60, 140],
            1.0,
        );
        ix += self.text_width_named("p11", &list, 1.0) + 12.0;
        let mut hovered = None;
        for item in items {
            if ix + ICON > right {
                break;
            }
            self.draw_item_icon(width, height, ix, y, ICON, item, 1.0);
            let rect = (ix, y, ICON, ICON);
            if point_in(rect, mouse) {
                hovered = Some((rect, vec![format!("{}: {} coins", item.name, item.cost)]));
            }
            ix += ICON + 6.0;
        }
        self.quad_alpha(
            width,
            height,
            x,
            y + ICON + 3.0,
            right - x,
            1.0,
            [120, 96, 60],
            0.5,
        );
        (y + BUILD_H, hovered)
    }

    /// Centre unshadowed text (ink on parchment) in `rect`.
    #[allow(clippy::too_many_arguments)]
    fn text_named_centered_raw(
        &mut self,
        width: f32,
        height: f32,
        rect: (f32, f32, f32, f32),
        font: &str,
        scale: f32,
        text: &str,
        color: [u8; 3],
    ) {
        let (x, y, w, h) = rect;
        let tw = self.text_width_named(font, text, scale);
        let th = self.font_ascent_named(font) * scale.round().max(1.0);
        self.text_named_raw(
            font,
            width,
            height,
            (x + (w - tw) * 0.5).round(),
            (y + (h - th) * 0.5).round(),
            scale,
            text,
            color,
            1.0,
        );
    }
}

/// "BOT MATCH" -> "Bot match": OSRS interfaces use sentence case.
fn sentence_case(text: &str) -> String {
    let lower = text.to_lowercase();
    let mut chars = lower.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Clip a name to `chars` characters with "..." when it is longer.
fn clip_name(name: &str, chars: usize) -> String {
    if name.chars().count() <= chars {
        name.to_string()
    } else {
        let keep = chars.saturating_sub(2).max(1);
        format!("{}..", name.chars().take(keep).collect::<String>())
    }
}

/// 7.0 -> "7", 6.5 -> "6.5".
fn format_seconds(value: f32) -> String {
    if (value - value.round()).abs() < 0.05 {
        format!("{:.0}", value)
    } else {
        format!("{:.1}", value)
    }
}

fn point_in(rect: (f32, f32, f32, f32), cursor: (f64, f64)) -> bool {
    let (x, y, w, h) = rect;
    let (mx, my) = (cursor.0 as f32, cursor.1 as f32);
    mx >= x && mx <= x + w && my >= y && my <= y + h
}
