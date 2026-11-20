//! The placing pass and the collapse order of the stage 3 interface
//! (docs/INTERFACE_STAGE3_PLAN.md, sections 5 to 8). It is a pure function of
//! the window, the scale and the plain data in [`HudState`]; it draws
//! nothing and reads nothing else.
//!
//! Every component is placed from its anchor and margins. If pieces clash,
//! the row or column that is wrong settles it, then the budget may shrink or
//! remove pieces. The result is a rectangle per piece and the list of what
//! gave way, which is what the tests and the labelled step B screen read.

use std::collections::BTreeMap;

use crate::draw::Rect;
use crate::hud::state::HudState;

/// The clearance every pair of pieces keeps, in interface pixels.
pub const GAP: i32 = 8;
/// The most of the window the usual state may cover.
pub const BUDGET: f64 = 0.25;

pub const INVENTORY: (i32, i32) = (144, 160);
pub const CHAT: (i32, i32) = (440, 140);
pub const CHAT_LAST_LINE: i32 = 24;
pub const ABILITY: (i32, i32) = (458, 94);
pub const MINIMAP: (i32, i32) = (266, 264);
pub const SCORE: (i32, i32) = (534, 38);
pub const TARGET: (i32, i32) = (332, 158);
pub const OBJECTIVES_WIDTH: i32 = 272;
pub const FEED_WIDTH: i32 = 286;
pub const TEAM: (i32, i32) = (180, 188);
pub const TEAM_COMPACT_WIDTH: i32 = 60;

/// A piece of the fixed interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Piece {
    Score,
    Feed,
    Team,
    Target,
    Objectives,
    Chat,
    Ability,
    Inventory,
    Minimap,
}

impl Piece {
    pub fn name(self) -> &'static str {
        match self {
            Self::Score => "score",
            Self::Feed => "feed",
            Self::Team => "team",
            Self::Target => "target",
            Self::Objectives => "objectives",
            Self::Chat => "chat",
            Self::Ability => "ability",
            Self::Inventory => "inventory",
            Self::Minimap => "minimap",
        }
    }
}

/// One way the placed rectangles are wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conflict {
    /// A piece leaves the window.
    Window(Piece),
    /// Two pieces are closer than [`GAP`], across and down.
    Pair(Piece, Piece),
}

impl Conflict {
    fn within(self, group: &[Piece]) -> bool {
        match self {
            Self::Window(piece) => group.contains(&piece),
            Self::Pair(a, b) => group.contains(&a) && group.contains(&b),
        }
    }

    fn touches(self, piece: Piece) -> bool {
        match self {
            Self::Window(p) => p == piece,
            Self::Pair(a, b) => a == piece || b == piece,
        }
    }

    fn is_pair(self, a: Piece, b: Piece) -> bool {
        matches!(self, Self::Pair(x, y) if (x == a && y == b) || (x == b && y == a))
    }
}

/// What the caller wants shown while play goes on. The usual state and the
/// busy state are two of these, so the scale and the budget read the same
/// rectangles as the finished screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct View {
    target: bool,
    feed: usize,
    /// The objectives height asked for.
    objectives: i32,
    /// The objectives height the budget is measured on.
    usual_objectives: i32,
}

impl View {
    fn actual(state: &HudState) -> Self {
        Self {
            target: state.target.is_some(),
            feed: state.feed.len(),
            objectives: state.objectives.wanted_height(),
            usual_objectives: state.objectives.usual_height(),
        }
    }

    fn busy(state: &HudState) -> Self {
        Self {
            target: true,
            feed: 4,
            objectives: state.objectives.longest_height(),
            usual_objectives: state.objectives.usual_height(),
        }
    }

    fn usual(state: &HudState) -> Self {
        Self {
            target: false,
            feed: 1,
            objectives: state.objectives.usual_height(),
            usual_objectives: state.objectives.usual_height(),
        }
    }
}

/// The mutable state of one collapse pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Placing {
    inventory: bool,
    chat_width: i32,
    chat_height: i32,
    target: bool,
    objectives_height: i32,
    feed: usize,
    feed_y: i32,
    score: bool,
    team: bool,
    team_compact: bool,
    slide: bool,
}

impl Placing {
    fn new(view: View, slide: bool) -> Self {
        Self {
            inventory: true,
            chat_width: CHAT.0,
            chat_height: CHAT.1,
            target: false,
            objectives_height: view.usual_objectives,
            feed: view.feed.min(1),
            feed_y: 18,
            score: true,
            team: true,
            team_compact: false,
            slide,
        }
    }
}

/// The finished placing: the scale drawn, every rectangle, what gave way,
/// and the usual state's share of the window.
#[derive(Debug, Clone)]
pub struct Layout {
    pub window: (u32, u32),
    pub scale: u32,
    pub requested: Option<u32>,
    pub pieces: BTreeMap<Piece, Rect>,
    pub steps: Vec<String>,
    pub conflicts: Vec<Conflict>,
    /// The part of the window the finished rectangles cover.
    pub coverage: f64,
    /// The usual state's share, which the quarter is measured on.
    pub usual_coverage: f64,
}

impl Layout {
    pub fn get(&self, piece: Piece) -> Option<Rect> {
        self.pieces.get(&piece).copied()
    }
}

/// Place the interface for this window and state. `requested` is a player's
/// scale request; `None` chooses the scale by fit.
pub fn place(state: &HudState, window: (u32, u32), requested: Option<u32>) -> Layout {
    let scale = effective_scale(state, window, requested);
    place_at(state, window, scale, requested)
}

/// Place at exactly this scale, even where the window cannot hold it. This
/// is for the edge tests, not for the game's normal path.
pub fn place_at(
    state: &HudState,
    window: (u32, u32),
    scale: u32,
    requested: Option<u32>,
) -> Layout {
    let view = View::actual(state);
    let collapsed = collapse(window, scale, BUDGET, view, true);
    let pieces = layout(window, scale, collapsed.placing);
    let covered = coverage(&pieces, window);
    let usual = collapse(window, scale, BUDGET, View::usual(state), true);
    let usual_coverage = coverage(&layout(window, scale, usual.placing), window);
    Layout {
        window,
        scale,
        requested,
        pieces,
        steps: collapsed.steps,
        conflicts: collapsed.conflicts,
        coverage: covered,
        usual_coverage,
    }
}

/// The largest whole scale, up to four, at which the whole interface fits
/// with nothing given up. The busy state is everything at its largest and
/// the usual state must be within the budget.
pub fn choose_scale(state: &HudState, window: (u32, u32), budget: f64) -> u32 {
    for scale in [4, 3, 2] {
        if fits_full(state, window, scale, budget) {
            return scale;
        }
    }
    1
}

pub fn fits_full(state: &HudState, window: (u32, u32), scale: u32, budget: f64) -> bool {
    let busy = busy_placing(View::busy(state));
    if !conflicts(&layout(window, scale, busy), window, scale).is_empty() {
        return false;
    }
    let usual = usual_placing(View::usual(state));
    coverage(&layout(window, scale, usual), window) <= budget
}

/// The scale actually drawn. A request is kept where the window can hold
/// it; where it cannot, the largest lower scale the window can hold is
/// drawn, and the request itself is not changed.
pub fn effective_scale(state: &HudState, window: (u32, u32), requested: Option<u32>) -> u32 {
    let Some(requested) = requested else {
        return choose_scale(state, window, BUDGET);
    };
    for scale in (2..=requested.clamp(1, 4)).rev() {
        if holds(state, window, scale) {
            return scale;
        }
    }
    1
}

/// Can the window hold the layout at this scale with every piece given up
/// that may be, and no clash left? Asked of the usual state.
pub fn holds(state: &HudState, window: (u32, u32), scale: u32) -> bool {
    collapse(window, scale, BUDGET, View::usual(state), true)
        .conflicts
        .is_empty()
}

fn busy_placing(view: View) -> Placing {
    Placing {
        target: view.target,
        feed: view.feed,
        objectives_height: view.objectives,
        ..Placing::new(view, false)
    }
}

fn usual_placing(view: View) -> Placing {
    Placing {
        target: false,
        feed: 1,
        objectives_height: view.usual_objectives,
        ..Placing::new(view, false)
    }
}

struct Collapsed {
    placing: Placing,
    steps: Vec<String>,
    conflicts: Vec<Conflict>,
}

fn collapse(window: (u32, u32), scale: u32, budget: f64, view: View, slide: bool) -> Collapsed {
    let mut st = Placing::new(view, slide);
    let mut steps = Vec::new();
    let bottom = [
        Piece::Chat,
        Piece::Ability,
        Piece::Inventory,
        Piece::Minimap,
    ];
    let right = [Piece::Feed, Piece::Team, Piece::Minimap, Piece::Score];

    // --- pass 1: the layout, from the window alone
    // The bottom row is settled in one go, from the right: the minimap
    // stays, the inventory sits against it, the ability bar is centred or
    // slid to 8 from what is on its right, and the chat takes what is left.
    if bad(window, scale, st, &bottom) {
        let tries = [
            (true, CHAT.1, 280),
            (true, CHAT_LAST_LINE, 200),
            (false, CHAT.1, 280),
            (false, CHAT_LAST_LINE, 200),
        ];
        'arrangements: for (inventory, height, least) in tries {
            st.inventory = inventory;
            st.chat_height = height;
            for width in (least..=CHAT.0).rev() {
                st.chat_width = width;
                if !bad(window, scale, st, &bottom) {
                    break 'arrangements;
                }
            }
        }
        if st.chat_width < CHAT.0 {
            steps.push(format!("chat narrows to {}", st.chat_width));
        }
        if st.chat_height == CHAT_LAST_LINE {
            steps.push("chat to its last line".to_string());
        }
        if !st.inventory {
            steps.push("inventory hidden".to_string());
        }
    }

    // The objectives are asked for only what their room can hold: the list,
    // else the title, else nothing.
    let room = objectives_room(window, scale, st);
    if st.objectives_height * scale as i32 > room {
        st.objectives_height = 24;
        steps.push("objectives to its title".to_string());
    }
    if st.objectives_height * scale as i32 > room {
        st.objectives_height = 0;
        steps.push("objectives hidden".to_string());
    }

    // The score strip and the kill feed meet along the top: the feed moves
    // down under the strip.
    if bad(window, scale, st, &right)
        && conflicts(&layout(window, scale, st), window, scale)
            .iter()
            .any(|c| c.is_pair(Piece::Score, Piece::Feed))
    {
        st.feed_y = 56;
        steps.push("kill feed moves under the score strip".to_string());
    }

    // A piece of the top row the minimap reaches is not shown: the kill feed
    // first, then the score strip. The minimap is never hidden.
    if st.feed > 0 && near_minimap(window, scale, st, Piece::Feed) {
        st.feed = 0;
        steps.push("kill feed not shown: the minimap reaches it".to_string());
    }
    if st.score && near_minimap(window, scale, st, Piece::Score) {
        st.score = false;
        steps.push("score strip not shown: the minimap reaches it".to_string());
    }

    if bad(window, scale, st, &right) {
        st.team_compact = true;
        steps.push("team frames to portraits".to_string());
    }
    if bad(window, scale, st, &right) {
        st.team = false;
        steps.push("team frames hidden".to_string());
    }

    // Only now is the budget looked at. Each step shrinks or removes one
    // piece, and after every step the figure is read afresh.
    for step in 0..6 {
        if coverage(&layout(window, scale, st), window) <= budget {
            break;
        }
        match step {
            0 if st.objectives_height > 24 => {
                st.objectives_height = 24;
                steps.push("objectives to its title (budget)".to_string());
            }
            1 if !st.team_compact => {
                st.team_compact = true;
                steps.push("team frames to portraits (budget)".to_string());
            }
            2 if st.chat_height > CHAT_LAST_LINE => {
                st.chat_height = CHAT_LAST_LINE;
                steps.push("chat to its last line (budget)".to_string());
            }
            3 if st.feed > 0 => {
                st.feed = 0;
                steps.push("kill feed hidden (budget)".to_string());
            }
            4 if st.score => {
                st.score = false;
                steps.push("score strip hidden (budget)".to_string());
            }
            5 if st.inventory => {
                st.inventory = false;
                steps.push("inventory hidden (budget)".to_string());
            }
            _ => {}
        }
    }

    // A budget step may have shortened the chat and made room. If the
    // objectives were hidden for want of room, their title comes back when
    // it now fits and the budget allows.
    if st.objectives_height == 0 {
        st.objectives_height = 24;
        let room = objectives_room(window, scale, st);
        if st.objectives_height * scale as i32 > room
            || coverage(&layout(window, scale, st), window) > budget
        {
            st.objectives_height = 0;
        } else if let Some(at) = steps.iter().position(|step| step == "objectives hidden") {
            steps[at] =
                "objectives hidden, then back to its title once the chat had shrunk".to_string();
        }
    }

    // --- pass 2: what comes and goes with play, each only where it fits
    // The longer objectives list, if there is room for it.
    if st.objectives_height > 24 && view.objectives > st.objectives_height {
        st.objectives_height = view.objectives;
        let room = objectives_room(window, scale, st);
        if st.objectives_height * scale as i32 > room {
            st.objectives_height = 24;
            steps.push("objectives, when long, to its title".to_string());
        }
    }

    // The target panel, clear of every other piece, not only its own column.
    if view.target {
        st.target = true;
        if conflicts(&layout(window, scale, st), window, scale)
            .iter()
            .any(|conflict| conflict.touches(Piece::Target))
        {
            st.target = false;
            steps.push("target panel not shown".to_string());
        }
    }

    // Further kill-feed entries, as many as are clear of every other piece.
    if st.feed > 0 && view.feed > 1 {
        for count in (1..=view.feed).rev() {
            st.feed = count;
            if !conflicts(&layout(window, scale, st), window, scale)
                .iter()
                .any(|conflict| conflict.touches(Piece::Feed))
            {
                break;
            }
        }
        if st.feed < view.feed {
            steps.push(format!(
                "kill feed to {} entr{}",
                st.feed,
                if st.feed == 1 { "y" } else { "ies" }
            ));
        }
    }

    let pieces = layout(window, scale, st);
    Collapsed {
        placing: st,
        steps,
        conflicts: conflicts(&pieces, window, scale),
    }
}

/// The room the objectives may use: from 8 below the target panel's place,
/// which is kept whether the panel shows or not, to 8 above the chat or the
/// ability bar where it has slid under them, whichever is higher.
fn objectives_room(window: (u32, u32), scale: u32, st: Placing) -> i32 {
    let mut without = st;
    without.objectives_height = 0;
    let pieces = layout(window, scale, without);
    let chat = pieces[&Piece::Chat].y;
    let ability = pieces[&Piece::Ability];
    let under = if ability.x < (GAP + OBJECTIVES_WIDTH + GAP) * scale as i32 {
        ability.y.min(chat)
    } else {
        chat
    };
    let top = (82 + TARGET.1 + GAP) * scale as i32;
    (under - GAP * scale as i32 - top).max(0)
}

fn layout(window: (u32, u32), scale: u32, st: Placing) -> BTreeMap<Piece, Rect> {
    let (w, h) = (window.0 as i32, window.1 as i32);
    let s = scale as i32;
    let mut pieces = BTreeMap::new();

    let minimap = Rect::new(
        w - (GAP + MINIMAP.0) * s,
        h - (16 + MINIMAP.1) * s,
        MINIMAP.0 * s,
        MINIMAP.1 * s,
    );
    pieces.insert(Piece::Minimap, minimap);

    if st.inventory {
        pieces.insert(
            Piece::Inventory,
            Rect::new(
                minimap.x - (GAP + INVENTORY.0) * s,
                h - (18 + INVENTORY.1) * s,
                INVENTORY.0 * s,
                INVENTORY.1 * s,
            ),
        );
    }

    pieces.insert(
        Piece::Chat,
        Rect::new(
            5 * s,
            h - (10 + st.chat_height) * s,
            st.chat_width * s,
            st.chat_height * s,
        ),
    );

    let mut ability_x = (w - ABILITY.0 * s).div_euclid(2);
    if st.slide {
        // The bar slides only when it would otherwise come within 8 of what
        // is on its right. The chat's width is worked out after it.
        let right = pieces
            .get(&Piece::Inventory)
            .map(|rect| rect.x)
            .unwrap_or(minimap.x);
        let limit = right - GAP * s - ABILITY.0 * s;
        ability_x = ability_x.min(limit);
    }
    pieces.insert(
        Piece::Ability,
        Rect::new(
            ability_x,
            h - (18 + ABILITY.1) * s,
            ABILITY.0 * s,
            ABILITY.1 * s,
        ),
    );

    if st.score {
        pieces.insert(
            Piece::Score,
            Rect::new(
                (w - SCORE.0 * s).div_euclid(2),
                10 * s,
                SCORE.0 * s,
                SCORE.1 * s,
            ),
        );
    }

    if st.feed > 0 {
        pieces.insert(
            Piece::Feed,
            Rect::new(
                w - (12 + FEED_WIDTH) * s,
                st.feed_y * s,
                FEED_WIDTH * s,
                (30 * st.feed as i32 - 2) * s,
            ),
        );
    }

    if st.team {
        let width = if st.team_compact {
            TEAM_COMPACT_WIDTH
        } else {
            TEAM.0
        };
        pieces.insert(
            Piece::Team,
            Rect::new(w - (12 + width) * s, 168 * s, width * s, TEAM.1 * s),
        );
    }

    if st.target {
        pieces.insert(
            Piece::Target,
            Rect::new(GAP * s, 82 * s, TARGET.0 * s, TARGET.1 * s),
        );
    }

    if st.objectives_height > 0 {
        let room = objectives_room(window, scale, st);
        let room_top = (82 + TARGET.1 + GAP) * s;
        let wanted = (38 * h + 50) / 100;
        let top = room_top.max(wanted.min(room_top + room - st.objectives_height * s));
        pieces.insert(
            Piece::Objectives,
            Rect::new(GAP * s, top, OBJECTIVES_WIDTH * s, st.objectives_height * s),
        );
    }

    pieces
}

fn conflicts(pieces: &BTreeMap<Piece, Rect>, window: (u32, u32), scale: u32) -> Vec<Conflict> {
    let gap = GAP * scale as i32;
    let (w, h) = (window.0 as i32, window.1 as i32);
    let list: Vec<(Piece, Rect)> = pieces.iter().map(|(p, r)| (*p, *r)).collect();
    let mut out = Vec::new();
    for (i, (piece, rect)) in list.iter().enumerate() {
        if rect.x < 0 || rect.y < 0 || rect.right() > w || rect.bottom() > h {
            out.push(Conflict::Window(*piece));
        }
        for (other, their) in &list[i + 1..] {
            if closer_than_gap(*rect, *their, gap) {
                out.push(Conflict::Pair(*piece, *other));
            }
        }
    }
    out
}

/// Whether two rectangles are closer than `gap`, across or down. Exactly
/// `gap` is clear; one less is a clash.
pub fn closer_than_gap(a: Rect, b: Rect, gap: i32) -> bool {
    !Rect::new(a.x - gap, a.y - gap, a.w + 2 * gap, a.h + 2 * gap)
        .intersect(&b)
        .is_empty()
}

fn bad(window: (u32, u32), scale: u32, st: Placing, group: &[Piece]) -> bool {
    conflicts(&layout(window, scale, st), window, scale)
        .iter()
        .any(|conflict| conflict.within(group))
}

fn near_minimap(window: (u32, u32), scale: u32, st: Placing, piece: Piece) -> bool {
    conflicts(&layout(window, scale, st), window, scale)
        .iter()
        .any(|conflict| conflict.is_pair(piece, Piece::Minimap))
}

/// The part of the window the rectangles cover, each pixel counted once.
pub fn coverage(pieces: &BTreeMap<Piece, Rect>, window: (u32, u32)) -> f64 {
    let rects: Vec<Rect> = pieces.values().copied().collect();
    let mut xs: Vec<i32> = rects.iter().flat_map(|r| [r.x, r.right()]).collect();
    let mut ys: Vec<i32> = rects.iter().flat_map(|r| [r.y, r.bottom()]).collect();
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();
    let mut area = 0i64;
    for x in xs.windows(2) {
        for y in ys.windows(2) {
            let (cx, cy) = ((x[0] + x[1]) as f64 / 2.0, (y[0] + y[1]) as f64 / 2.0);
            if rects.iter().any(|r| {
                cx >= r.x as f64
                    && cx < r.right() as f64
                    && cy >= r.y as f64
                    && cy < r.bottom() as f64
            }) {
                area += (x[1] - x[0]) as i64 * (y[1] - y[0]) as i64;
            }
        }
    }
    area as f64 / (window.0 as f64 * window.1 as f64)
}
