//! The Tab scoreboard checked as rectangles and vertices: centred at 540 by
//! 374 for five a side, the team totals come from the rows, the figures read
//! as drawn, everything stays in the window at scales 1 to 4, and a theme
//! with only a font still draws it.

use rift_ui::atlas::Image;
use rift_ui::draw::{DrawList, Rect};
use rift_ui::hud::state::ScoreboardState;
use rift_ui::hud::{sample, scoreboard, stage3};
use rift_ui::theme::{GlyphSource, Theme, ThemeBuilder};

fn boxes() -> Vec<GlyphSource> {
    (32u8..127)
        .map(|code| {
            let c = code as char;
            let w = (code % 3 + 2) as usize;
            GlyphSource {
                character: c,
                image: (c != ' ').then(|| Image::filled(w, 6, [0, 0, 0, 255])),
                left: 1,
                top: 2,
                advance: w as i32 + 2,
            }
        })
        .collect()
}

fn theme() -> Theme {
    let mut builder = ThemeBuilder::new();
    builder
        .panel(
            "panel.dark",
            Image::filled(20, 20, [9, 9, 9, 255]),
            [5, 5, 5, 5],
        )
        .unwrap();
    builder
        .panel(
            "panel.slot",
            Image::filled(12, 12, [30, 30, 30, 255]),
            [3, 3, 3, 3],
        )
        .unwrap();
    builder.font("font.small", boxes(), 8, 10);
    builder.font("font.bold", boxes(), 10, 12);
    builder.font("font.title", boxes(), 12, 14);
    for role in [
        "text",
        "text.quiet",
        "announcement",
        "text.team.blue",
        "text.team.red",
    ] {
        builder.colour(role, [1.0; 4]);
    }
    builder.build()
}

#[test]
fn the_panel_is_centred_and_grows_with_the_rows() {
    assert_eq!(scoreboard::panel_height(5, 5), 374);
    assert_eq!(
        scoreboard::panel_rect(Rect::new(0, 0, 1920, 1080), 1, 5, 5),
        Rect::new(690, 353, 540, 374)
    );
    // An empty team still takes one row.
    assert_eq!(scoreboard::panel_height(5, 0), 374 - 4 * 28);
    // Scale 2 doubles it; a short window cuts it to fit.
    assert_eq!(
        scoreboard::panel_rect(Rect::new(0, 0, 1920, 1080), 2, 5, 5),
        Rect::new(420, 166, 1080, 748)
    );
    assert_eq!(
        scoreboard::panel_rect(Rect::new(0, 0, 1024, 600), 2, 5, 5).h,
        600
    );
}

#[test]
fn team_totals_and_figures_come_from_the_rows() {
    let board = sample::scoreboard();
    assert_eq!(ScoreboardState::kills(&board.blue), 10);
    assert_eq!(ScoreboardState::kills(&board.red), 9);
    assert_eq!(ScoreboardState::gold(&board.blue), 32_500);
    assert_eq!(scoreboard::kda(&board.blue[1]), "3 / 1 / 4");
    assert_eq!(scoreboard::gold_text(9800), "9800");
    assert_eq!(scoreboard::gold_text(12_350), "12.4k");
    assert_eq!(scoreboard::gold_text(12_340), "12.3k");
    assert_eq!(scoreboard::gold_text(10_000), "10.0k");
}

#[test]
fn everything_stays_inside_the_window_at_every_scale() {
    let theme = theme();
    let board = sample::scoreboard();
    for (w, h) in [(1920u32, 1080u32), (1024, 600)] {
        for s in 1..=4u32 {
            let mut list = DrawList::new((w, h), s, &theme.atlas);
            scoreboard::draw(&theme, &mut list, &board);
            let (vertices, indices) = list.finish();
            assert!(!vertices.is_empty() && indices.len() % 6 == 0);
            for vertex in vertices {
                assert!(
                    vertex.pos[0] >= 0.0 && vertex.pos[0] <= w as f32,
                    "{w}x{h} s{s}"
                );
                assert!(
                    vertex.pos[1] >= 0.0 && vertex.pos[1] <= h as f32,
                    "{w}x{h} s{s}"
                );
            }
        }
    }
}

#[test]
fn a_theme_with_only_a_font_still_draws_the_board() {
    let mut builder = ThemeBuilder::new();
    builder.font("font.body", boxes(), 10, 12);
    builder.colour("text", [1.0; 4]);
    let theme = builder.build();
    let mut list = DrawList::new((1920, 1080), 1, &theme.atlas);
    scoreboard::draw(&theme, &mut list, &sample::scoreboard());
    assert!(!list.is_empty());
}

#[test]
fn the_stage3_screen_draws_the_board_only_when_asked() {
    let theme = theme();
    let layout = rift_ui::hud::layout::place(&sample::state(), (1920, 1080), Some(1));
    let mut state = sample::state();
    let without = stage3::build(&theme, &layout, &state).finish().0.len();
    state.scoreboard = Some(sample::scoreboard());
    let with = stage3::build(&theme, &layout, &state).finish().0.len();
    assert!(with > without);
}
