//! Draws a list of quads into a plain picture, the way the graphics card is
//! asked to: for each window pixel, the atlas pixel under its middle,
//! multiplied by the quad's colour and laid over what is there by its
//! opacity. It exists so the drawing layer can be tested, and its pictures
//! looked at, without a graphics card.
//!
//! The one way it differs: it mixes colours as the bytes they are stored as,
//! where the graphics card mixes light. A see-through edge comes out a
//! shade different; a solid pixel comes out the same.

use crate::atlas::Atlas;
use crate::draw::Vertex;

/// Draw onto `picture`, which is `width` by `height` pixels of red, green,
/// blue and opacity.
pub fn render(
    vertices: &[Vertex],
    indices: &[u32],
    atlas: &Atlas,
    width: usize,
    height: usize,
    picture: &mut [[u8; 4]],
) {
    assert_eq!(picture.len(), width * height);
    // every quad is six indices over four corners: top left, top right,
    // bottom left, bottom right
    for quad in indices.chunks(6) {
        let (tl, br) = (&vertices[quad[0] as usize], &vertices[quad[5] as usize]);
        let (x0, y0, x1, y1) = (tl.pos[0], tl.pos[1], br.pos[0], br.pos[1]);
        let from_x = (x0 - 0.5).ceil().max(0.0) as usize;
        let from_y = (y0 - 0.5).ceil().max(0.0) as usize;
        let mut py = from_y;
        while (py as f32 + 0.5) < y1 && py < height {
            let v = tl.uv[1] + (py as f32 + 0.5 - y0) / (y1 - y0) * (br.uv[1] - tl.uv[1]);
            let mut px = from_x;
            while (px as f32 + 0.5) < x1 && px < width {
                let u = tl.uv[0] + (px as f32 + 0.5 - x0) / (x1 - x0) * (br.uv[0] - tl.uv[0]);
                let texel = atlas.pixel(u.floor() as i64, v.floor() as i64);
                let colour: Vec<f32> = (0..4)
                    .map(|k| texel[k] as f32 / 255.0 * tl.colour[k])
                    .collect();
                let under = &mut picture[py * width + px];
                let a = colour[3].clamp(0.0, 1.0);
                for k in 0..3 {
                    let mixed = colour[k].clamp(0.0, 1.0) * 255.0 * a + under[k] as f32 * (1.0 - a);
                    under[k] = mixed.round() as u8;
                }
                under[3] = 255;
                px += 1;
            }
            py += 1;
        }
    }
}
