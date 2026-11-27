//! The scoreboard held open on Tab (docs/INTERFACE_STAGE3_PLAN.md, section
//! 5.12; roadmap 7.2).
//!
//! A dark panel centred on the window, 540 wide at scale 1:
//!
//! - A title band with the blue kill total at the left, the clock in the
//!   middle and the red total at the right.
//! - A quiet column header: Hero, Lvl, K / D / A, CS, Items, Gold.
//! - For each team, a header line in the team colour with its gold, then a
//!   28-pixel row per hero: portrait (the team-coloured initial when none is
//!   baked; grey with the respawn count while dead), name, level, K/D/A,
//!   creep score, six item slots and net worth. The player's own row has a
//!   faint gold backing.

use crate::Colour;
use crate::draw::{DrawList, Rect};
use crate::font::Font;
use crate::hud::state::{ScoreRow, ScoreboardState};
use crate::theme::Theme;

const LAYER: i32 = 48;
pub const PANEL_WIDTH: i32 = 540;
const TITLE: i32 = 28;
const COLUMNS: i32 = 16;
const TEAM_HEAD: i32 = 18;
const ROW: i32 = 28;
const GAP: i32 = 6;
const FOOT: i32 = 8;
const ITEM_SLOTS: usize = 6;

/// Column left edges (and widths where text is centred), scale 1.
const PORTRAIT_X: i32 = 8;
const NAME_X: i32 = 38;
const LEVEL_X: i32 = 168;
const LEVEL_W: i32 = 26;
const KDA_X: i32 = 198;
const KDA_W: i32 = 76;
const CS_X: i32 = 278;
const CS_W: i32 = 38;
const ITEMS_X: i32 = 322;
const ITEM: i32 = 20;
const ITEM_GAP: i32 = 2;
const GOLD_RIGHT: i32 = 532;

/// Rows a team block takes: one even for an empty team, so its header
/// never sits on the next one.
fn rows(team: &[ScoreRow]) -> i32 {
    team.len().max(1) as i32
}

/// The panel's height at scale 1.
pub fn panel_height(blue: usize, red: usize) -> i32 {
    TITLE + COLUMNS + TEAM_HEAD * 2 + (blue.max(1) as i32 + red.max(1) as i32) * ROW + GAP + FOOT
}

/// The panel: centred, and cut to the window when the window is smaller
/// (the clip then hides the rows that don't fit).
pub fn panel_rect(screen: Rect, scale: i32, blue: usize, red: usize) -> Rect {
    let s = scale.max(1);
    let w = (PANEL_WIDTH * s).min(screen.w);
    let h = (panel_height(blue, red) * s).min(screen.h);
    let x = screen.x + (screen.w - w) / 2;
    let y = (screen.y + (screen.h - h) / 2).max(screen.y);
    Rect::new(x, y, w, h)
}

/// "K / D / A" as drawn.
pub fn kda(row: &ScoreRow) -> String {
    format!("{} / {} / {}", row.kills, row.deaths, row.assists)
}

/// Gold as drawn: "12.4k" from ten thousand up, the plain figure below.
pub fn gold_text(gold: u32) -> String {
    if gold >= 10_000 {
        let tenths = (gold + 50) / 100;
        format!("{}.{}k", tenths / 10, tenths % 10)
    } else {
        gold.to_string()
    }
}

struct Ink {
    text: Colour,
    quiet: Colour,
    yellow: Colour,
    blue: Colour,
    red: Colour,
}

pub fn draw(theme: &Theme, list: &mut DrawList, board: &ScoreboardState) {
    let s = list.scale();
    let screen = list.screen();
    let area = panel_rect(screen, s, board.blue.len(), board.red.len());
    let previous = list.set_layer(LAYER);
    list.push_clip(area);
    // A solid backing first: the dark panel art is translucent, and the
    // match (unit plates, bright terrain) must not show through the rows.
    list.rect(area, [0.03, 0.03, 0.035, 0.94]);
    if let Some(style) = theme.panel("panel.dark") {
        list.panel(style, area);
    }

    let ink = Ink {
        text: theme.colour("text"),
        quiet: theme.colour("text.quiet"),
        yellow: theme.colour("announcement"),
        blue: theme.colour("text.team.blue"),
        red: theme.colour("text.team.red"),
    };
    let small = theme.font("font.small").or(theme.font("font.body"));
    let bold = theme.font("font.bold").or(small);
    let title = theme.font("font.title").or(bold);
    let mid = |font: &Font, top: i32, height: i32| top + (height * s + font.ascent * s) / 2;

    // Title band: blue total, clock, red total.
    if let Some(font) = title {
        let baseline = mid(font, area.y, TITLE);
        let blue = format!("Blue  {}", ScoreboardState::kills(&board.blue));
        list.text(font, &blue, area.x + 12 * s, baseline, ink.blue);
        let red = format!("{}  Red", ScoreboardState::kills(&board.red));
        let width = font.width(&red) * s;
        list.text(font, &red, area.right() - 12 * s - width, baseline, ink.red);
        let width = font.width(&board.clock) * s;
        list.text(
            font,
            &board.clock,
            area.x + (area.w - width) / 2,
            baseline,
            ink.yellow,
        );
    }
    let mut y = area.y + TITLE * s;
    list.rect(
        Rect::new(area.x + 6 * s, y - s, area.w - 12 * s, s),
        [0.45, 0.38, 0.24, 0.6],
    );

    // Column header.
    if let Some(font) = small {
        let baseline = mid(font, y, COLUMNS);
        list.text(font, "Hero", area.x + NAME_X * s, baseline, ink.quiet);
        centred(
            list,
            font,
            "Lvl",
            area.x + LEVEL_X * s,
            LEVEL_W * s,
            baseline,
            ink.quiet,
        );
        centred(
            list,
            font,
            "K / D / A",
            area.x + KDA_X * s,
            KDA_W * s,
            baseline,
            ink.quiet,
        );
        centred(
            list,
            font,
            "CS",
            area.x + CS_X * s,
            CS_W * s,
            baseline,
            ink.quiet,
        );
        list.text(font, "Items", area.x + ITEMS_X * s, baseline, ink.quiet);
        right(
            list,
            font,
            "Gold",
            area.x + GOLD_RIGHT * s,
            baseline,
            ink.quiet,
        );
    }
    y += COLUMNS * s;

    for (n, (team, name, colour, wash)) in [
        (&board.blue, "Blue team", ink.blue, [0.20, 0.32, 0.62, 0.95]),
        (&board.red, "Red team", ink.red, [0.62, 0.20, 0.18, 0.95]),
    ]
    .into_iter()
    .enumerate()
    {
        if n == 1 {
            y += GAP * s;
        }
        if let Some(font) = bold {
            let baseline = mid(font, y, TEAM_HEAD);
            list.text(font, name, area.x + 10 * s, baseline, colour);
            let gold = gold_text(ScoreboardState::gold(team));
            right(
                list,
                font,
                &gold,
                area.x + GOLD_RIGHT * s,
                baseline,
                ink.yellow,
            );
        }
        list.rect(
            Rect::new(area.x + 6 * s, y + TEAM_HEAD * s - s, area.w - 12 * s, s),
            [colour[0], colour[1], colour[2], 0.45],
        );
        y += TEAM_HEAD * s;
        for row in team.iter() {
            draw_row(theme, list, area, y, row, wash, &ink, small);
            y += ROW * s;
        }
        y += (rows(team) - team.len() as i32) * ROW * s;
    }

    list.pop_clip();
    list.set_layer(previous);
}

#[allow(clippy::too_many_arguments)]
fn draw_row(
    theme: &Theme,
    list: &mut DrawList,
    area: Rect,
    y: i32,
    row: &ScoreRow,
    wash: Colour,
    ink: &Ink,
    font: Option<&Font>,
) {
    let s = list.scale();
    if row.me {
        list.rect(
            Rect::new(area.x + 4 * s, y + s, area.w - 8 * s, (ROW - 2) * s),
            [0.55, 0.45, 0.18, 0.22],
        );
    }
    let slot = |list: &mut DrawList, rect: Rect| match theme.panel("panel.slot") {
        Some(panel) => list.panel(panel, rect),
        None => list.rect(rect, [0.08, 0.07, 0.06, 0.95]),
    };

    // Portrait.
    let portrait = Rect::new(area.x + PORTRAIT_X * s, y + 2 * s, 24 * s, 24 * s);
    slot(list, portrait);
    let inner = Rect::new(
        portrait.x + 2 * s,
        portrait.y + 2 * s,
        portrait.w - 4 * s,
        portrait.h - 4 * s,
    );
    let dead = row.respawn_seconds.is_some();
    list.rect(inner, if dead { [0.18, 0.18, 0.18, 0.95] } else { wash });
    let mut drew = false;
    if !dead
        && let Some(role) = row.portrait.as_deref()
        && theme.has_sprite(role)
    {
        list.sprite_fit(theme.sprite(role), inner);
        drew = true;
    }
    let Some(font) = font else {
        return;
    };
    let baseline = y + (ROW * s + font.ascent * s) / 2;
    if !drew {
        let mark = match row.respawn_seconds {
            Some(seconds) => seconds.to_string(),
            None => row.name.chars().next().unwrap_or('?').to_string(),
        };
        centred(
            list,
            font,
            &mark,
            portrait.x,
            portrait.w,
            baseline,
            if dead { ink.quiet } else { [1.0; 4] },
        );
    }

    let name_colour = if dead { ink.quiet } else { ink.text };
    list.push_clip(Rect::new(
        area.x + NAME_X * s,
        y,
        (LEVEL_X - NAME_X - 4) * s,
        ROW * s,
    ));
    list.text(font, &row.name, area.x + NAME_X * s, baseline, name_colour);
    list.pop_clip();
    centred(
        list,
        font,
        &row.level.to_string(),
        area.x + LEVEL_X * s,
        LEVEL_W * s,
        baseline,
        ink.text,
    );
    centred(
        list,
        font,
        &kda(row),
        area.x + KDA_X * s,
        KDA_W * s,
        baseline,
        ink.text,
    );
    centred(
        list,
        font,
        &row.creep_score.to_string(),
        area.x + CS_X * s,
        CS_W * s,
        baseline,
        ink.text,
    );

    for n in 0..ITEM_SLOTS {
        let rect = Rect::new(
            area.x + (ITEMS_X + n as i32 * (ITEM + ITEM_GAP)) * s,
            y + 4 * s,
            ITEM * s,
            ITEM * s,
        );
        slot(list, rect);
        if let Some(role) = row.items.get(n)
            && theme.has_sprite(role)
        {
            list.sprite_fit(
                theme.sprite(role),
                Rect::new(rect.x + s, rect.y + s, rect.w - 2 * s, rect.h - 2 * s),
            );
        }
    }

    right(
        list,
        font,
        &gold_text(row.gold),
        area.x + GOLD_RIGHT * s,
        baseline,
        ink.yellow,
    );
}

fn centred(
    list: &mut DrawList,
    font: &Font,
    words: &str,
    x: i32,
    width: i32,
    baseline: i32,
    colour: Colour,
) {
    let w = font.width(words) * list.scale();
    list.text(font, words, x + (width - w) / 2, baseline, colour);
}

fn right(list: &mut DrawList, font: &Font, words: &str, right: i32, baseline: i32, colour: Colour) {
    let w = font.width(words) * list.scale();
    list.text(font, words, right - w, baseline, colour);
}
