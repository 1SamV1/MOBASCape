//! What the dump tool makes of a decoded sprite or font: the pictures and
//! the lines of report that let a person see whether the decoder is right
//! (docs/INTERFACE.md, stage 1). Nothing here reads the cache.

use rift_cache::{Font, SpriteSheet};

use crate::picture::{CLEAR, DARK, GREY, MAGENTA, Picture, RED, WHITE, YELLOW};

/// The line every font is drawn with, as the design asks.
pub const TEST_LINE: &str = "The quick brown fox 0123456789 Oh dear, you are dead!";

/// Pictures to save, each with its path under the output folder, and the
/// lines to add to the report.
#[derive(Default)]
pub struct Dump {
    pub files: Vec<(String, Picture)>,
    pub report: String,
}

/// One frame on its own, at its own size.
pub fn frame_picture(sheet: &SpriteSheet, frame: usize) -> Picture {
    let f = &sheet.frames[frame];
    Picture::from_pixels(f.width, f.height, f.rgba(&sheet.palette))
}

/// The heading of the sprite report.
pub fn sprite_report_heading() -> String {
    "id\tname\tframes\tsheet\tpalette\tunread\tframe\tsize\toffset\tstored\topacity\tclear pixels\n"
        .to_string()
}

pub fn dump_sprite(sheet: &SpriteSheet, name: &str) -> Dump {
    let mut dump = Dump::default();
    let id = sheet.id;
    if sheet.frames.is_empty() {
        dump.report.push_str(&format!(
            "{id}\t{name}\t0\t{}x{}\t{}\t{}\t-\t-\t-\t-\t-\t-\n",
            sheet.width,
            sheet.height,
            sheet.palette.len(),
            sheet.unread
        ));
    }
    for (i, frame) in sheet.frames.iter().enumerate() {
        dump.report.push_str(&format!(
            "{id}\t{name}\t{}\t{}x{}\t{}\t{}\t{i}\t{}x{}\t{},{}\t{}\t{}\t{} of {}\n",
            sheet.frames.len(),
            sheet.width,
            sheet.height,
            sheet.palette.len(),
            sheet.unread,
            frame.width,
            frame.height,
            frame.offset_x,
            frame.offset_y,
            if frame.by_column {
                "by column"
            } else {
                "by row"
            },
            if frame.has_alpha { "given" } else { "made" },
            frame.clear_pixels(),
            frame.width * frame.height,
        ));
        if frame.width == 0 || frame.height == 0 {
            continue;
        }
        let picture = frame_picture(sheet, i);
        dump.files.push((
            format!("sprites_magenta/{id:05}_{i}.png"),
            picture.over(MAGENTA),
        ));
        dump.files
            .push((format!("sprites/{id:05}_{i}.png"), picture));
    }

    // every frame in its place within the sheet's full size, side by side,
    // so their alignment against each other can be seen
    let (cell_w, cell_h) = (sheet.width.max(1), sheet.height.max(1));
    if sheet.frames.len() > 1 && cell_w * sheet.frames.len() <= 16384 && cell_h <= 4096 {
        let mut strip = Picture::new((cell_w + 1) * sheet.frames.len() + 1, cell_h + 2, GREY);
        for (i, frame) in sheet.frames.iter().enumerate() {
            let x = (1 + i * (cell_w + 1)) as i32;
            strip.fill(x, 1, cell_w, cell_h, MAGENTA);
            if frame.width != 0 && frame.height != 0 {
                strip.draw(
                    &frame_picture(sheet, i),
                    x + frame.offset_x as i32,
                    1 + frame.offset_y as i32,
                );
            }
        }
        dump.files.push((format!("strips/{id:05}.png"), strip));
    }

    // the palette as swatches: red first means red was read as red
    const SWATCH: usize = 10;
    let across = 16usize;
    let down = sheet.palette.len().div_ceil(across).max(1);
    let mut swatches = Picture::new(across * SWATCH, down * SWATCH, DARK);
    for (i, &colour) in sheet.palette.iter().enumerate() {
        let (x, y) = ((i % across * SWATCH) as i32, (i / across * SWATCH) as i32);
        if i == 0 {
            // entry 0 is "nothing": a magenta square with a mark through it
            swatches.fill(x, y, SWATCH - 1, SWATCH - 1, MAGENTA);
            for k in 0..(SWATCH - 1) as i32 {
                swatches.set(x + k, y + k, DARK);
            }
        } else {
            swatches.fill(
                x,
                y,
                SWATCH - 1,
                SWATCH - 1,
                [(colour >> 16) as u8, (colour >> 8) as u8, colour as u8, 255],
            );
        }
    }
    dump.files.push((format!("palettes/{id:05}.png"), swatches));
    dump
}

/// What one sprite shows of itself on an overview sheet.
pub struct Thumb {
    pub id: u32,
    pub frames: usize,
    pub size: (usize, usize),
    pub first: Option<Picture>,
}

impl Thumb {
    pub fn of(sheet: &SpriteSheet) -> Self {
        let first = sheet
            .frames
            .iter()
            .position(|f| f.width != 0 && f.height != 0);
        Self {
            id: sheet.id,
            frames: sheet.frames.len(),
            size: first
                .map(|i| (sheet.frames[i].width, sheet.frames[i].height))
                .unwrap_or((0, 0)),
            first: first.map(|i| frame_picture(sheet, i)),
        }
    }
}

/// Many sprites on one sheet, each with its id, so that a panel or an icon
/// can be found by looking. Big sprites are shown smaller, by whole steps.
pub fn overview(thumbs: &[Thumb]) -> Picture {
    const CELL: usize = 72;
    const LABEL: usize = 14;
    let across = 10usize;
    let down = thumbs.len().div_ceil(across).max(1);
    let mut sheet = Picture::new(across * (CELL + 1) + 1, down * (CELL + LABEL + 1) + 1, GREY);
    for (i, thumb) in thumbs.iter().enumerate() {
        let x = (1 + i % across * (CELL + 1)) as i32;
        let y = (1 + i / across * (CELL + LABEL + 1)) as i32;
        sheet.fill(x, y, CELL, LABEL, DARK);
        sheet.label(&thumb.id.to_string(), x + 2, y + 1, YELLOW);
        let mut about = format!("{}x{}", thumb.size.0, thumb.size.1);
        if thumb.frames != 1 {
            about.push_str(&format!(" {}", thumb.frames));
        }
        sheet.label(&about, x + 2, y + 8, WHITE);
        sheet.fill(x, y + LABEL as i32, CELL, CELL, MAGENTA);
        if let Some(first) = &thumb.first {
            // big ones smaller, small ones up to three times larger, always
            // by whole steps
            let mut small = first.fitted(CELL, CELL);
            let times = (CELL / small.width.max(1))
                .min(CELL / small.height.max(1))
                .min(3);
            if times > 1 {
                let mut large = Picture::new(small.width * times, small.height * times, CLEAR);
                for py in 0..large.height {
                    for px in 0..large.width {
                        large.pixels[py * large.width + px] = small.get(px / times, py / times);
                    }
                }
                small = large;
            }
            sheet.draw(
                &small,
                x + ((CELL - small.width) / 2) as i32,
                y + (LABEL + (CELL - small.height) / 2) as i32,
            );
        }
    }
    sheet
}

/// The heading of the font report.
pub fn font_report_heading() -> String {
    "font\tname\tbaseline\tabove\tbelow\tnote\tcharacter\tsize\toffset\tadvance\n".to_string()
}

pub fn dump_font(font: &Font, letters: &SpriteSheet, name: &str) -> Dump {
    let mut dump = Dump::default();
    let id = font.id;
    let note = font.note.as_deref().unwrap_or("");
    for (code, glyph) in font.glyphs.iter().enumerate() {
        dump.report.push_str(&format!(
            "{id}\t{name}\t{}\t{}\t{}\t{note}\t{code}\t{}x{}\t{},{}\t{}\n",
            font.ascent,
            font.max_ascent,
            font.max_descent,
            glyph.width,
            glyph.height,
            glyph.left,
            glyph.top,
            glyph.advance
        ));
    }
    let glyph_picture = |code: usize| -> Option<Picture> {
        let frame = letters.frames.get(code)?;
        (frame.width != 0 && frame.height != 0).then(|| frame_picture(letters, code))
    };

    // every character in a grid of sixteen by sixteen, each in a cell the
    // size of the sheet, at its own offsets, with its number above it
    const LABEL: usize = 7;
    let (cell_w, cell_h) = (letters.width.max(12) + 2, letters.height.max(1) + 2);
    let mut grid = Picture::new(16 * (cell_w + 1) + 1, 16 * (cell_h + LABEL + 1) + 1, GREY);
    for code in 0..256usize {
        let x = (1 + code % 16 * (cell_w + 1)) as i32;
        let y = (1 + code / 16 * (cell_h + LABEL + 1)) as i32;
        grid.fill(x, y, cell_w, LABEL, DARK);
        grid.label(&code.to_string(), x + 1, y + 1, YELLOW);
        grid.fill(x, y + LABEL as i32, cell_w, cell_h, DARK);
        // the baseline, as the measurements have it
        grid.fill(x, y + LABEL as i32 + 1 + font.ascent, cell_w, 1, RED);
        if let Some(picture) = glyph_picture(code) {
            let glyph = font.glyphs[code];
            grid.stamp(
                &picture,
                x + 1 + glyph.left,
                y + LABEL as i32 + 1 + glyph.top,
                WHITE,
            );
        }
    }
    dump.files.push((format!("fonts/{id:05}_glyphs.png"), grid));

    // the test line, placed by the decoder's own spacing, over the baseline
    // it should stand on, with a tick at the pen's start and end
    let width = font.width(TEST_LINE).max(1) as usize;
    let above = font.max_ascent.max(font.ascent).max(1);
    let below = font.max_descent.max(1);
    let margin = 8;
    let baseline = margin + above;
    let mut line = Picture::new(
        width + 2 * margin as usize,
        (baseline + below + margin) as usize,
        DARK,
    );
    line.fill(0, baseline, line.width, 1, RED);
    line.fill(margin, baseline - above, 1, (above + below) as usize, GREY);
    line.fill(
        margin + width as i32,
        baseline - above,
        1,
        (above + below) as usize,
        GREY,
    );
    for placed in font.place(TEST_LINE, margin, baseline) {
        if let Some(picture) = glyph_picture(placed.glyph) {
            line.stamp(&picture, placed.x, placed.y, YELLOW);
        }
    }
    dump.files
        .push((format!("fonts/{id:05}_line.png"), line.clone()));
    // and three times the size, each pixel a square, to see it properly
    let mut big = Picture::new(line.width * 3, line.height * 3, CLEAR);
    for y in 0..big.height {
        for x in 0..big.width {
            big.pixels[y * big.width + x] = line.get(x / 3, y / 3);
        }
    }
    dump.files.push((format!("fonts/{id:05}_line_x3.png"), big));
    dump
}

/// Names of fonts and sprites worth knowing, to match against the hashes
/// the cache keeps in place of names. A name not here is not an error: the
/// report shows the hash instead.
pub const KNOWN_NAMES: &[&str] = &[
    "p11_full",
    "p12_full",
    "b12_full",
    "q8_full",
    "quill_oblique_large",
    "quill_caps_large",
    "lunar_alphabet",
    "lunar_alphabet_lrg",
    "barbarian",
    "surok",
    "verdana_11pt_regular",
    "verdana_11pt_bold",
    "verdana_13pt_regular",
    "verdana_13pt_bold",
    "verdana_15pt_regular",
    "verdana_15pt_bold",
    "tahoma_11pt_regular",
    "tahoma_11pt_bold",
    "fairy_small",
    "fairy_large",
    "mod_icons",
    "headicons_pk",
    "headicons_prayer",
    "headicons_hint",
    "mapfunction",
    "mapscene",
    "mapdots",
    "mapmarker",
    "scrollbar",
    "cross",
    "compass",
    "mapedge",
    "invback",
    "chatback",
    "mapback",
    "backbase1",
    "backbase2",
    "backhmid1",
    "backhmid2",
    "backleft1",
    "backleft2",
    "backright1",
    "backright2",
    "backtop1",
    "backvmid1",
    "backvmid2",
    "backvmid3",
    "sideicons",
    "redstone1",
    "redstone2",
    "redstone3",
    "titlebox",
    "titlebutton",
    "logo",
    "runes",
    "magicon",
    "magicoff",
    "prayeron",
    "prayeroff",
    "combaticons",
    "staticons",
    "miscgraphics",
    "worldmap_icon",
    "options_icons",
];

/// The name behind a hash, if it is one of the known ones.
pub fn name_of(hash: i32) -> String {
    KNOWN_NAMES
        .iter()
        .find(|name| rift_cache::sprite::name_hash(name) == hash)
        .map(|name| name.to_string())
        .unwrap_or_else(|| format!("#{hash}"))
}
