//! The main menu the game boots into: Bot Match, Public Match, Private
//! Match (and Quit), over a slow cinematic orbit of the arena.
//!
//! Public and private matches need the match server (M7/M8), which does not
//! exist yet. Until it does, both modes run the same local match as Bot
//! Match, with bots in the open slots, and the screens say so.

/// How the match the player picked is filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchMode {
    Bot,
    Public,
    Private,
}

impl MatchMode {
    pub fn label(self) -> &'static str {
        match self {
            MatchMode::Bot => "BOT MATCH",
            MatchMode::Public => "PUBLIC MATCH",
            MatchMode::Private => "PRIVATE MATCH",
        }
    }

    /// One line under the button.
    pub fn blurb(self) -> &'static str {
        match self {
            MatchMode::Bot => "5v5 against bots. Practice your hero.",
            MatchMode::Public => "Queue with other players. Servers offline: bots fill.",
            MatchMode::Private => "Invite friends with a code. Servers offline: bots fill.",
        }
    }

    /// True when the mode needs the match server, which is not online yet.
    pub fn needs_server(self) -> bool {
        !matches!(self, MatchMode::Bot)
    }
}

/// What a menu button does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Play(MatchMode),
    Quit,
}

/// The buttons, top to bottom.
pub const BUTTONS: [MenuAction; 4] = [
    MenuAction::Play(MatchMode::Bot),
    MenuAction::Play(MatchMode::Public),
    MenuAction::Play(MatchMode::Private),
    MenuAction::Quit,
];

pub struct MainMenu {
    pub open: bool,
    /// Button under the cursor, or the keyboard focus.
    pub hover: Option<usize>,
    /// Keyboard focus (arrow keys); the mouse moves it too.
    pub focus: usize,
    /// Seconds on the menu: drives the flames, the title glow and the orbit.
    pub time: f32,
}

impl MainMenu {
    pub fn new(open: bool) -> Self {
        Self {
            open,
            hover: None,
            focus: 0,
            time: 0.0,
        }
    }

    pub fn move_focus(&mut self, delta: isize) {
        let count = BUTTONS.len() as isize;
        self.focus = (self.focus as isize + delta).rem_euclid(count) as usize;
    }

    pub fn focused(&self) -> MenuAction {
        BUTTONS[self.focus.min(BUTTONS.len() - 1)]
    }
}

/// The stone tablet behind the title and buttons.
pub fn tablet_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let w = (width * 0.36).clamp(380.0, 480.0);
    let h = 62.0 + 3.0 * (52.0 + 22.0) + 6.0 + 34.0 + 22.0;
    let top = (height * 0.30).max(200.0).min(height - h - 70.0);
    ((width - w) * 0.5, top, w, h)
}

/// Button rectangles, in [`BUTTONS`] order. Quit is a smaller button.
pub fn button_rects(width: f32, height: f32) -> Vec<(f32, f32, f32, f32)> {
    let (tx, ty, tw, th) = tablet_rect(width, height);
    let bw = tw - 64.0;
    let bh = 52.0;
    let gap = 22.0;
    let _ = th;
    let top = ty + 62.0;
    let mut rects: Vec<_> = (0..3)
        .map(|i| (tx + 32.0, top + i as f32 * (bh + gap), bw, bh))
        .collect();
    let qw = bw * 0.42;
    rects.push((tx + (tw - qw) * 0.5, top + 3.0 * (bh + gap) + 6.0, qw, 34.0));
    rects
}

pub fn button_at(width: f32, height: f32, cursor: (f64, f64)) -> Option<usize> {
    let (mx, my) = (cursor.0 as f32, cursor.1 as f32);
    button_rects(width, height)
        .iter()
        .position(|&(x, y, w, h)| mx >= x && mx <= x + w && my >= y && my <= y + h)
}

/// Deterministic 0..1 noise for the flame sprites.
pub fn hash01(a: u32, b: u32) -> f32 {
    let mut h = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h & 0xFFFF) as f32 / 65535.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_stack_inside_the_tablet_without_overlapping() {
        for (w, h) in [(1280.0, 720.0), (1920.0, 1080.0), (1024.0, 600.0)] {
            let (tx, ty, tw, th) = tablet_rect(w, h);
            let rects = button_rects(w, h);
            assert_eq!(rects.len(), BUTTONS.len());
            for pair in rects.windows(2) {
                assert!(pair[0].1 + pair[0].3 < pair[1].1, "{w}x{h} overlap");
            }
            for &(x, y, bw, bh) in &rects {
                assert!(x >= tx && x + bw <= tx + tw, "{w}x{h} x");
                assert!(y >= ty && y + bh <= ty + th, "{w}x{h} y");
            }
        }
    }

    #[test]
    fn clicks_and_keys_pick_the_same_buttons() {
        let rects = button_rects(1280.0, 720.0);
        let (x, y, w, h) = rects[1];
        let hit = button_at(1280.0, 720.0, ((x + w * 0.5) as f64, (y + h * 0.5) as f64));
        assert_eq!(hit, Some(1));
        assert_eq!(BUTTONS[1], MenuAction::Play(MatchMode::Public));
        let mut menu = MainMenu::new(true);
        menu.move_focus(-1);
        assert_eq!(menu.focused(), MenuAction::Quit);
        menu.move_focus(1);
        assert_eq!(menu.focused(), MenuAction::Play(MatchMode::Bot));
        assert!(button_at(1280.0, 720.0, (2.0, 2.0)).is_none());
    }

    #[test]
    fn only_the_bot_match_runs_without_a_server() {
        assert!(!MatchMode::Bot.needs_server());
        assert!(MatchMode::Public.needs_server() && MatchMode::Private.needs_server());
    }
}
