//! The step 1 stage 3 screen: the labelled rectangles from step B, with the
//! score strip drawn in its place. Each later component replaces one more
//! labelled rectangle here; until then a label is never mistaken for a
//! working component.

use crate::draw::DrawList;
use crate::hud::layout::{Layout, Piece};
use crate::hud::{ability_bar, minimap, score_strip, sketch, state::HudState};
use crate::theme::Theme;

pub fn build(theme: &Theme, layout: &Layout, state: &HudState) -> DrawList {
    let mut list = sketch::build(theme, layout);
    if let Some(area) = layout.get(Piece::Score) {
        score_strip::draw(theme, &mut list, area, &state.score);
    }
    if let Some(area) = layout.get(Piece::Ability) {
        ability_bar::draw(theme, &mut list, area, &state.ability);
    }
    if let Some(area) = layout.get(Piece::Minimap) {
        minimap::draw(theme, &mut list, area, &state.minimap);
    }
    list
}
