//! The OSRS interface kit for the front-end screens: stone windows, darker
//! inset panels, parchment, the notched grey and red buttons and item
//! slots. Every piece is a cache sprite in the UI atlas (see
//! `ui_assets::UI_SPRITES` and `stone.rs`); each draw falls back to flat
//! fills when the cache did not provide the piece.

use super::Hud;

/// A screen rectangle: x, y, width, height in window pixels.
pub type Rect = (f32, f32, f32, f32);

/// Corner size of the joined stone frames (`stone::CORNER`).
const FRAME_INSET: f32 = crate::stone::CORNER as f32;
/// The notched buttons keep 6px of each side (the cut corners).
const BUTTON_INSET: f32 = 6.0;
/// The parchment keeps its 8px stone rim.
const PARCHMENT_INSET: f32 = 8.0;

/// Text colours the OSRS interfaces use on stone and on parchment.
pub const ORANGE: [u8; 3] = [255, 152, 31];
pub const STONE_TEXT: [u8; 3] = [255, 236, 200];
pub const STONE_QUIET: [u8; 3] = [190, 176, 150];
pub const INK: [u8; 3] = [48, 32, 16];
pub const INK_RED: [u8; 3] = [128, 0, 0];

impl Hud {
    /// Draw atlas sprite `key` as a nine-slice whose edges and middle
    /// repeat (they are textures, not gradients), keeping `inset` pixels of
    /// each side. Returns false when the atlas has no such sprite.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn tiled_nine(
        &mut self,
        width: f32,
        height: f32,
        rect: Rect,
        key: &str,
        inset: f32,
        tint: [u8; 3],
        alpha: f32,
    ) -> bool {
        self.nine(width, height, rect, key, inset, tint, alpha, false)
    }

    /// The nine-slice behind [`Hud::tiled_nine`]. With `stretch`, the edges
    /// and middle are scaled instead of repeated (for pictures with a
    /// gradient, such as the parchment, whose repeats would show seams).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn nine(
        &mut self,
        width: f32,
        height: f32,
        rect: Rect,
        key: &str,
        inset: f32,
        tint: [u8; 3],
        alpha: f32,
        stretch: bool,
    ) -> bool {
        let (Some(uv), Some((sw, sh))) = (self.atlas.ui_uv(key), self.atlas.ui_size(key)) else {
            return false;
        };
        let (x, y, w, h) = (
            rect.0.round(),
            rect.1.round(),
            rect.2.round(),
            rect.3.round(),
        );
        if w <= 0.0 || h <= 0.0 || sw <= inset * 2.0 || sh <= inset * 2.0 {
            return true;
        }
        let ix = inset.min((w * 0.5).floor());
        let iy = inset.min((h * 0.5).floor());
        let du = (uv[2] - uv[0]) / sw;
        let dv = (uv[3] - uv[1]) / sh;
        // Source spans per column/row: the outer pieces crop towards their
        // own side when the rectangle is smaller than two insets.
        let src_x = [(0.0, ix), (inset, sw - inset), (sw - ix, sw)];
        let src_y = [(0.0, iy), (inset, sh - inset), (sh - iy, sh)];
        let dst_x = [(x, x + ix), (x + ix, x + w - ix), (x + w - ix, x + w)];
        let dst_y = [(y, y + iy), (y + iy, y + h - iy), (y + h - iy, y + h)];
        for row in 0..3 {
            let (dy0, dy1) = dst_y[row];
            let (sy0, sy1) = src_y[row];
            let period_y = sy1 - sy0;
            if dy1 <= dy0 || period_y <= 0.0 {
                continue;
            }
            for col in 0..3 {
                let (dx0, dx1) = dst_x[col];
                let (sx0, sx1) = src_x[col];
                let period_x = sx1 - sx0;
                if dx1 <= dx0 || period_x <= 0.0 {
                    continue;
                }
                if stretch {
                    let quad = [
                        uv[0] + sx0 * du,
                        uv[1] + sy0 * dv,
                        uv[0] + sx1 * du,
                        uv[1] + sy1 * dv,
                    ];
                    self.sprite(
                        width,
                        height,
                        dx0,
                        dy0,
                        dx1 - dx0,
                        dy1 - dy0,
                        quad,
                        tint,
                        alpha,
                    );
                    continue;
                }
                let mut ty = dy0;
                while ty < dy1 {
                    let th = period_y.min(dy1 - ty);
                    let mut tx = dx0;
                    while tx < dx1 {
                        let tw = period_x.min(dx1 - tx);
                        let quad = [
                            uv[0] + sx0 * du,
                            uv[1] + sy0 * dv,
                            uv[0] + (sx0 + tw) * du,
                            uv[1] + (sy0 + th) * dv,
                        ];
                        self.sprite(width, height, tx, ty, tw, th, quad, tint, alpha);
                        tx += tw;
                    }
                    ty += th;
                }
            }
        }
        true
    }

    /// An OSRS window: the stone background inside the riveted border.
    pub(super) fn stone_frame(&mut self, width: f32, height: f32, rect: Rect) {
        if !self.tiled_nine(
            width,
            height,
            rect,
            "frame_outer",
            FRAME_INSET,
            [255, 255, 255],
            1.0,
        ) {
            let (x, y, w, h) = rect;
            self.quad(width, height, x, y, w, h, [62, 54, 44]);
            self.rect_outline(width, height, x, y, w, h, 2.0, [24, 20, 16], 1.0);
        }
    }

    /// A recessed panel inside a window: darker stone, thin border.
    pub(super) fn stone_inset(&mut self, width: f32, height: f32, rect: Rect) {
        if !self.tiled_nine(
            width,
            height,
            rect,
            "frame_inner",
            FRAME_INSET,
            [255, 255, 255],
            1.0,
        ) {
            let (x, y, w, h) = rect;
            self.quad(width, height, x, y, w, h, [40, 34, 27]);
            self.rect_outline(width, height, x, y, w, h, 1.0, [90, 80, 62], 1.0);
        }
    }

    /// Parchment inside its stone rim (the chatbox scroll).
    pub(super) fn parchment(&mut self, width: f32, height: f32, rect: Rect) {
        if !self.nine(
            width,
            height,
            rect,
            "parchment",
            PARCHMENT_INSET,
            [255, 255, 255],
            1.0,
            true,
        ) {
            let (x, y, w, h) = rect;
            self.quad(width, height, x, y, w, h, [200, 184, 148]);
        }
    }

    /// The notched OSRS button: grey, or red when `red` (a chosen tab or the
    /// main action). Hovering lightens it.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn osrs_button(
        &mut self,
        width: f32,
        height: f32,
        rect: Rect,
        red: bool,
        hot: bool,
        thin: bool,
    ) {
        let key = match (red, thin) {
            (false, false) => "button_grey",
            (true, false) => "button_red",
            (false, true) => "button_grey_thin",
            (true, true) => "button_red_thin",
        };
        let tint = if hot {
            [255, 255, 255]
        } else {
            [222, 222, 222]
        };
        if !self.tiled_nine(width, height, rect, key, BUTTON_INSET, tint, 1.0) {
            let (x, y, w, h) = rect;
            self.quad(
                width,
                height,
                x,
                y,
                w,
                h,
                if red { [120, 30, 26] } else { [86, 86, 84] },
            );
        }
    }

    /// A dark item or portrait slot.
    pub(super) fn item_slot(&mut self, width: f32, height: f32, rect: Rect, tint: [u8; 3]) {
        let (x, y, w, h) = rect;
        match self.atlas.ui_uv("slot") {
            Some(uv) => self.sprite(width, height, x, y, w, h, uv, tint, 1.0),
            None => self.quad(width, height, x, y, w, h, [30, 26, 20]),
        }
    }

    /// Centre `text` in `rect` with a named font.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn label_in(
        &mut self,
        width: f32,
        height: f32,
        rect: Rect,
        font: &str,
        scale: f32,
        text: &str,
        color: [u8; 3],
    ) {
        let (x, y, w, h) = rect;
        let tw = self.text_width_named(font, text, scale);
        let snapped = scale.round().max(1.0);
        let th = self.font_ascent_named(font) * snapped;
        self.text_named(
            font,
            width,
            height,
            (x + (w - tw) * 0.5).round(),
            (y + (h - th) * 0.5).round(),
            scale,
            text,
            color,
        );
    }
}
