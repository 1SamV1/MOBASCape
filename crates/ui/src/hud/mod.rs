//! The stage 3 interface's plain data, placing pass and labelled step B
//! screen. It is all pure: no cache, no graphics card, no match.

pub mod ability_bar;
pub mod layout;
pub mod minimap;
pub mod sample;
pub mod score_strip;
pub mod sketch;
pub mod stage3;
pub mod state;

pub use layout::{
    Conflict, Layout, Piece, choose_scale, closer_than_gap, effective_scale, fits_full, holds,
    place, place_at,
};
pub use state::HudState;
