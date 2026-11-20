//! The stage 3 screen: the labelled rectangles from step B, with each
//! component drawn in its place as it is built. Until a component is built
//! its label is never mistaken for a working piece.

use crate::draw::DrawList;
use crate::hud::layout::{Layout, Piece};
use crate::hud::{
    ability_bar, minimap, objectives, score_strip, sketch, state::HudState, target_panel,
    team_frames,
};
use crate::theme::Theme;

pub fn build(theme: &Theme, layout: &Layout, state: &HudState) -> DrawList {
    let mut list = sketch::build_without(
        theme,
        layout,
        &[
            Piece::Score,
            Piece::Ability,
            Piece::Minimap,
            Piece::Objectives,
            Piece::Team,
            Piece::Target,
        ],
    );
    if let Some(area) = layout.get(Piece::Score) {
        score_strip::draw(theme, &mut list, area, &state.score);
    }
    if let Some(area) = layout.get(Piece::Ability) {
        ability_bar::draw(theme, &mut list, area, &state.ability);
    }
    if let Some(area) = layout.get(Piece::Minimap) {
        minimap::draw(theme, &mut list, area, &state.minimap);
    }
    if let Some(area) = layout.get(Piece::Objectives) {
        objectives::draw(theme, &mut list, area, &state.objectives);
    }
    if let Some(area) = layout.get(Piece::Team) {
        team_frames::draw(theme, &mut list, area, &state.allies);
    }
    if let (Some(area), Some(target)) = (layout.get(Piece::Target), state.target.as_ref()) {
        target_panel::draw(theme, &mut list, area, target);
    }
    list
}
