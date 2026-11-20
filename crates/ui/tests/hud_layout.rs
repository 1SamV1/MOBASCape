//! The placing pass checked against the figures in
//! docs/INTERFACE_STAGE3_PLAN.md, section 8. These are tests of rectangles,
//! not pictures: the labelled screen is what the eye checks.

use rift_ui::draw::Rect;
use rift_ui::hud::layout::{
    BUDGET, CHAT, Piece, choose_scale, closer_than_gap, effective_scale, fits_full, holds, place,
    place_at,
};
use rift_ui::hud::{HudState, sample};

fn busy() -> HudState {
    let mut state = sample::state();
    state.feed = vec![
        sample::state().feed[0].clone(),
        sample::state().feed[1].clone(),
        sample::state().feed[0].clone(),
        sample::state().feed[1].clone(),
    ];
    state
}

fn usual() -> HudState {
    let mut state = sample::state();
    state.target = None;
    state.feed.truncate(1);
    state.objectives.second_lines = 0;
    state
}

fn rect(layout: &rift_ui::hud::Layout, piece: Piece) -> Rect {
    layout
        .get(piece)
        .unwrap_or_else(|| panic!("{} missing", piece.name()))
}

#[test]
fn the_automatic_scale_reaches_two_at_the_sizes_that_can_hold_it() {
    let state = sample::state();
    for (window, scale) in [
        ((3200, 1800), 2),
        ((3840, 2160), 2),
        ((2560, 1440), 1),
        ((1920, 1080), 1),
    ] {
        let layout = place(&state, window, None);
        assert_eq!(layout.scale, scale, "{window:?}");
        assert!(
            layout.conflicts.is_empty(),
            "{window:?}: {:?}",
            layout.conflicts
        );
    }
}

#[test]
fn the_1920_by_1080_rectangles_match_the_plan() {
    let state = busy();
    let layout = place_at(&state, (1920, 1080), 1, None);
    assert_eq!(layout.scale, 1);
    assert_eq!(
        rect(&layout, Piece::Minimap),
        Rect::new(1646, 800, 266, 264)
    );
    assert_eq!(
        rect(&layout, Piece::Inventory),
        Rect::new(1494, 902, 144, 160)
    );
    assert_eq!(rect(&layout, Piece::Chat), Rect::new(5, 930, 440, 140));
    assert_eq!(rect(&layout, Piece::Ability), Rect::new(731, 968, 458, 94));
    assert_eq!(rect(&layout, Piece::Score), Rect::new(693, 10, 534, 38));
    assert_eq!(rect(&layout, Piece::Feed), Rect::new(1622, 18, 286, 118));
    assert_eq!(rect(&layout, Piece::Team), Rect::new(1728, 168, 180, 188));
    assert_eq!(rect(&layout, Piece::Target), Rect::new(8, 82, 332, 158));
    assert_eq!(
        rect(&layout, Piece::Objectives),
        Rect::new(8, 410, 272, 398)
    );
    assert!(layout.conflicts.is_empty(), "{:?}", layout.conflicts);
    assert!((layout.usual_coverage - 0.165_027_006_172_839_5).abs() < 1e-12);
}

#[test]
fn the_game_sample_gives_way_at_the_sizes_the_plan_lists() {
    let state = sample::state();
    let wide = place(&state, (1920, 1080), None);
    assert_eq!(wide.scale, 1);
    assert!(wide.steps.is_empty(), "{:?}", wide.steps);
    assert_eq!(rect(&wide, Piece::Objectives), Rect::new(8, 410, 272, 398));
    assert_eq!(rect(&wide, Piece::Feed), Rect::new(1622, 18, 286, 58));
    assert_eq!(rect(&wide, Piece::Target), Rect::new(8, 82, 332, 158));

    let small = place(&state, (1280, 720), None);
    assert_eq!(small.scale, 1);
    assert_eq!(
        small.steps,
        [
            "chat narrows to 375",
            "objectives to its title (budget)",
            "team frames to portraits (budget)",
            "chat to its last line (budget)",
        ]
    );
    assert_eq!(rect(&small, Piece::Chat), Rect::new(5, 686, 375, 24));
    assert_eq!(rect(&small, Piece::Ability), Rect::new(388, 608, 458, 94));
    assert_eq!(rect(&small, Piece::Objectives), Rect::new(8, 274, 272, 24));
    assert_eq!(rect(&small, Piece::Target), Rect::new(8, 82, 332, 158));
    assert!(small.get(Piece::Inventory).is_some());
    assert!(small.conflicts.is_empty(), "{:?}", small.conflicts);

    let middle = place(&state, (1366, 768), None);
    assert_eq!(
        middle.steps,
        [
            "objectives to its title (budget)",
            "team frames to portraits (budget)",
        ]
    );
    assert_eq!(rect(&middle, Piece::Chat), Rect::new(5, 618, 440, 140));
    assert_eq!(rect(&middle, Piece::Ability), Rect::new(454, 656, 458, 94));
}

#[test]
fn the_scale_is_the_largest_that_fits_with_nothing_given_up() {
    let state = busy();
    assert!(fits_full(&state, (1562, 878), 1, BUDGET));
    assert!(fits_full(&state, (3120, 1755), 2, BUDGET));
    assert!(!fits_full(&state, (3120, 1754), 2, BUDGET));
    assert_eq!(choose_scale(&state, (1280, 720), BUDGET), 1);
    assert_eq!(choose_scale(&state, (1366, 768), BUDGET), 1);
    assert_eq!(choose_scale(&state, (1600, 900), BUDGET), 1);
    assert_eq!(choose_scale(&state, (1920, 1080), BUDGET), 1);
    assert_eq!(choose_scale(&state, (2560, 1440), BUDGET), 1);
    assert_eq!(choose_scale(&state, (3200, 1800), BUDGET), 2);
    assert_eq!(choose_scale(&state, (3840, 2160), BUDGET), 2);
    assert_eq!(choose_scale(&state, (5120, 1440), BUDGET), 1);
}

#[test]
fn can_hold_is_exact_at_953_by_280_times_the_scale() {
    let state = busy();
    assert!(holds(&state, (953, 280), 1));
    assert!(!holds(&state, (952, 280), 1));
    assert!(!holds(&state, (953, 279), 1));
    assert!(holds(&state, (1906, 560), 2));
    assert!(!holds(&state, (1905, 560), 2));
    assert!(!holds(&state, (1906, 559), 2));
}

#[test]
fn a_requested_scale_is_a_request_and_never_an_overlap() {
    let state = busy();
    assert_eq!(effective_scale(&state, (1811, 1040), Some(2)), 1);
    assert_eq!(effective_scale(&state, (1905, 1040), Some(2)), 1);
    assert_eq!(effective_scale(&state, (1906, 1040), Some(2)), 2);
    assert_eq!(effective_scale(&state, (1920, 559), Some(2)), 1);
    assert_eq!(effective_scale(&state, (1920, 560), Some(2)), 2);
    assert_eq!(effective_scale(&state, (2240, 1260), Some(3)), 2);
    assert_eq!(effective_scale(&state, (3840, 2160), Some(4)), 4);
}

#[test]
fn the_clearance_is_seven_clash_eight_clear_at_every_scale() {
    for scale in [1, 2] {
        let gap = 8 * scale;
        let a = Rect::new(10, 10, 100, 50);
        let at = |distance: i32| Rect::new(a.right() + distance, 10, 100, 50);
        assert!(closer_than_gap(a, at(gap - 1), gap));
        assert!(!closer_than_gap(a, at(gap), gap));
        assert!(!closer_than_gap(a, at(gap + 1), gap));
        let down = |distance: i32| Rect::new(10, a.bottom() + distance, 100, 50);
        assert!(closer_than_gap(a, down(gap - 1), gap));
        assert!(!closer_than_gap(a, down(gap), gap));
        assert!(!closer_than_gap(a, down(gap + 1), gap));
    }
}

#[test]
fn the_objectives_keep_the_target_panels_place_at_a_forced_scale() {
    let state = busy();
    let layout = place_at(&state, (1920, 1080), 2, Some(2));
    assert_eq!(rect(&layout, Piece::Target), Rect::new(16, 164, 664, 316));
    assert_eq!(
        rect(&layout, Piece::Objectives),
        Rect::new(16, 496, 544, 48)
    );
    assert!(layout.get(Piece::Objectives).unwrap().y >= 480 + 8 * 2);
}

#[test]
fn the_bar_slides_and_the_chat_takes_what_is_left() {
    let state = usual();
    let at = |width: u32| place(&state, (width, 720), None);
    let wide = at(1364);
    assert_eq!(rect(&wide, Piece::Ability).x, 453);
    assert_eq!(rect(&wide, Piece::Chat).w, CHAT.0);
    let tight = at(1280);
    assert_eq!(rect(&tight, Piece::Ability).x, 388);
    assert_eq!(rect(&tight, Piece::Chat).w, 375);
    assert!(tight.get(Piece::Inventory).is_some());
    let sliding = at(1325);
    assert_eq!(rect(&sliding, Piece::Ability).x, 433);
    assert_eq!(rect(&sliding, Piece::Chat).w, 420);
    assert_eq!(
        rect(&sliding, Piece::Inventory).x - rect(&sliding, Piece::Ability).right(),
        8
    );
    let hidden = at(1100);
    assert_eq!(rect(&hidden, Piece::Ability).x, 321);
    assert_eq!(rect(&hidden, Piece::Chat).w, 308);
    assert!(hidden.get(Piece::Inventory).is_none());
}

#[test]
fn the_minimap_takes_the_top_row_only_when_it_reaches_it() {
    let state = usual();
    let clear = place(&state, (2400, 334), None);
    let feed = rect(&clear, Piece::Feed);
    assert_eq!(rect(&clear, Piece::Minimap).y - feed.bottom(), 8);
    let reached = place(&state, (2400, 333), None);
    assert!(reached.get(Piece::Feed).is_none());
    // The score strip is not beside the minimap at this width, so height
    // alone does not take it away.
    assert!(reached.get(Piece::Score).is_some());
}

#[test]
fn the_objectives_top_is_38_percent_with_a_half_going_up() {
    let state = busy();
    let layout = place_at(&state, (1440, 2560), 1, None);
    assert_eq!(rect(&layout, Piece::Objectives).y, 973);
    // 38% of 2559 is 972.42, a whole number rule gives 972.
    let lower = place_at(&busy(), (1440, 2559), 1, None);
    assert_eq!(rect(&lower, Piece::Objectives).y, 972);
}

#[test]
fn every_swept_window_after_collapse_has_no_clash_and_is_within_the_quarter() {
    let state = sample::state();
    for width in (960..=3840).step_by(128) {
        for height in (384..=2400).step_by(128) {
            let layout = place(&state, (width, height), None);
            assert!(
                layout.conflicts.is_empty(),
                "{width} x {height} at scale {}: {:?}",
                layout.scale,
                layout.conflicts
            );
            // A window smaller than the ability bar and minimap together
            // cannot reach a quarter whatever gives way; the plan's
            // supported sizes are checked separately below.
            if width >= 1280 && height >= 720 {
                assert!(
                    layout.usual_coverage <= BUDGET + 1e-12,
                    "{width} x {height}: usual coverage {}",
                    layout.usual_coverage
                );
            }
        }
    }
}

#[test]
fn the_same_window_gives_the_same_rectangles_whatever_came_before() {
    let state = sample::state();
    // The function takes only the window and the state, so this is a guard
    // against a later implementation keeping a previous frame's layout.
    let mut seen = None;
    for (width, height) in [(1920, 1080), (960, 384), (3840, 2400), (1280, 720)] {
        let _ = place(&state, (width, height), None);
        let again = place(&state, (1920, 1080), None);
        let rectangles: Vec<(Piece, Rect)> = again.pieces.iter().map(|(p, r)| (*p, *r)).collect();
        if let Some(first) = &seen {
            assert_eq!(first, &rectangles);
        } else {
            seen = Some(rectangles);
        }
    }
}

#[test]
fn every_pair_is_at_least_eight_apart_at_the_named_sizes() {
    let state = sample::state();
    for window in [
        (956, 280),
        (1280, 720),
        (1366, 768),
        (1600, 900),
        (1920, 1080),
        (2560, 1080),
        (3440, 1440),
        (5120, 1440),
        (1920, 1200),
        (1600, 1200),
        (1280, 1024),
        (1024, 768),
        (1080, 1920),
        (1440, 2560),
    ] {
        let layout = place(&state, window, None);
        assert!(
            layout.conflicts.is_empty(),
            "{window:?} at scale {}: {:?}",
            layout.scale,
            layout.conflicts
        );
    }
}

#[test]
fn the_usual_state_is_within_the_quarter_at_the_supported_sizes() {
    let state = sample::state();
    for (width, height) in [
        (1280, 720),
        (1366, 768),
        (1600, 900),
        (1920, 1080),
        (2560, 1440),
        (3200, 1800),
        (3840, 2160),
        (1024, 768),
        (1080, 1920),
        (1280, 1024),
    ] {
        let layout = place(&state, (width, height), None);
        assert!(
            layout.usual_coverage <= BUDGET + 1e-12,
            "{width} x {height}: usual coverage {}",
            layout.usual_coverage
        );
    }
}
