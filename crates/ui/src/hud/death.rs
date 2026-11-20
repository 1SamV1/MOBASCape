//! The death overlay (docs/INTERFACE_STAGE3_PLAN.md, section 5.11): the
//! match dimmed, "Oh dear, you are dead!" and the respawn countdown, then a
//! dark panel with who dealt the killing blow and a bar for each source of
//! damage with its total.
//!
//! Stage 3 draws a sample death. The scene is only dimmed; drawing the world
//! in grey is a later rendering task.

use crate::draw::{DrawList, Rect};
use crate::hud::state::DeathState;
use crate::outline;
use crate::theme::Theme;

const DIM_LAYER: i32 = 5;
const LAYER: i32 = 46;
const PANEL_WIDTH: i32 = 330;

/// The damage panel's rectangle: 330 wide, one line for the killer and 16
/// pixels for each source.
pub fn panel_rect(screen: Rect, scale: i32, sources: usize) -> Rect {
    let s = scale.max(1);
    let w = PANEL_WIDTH * s;
    let h = (24 + sources as i32 * 16 + 8) * s;
    let x = screen.x + (screen.w - w) / 2;
    let y = (screen.h * 41 / 100)
        .min(screen.bottom() - h - 8 * s)
        .max(screen.y + 8 * s);
    Rect::new(x, y, w, h)
}

pub fn draw(theme: &Theme, list: &mut DrawList, death: &DeathState) {
    let s = list.scale();
    let screen = list.screen();
    let previous = list.set_layer(DIM_LAYER);
    list.rect(screen, theme.colour("dim"));
    list.set_layer(LAYER);

    let text = theme.colour("text");
    let quiet = theme.colour("text.quiet");
    let red = theme.colour("death");
    let yellow = theme.colour("announcement");
    let outline_colour = theme.colour("text.outline");
    let enemy = theme.colour("bar.enemy");
    let back = theme.colour("bar.back");
    let blue_team = theme.colour("text.team.blue");
    let red_team = theme.colour("text.team.red");
    let small = theme.font("font.small");
    let body = theme.font("font.body").or(small);
    let bold = theme.font("font.bold").or(body);
    let title = theme.font("font.title").or(bold);

    let mut below = screen.h * 37 / 100;
    if let Some(font) = title {
        let words = "Oh dear, you are dead!";
        let width = font.width(words) * s;
        let x = screen.x + (screen.w - width - s) / 2;
        outline::draw(list, font, words, x, below, red, outline_colour);
        below += 32 * s;
    }

    if let Some(font) = title.or(bold) {
        let words = format!("Respawning in {}s", death.respawn_seconds);
        let width = font.width(&words) * s;
        let x = screen.x + (screen.w - width - s) / 2;
        outline::draw(list, font, &words, x, below, yellow, outline_colour);
    }

    let area = panel_rect(screen, s, death.damage.len());
    match theme.panel("panel.dark") {
        Some(style) => list.panel(style, area),
        None => list.rect(area, [0.02, 0.02, 0.03, 0.92]),
    }

    let killer_team = if death.killer_blue {
        blue_team
    } else {
        red_team
    };
    let heading = bold.or(body);
    let baseline = area.y + 17 * s;
    if let Some(font) = heading {
        let prefix = "Killed by ";
        let width = list.text(font, prefix, area.x + 10 * s, baseline, quiet);
        list.text(
            font,
            &death.killer,
            area.x + 10 * s + width,
            baseline,
            killer_team,
        );
        let total = format!("{}", death.total());
        let width = font.width(&total) * s;
        list.text(
            font,
            &total,
            area.right() - 10 * s - width,
            baseline,
            yellow,
        );
    }

    let total = death.total().max(1);
    let mut y = area.y + 28 * s;
    for source in &death.damage {
        if let Some(font) = small {
            list.text(font, &source.name, area.x + 10 * s, y + 12 * s, quiet);
            let amount = source.amount.to_string();
            let width = font.width(&amount) * s;
            list.text(
                font,
                &amount,
                area.right() - 10 * s - width,
                y + 12 * s,
                text,
            );
        }
        let bar = Rect::new(area.x + 90 * s, y + 4 * s, 160 * s, 8 * s);
        list.rect(bar, back);
        let fraction = (source.amount as f32 / total as f32).clamp(0.0, 1.0);
        list.rect(
            Rect::new(bar.x, bar.y, (bar.w as f32 * fraction) as i32, bar.h),
            enemy,
        );
        y += 16 * s;
    }

    list.set_layer(previous);
}
