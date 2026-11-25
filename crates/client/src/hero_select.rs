use crate::champion::ChampionConfig;
use crate::menu::MatchMode;

/// Filter tabs: the hero types.
pub const FILTERS: [&str; 4] = ["ALL", "MELEE", "RANGED", "MAGIC"];

/// Seconds to pick before the highlighted hero is locked in for you.
pub const PICK_SECONDS: f32 = 30.0;

/// Roster section order: heroes are grouped by type.
pub const CLASS_ORDER: [&str; 3] = ["MELEE", "RANGED", "MAGIC"];

fn class_rank(class: &str) -> usize {
    CLASS_ORDER
        .iter()
        .position(|c| c.eq_ignore_ascii_case(class))
        .unwrap_or(CLASS_ORDER.len())
}

/// A lane a player calls for. It is a note to the team, not a rule: two
/// players may call the same lane, anyone may change their call until the
/// match starts, and nothing in the match holds a player to it. Bots call
/// the lane they will really play, so the columns read true.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lane {
    Top,
    Jungle,
    Mid,
    Carry,
    Support,
}

impl Lane {
    /// The picker's order, top lane to support.
    pub const ALL: [Lane; 5] = [
        Lane::Top,
        Lane::Jungle,
        Lane::Mid,
        Lane::Carry,
        Lane::Support,
    ];

    /// Read a lane name, any case (`--lane mid`).
    pub fn parse(name: &str) -> Option<Lane> {
        Lane::ALL
            .into_iter()
            .find(|lane| lane.label().eq_ignore_ascii_case(name.trim()))
    }

    pub fn label(self) -> &'static str {
        match self {
            Lane::Top => "Top",
            Lane::Jungle => "Jungle",
            Lane::Mid => "Mid",
            Lane::Carry => "Carry",
            Lane::Support => "Support",
        }
    }

    /// The cache skill icon drawn with the lane (a UI atlas key): Defence,
    /// Hunter, Runecraft, Fletching and Herblore, apart from the Attack,
    /// Ranged and Magic icons the class filters use.
    pub fn icon(self) -> &'static str {
        match self {
            Lane::Top => "stat_defence",
            Lane::Jungle => "skill_hunter",
            Lane::Mid => "skill_runecraft",
            Lane::Carry => "skill_fletching",
            Lane::Support => "skill_herblore",
        }
    }

    /// The bot role that plays the lane: carry and support share bottom.
    pub fn bot_role(self) -> crate::animated::bots::BotRole {
        use crate::animated::bots::BotRole;
        match self {
            Lane::Top => BotRole::Lane(0),
            Lane::Mid => BotRole::Lane(1),
            Lane::Carry | Lane::Support => BotRole::Lane(2),
            Lane::Jungle => BotRole::Jungle,
        }
    }
}

/// The red bots' calls, row by row: every lane once, in draft order (the
/// roles they played before the picker).
pub const RED_LANES: [Lane; 5] = [
    Lane::Top,
    Lane::Mid,
    Lane::Carry,
    Lane::Support,
    Lane::Jungle,
];

/// What blue's four bots call (and play) around the player's call: the
/// lanes nobody has called, in draft order. With no call they leave mid
/// open, as they did before the picker.
pub fn bot_lanes(player: Option<Lane>) -> [Lane; 4] {
    let taken = player.unwrap_or(Lane::Mid);
    let mut free = RED_LANES.into_iter().filter(|&lane| lane != taken);
    std::array::from_fn(|_| free.next().unwrap_or(Lane::Mid))
}

/// One row of a team column: who sits there and what they have picked.
#[derive(Debug, Clone, PartialEq)]
pub struct PickSlot {
    pub name: String,
    /// The locked hero (a champion index); `None` while still choosing.
    /// The player's own row stays `None`: it shows the highlighted hero.
    pub champion: Option<usize>,
    /// Seconds into the countdown when this bot locks in.
    pub at: f32,
    /// The lane this player has called, if any (see [`Lane`]).
    pub lane: Option<Lane>,
}

/// When each bot locks in, as (team, row, seconds into the countdown):
/// red and blue alternate, two to three seconds apart, all done by 22 s.
pub const PICK_SCHEDULE: [(Team, usize, f32); 9] = [
    (Team::Red, 0, 2.5),
    (Team::Blue, 1, 4.5),
    (Team::Red, 1, 7.0),
    (Team::Blue, 2, 9.5),
    (Team::Red, 2, 12.0),
    (Team::Blue, 3, 14.5),
    (Team::Red, 3, 17.0),
    (Team::Blue, 4, 19.5),
    (Team::Red, 4, 22.0),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Team {
    Blue,
    Red,
}

/// State for the pre-match champion picker.
pub struct HeroSelect {
    pub open: bool,
    pub selected: usize,
    pub hover: Option<usize>,
    pub champions: Vec<ChampionConfig>,
    /// Active filter tab.
    pub filter: usize,
    /// Champion indices shown by the current filter, grouped by type.
    pub visible: Vec<usize>,
    /// Seconds spent on the screen, used for the selected-card pulse.
    pub time: f32,
    /// Seconds left to lock in; at zero the highlighted hero is locked.
    pub countdown: f32,
    /// The match mode picked on the main menu.
    pub mode: MatchMode,
    /// Your team's five rows; row 0 is you.
    pub blue: Vec<PickSlot>,
    /// The enemy team's five rows.
    pub red: Vec<PickSlot>,
    /// Varies the bots' choices between matches.
    pub seed: u32,
}

impl HeroSelect {
    pub fn new(champions: Vec<ChampionConfig>, selected: usize, open: bool) -> Self {
        let selected = selected.min(champions.len().saturating_sub(1));
        let mut select = Self {
            open,
            selected,
            hover: None,
            champions,
            filter: 0,
            visible: Vec::new(),
            time: 0.0,
            countdown: PICK_SECONDS,
            mode: MatchMode::Bot,
            blue: Vec::new(),
            red: Vec::new(),
            seed: 0,
        };
        select.reset_picks(0);
        select.apply_filter();
        select
    }

    /// Empty both team columns for a new pick phase.
    pub fn reset_picks(&mut self, seed: u32) {
        self.seed = seed;
        // Your call is a standing preference: it carries into the next
        // match's pick phase.
        let mine = self.blue.first().and_then(|slot| slot.lane);
        let row = |name: String, team: Team, index: usize| PickSlot {
            name,
            champion: None,
            at: PICK_SCHEDULE
                .iter()
                .find(|(t, i, _)| *t == team && *i == index)
                .map(|(_, _, at)| *at)
                .unwrap_or(f32::INFINITY),
            lane: None,
        };
        self.blue = (0..5)
            .map(|i| {
                let name = if i == 0 {
                    "You".to_string()
                } else {
                    format!("Bot {i}")
                };
                row(name, Team::Blue, i)
            })
            .collect();
        self.red = (0..5)
            .map(|i| row(format!("Bot {}", i + 1), Team::Red, i))
            .collect();
        for (slot, lane) in self.red.iter_mut().zip(RED_LANES) {
            slot.lane = Some(lane);
        }
        self.set_lane(mine);
    }

    /// Call a lane for yourself (or clear the call). Blue's bots move their
    /// own calls to the lanes left open; nothing stops two humans calling
    /// the same lane.
    pub fn set_lane(&mut self, lane: Option<Lane>) {
        if let Some(you) = self.blue.first_mut() {
            you.lane = lane;
        }
        for (slot, call) in self.blue.iter_mut().skip(1).zip(bot_lanes(lane)) {
            slot.lane = Some(call);
        }
    }

    /// Click a lane button: call it, or clear the call if it is yours.
    pub fn toggle_lane(&mut self, lane: Lane) {
        let mine = self.blue.first().and_then(|slot| slot.lane);
        self.set_lane(if mine == Some(lane) { None } else { Some(lane) });
    }

    /// Every row's call, in lineup order (you first).
    pub fn lanes(&self) -> (Vec<Option<Lane>>, Vec<Option<Lane>>) {
        let calls = |slots: &[PickSlot]| slots.iter().map(|slot| slot.lane).collect();
        (calls(&self.blue), calls(&self.red))
    }

    /// True when a bot on either team has locked this hero in.
    pub fn is_picked(&self, index: usize) -> bool {
        self.blue
            .iter()
            .chain(&self.red)
            .any(|slot| slot.champion == Some(index))
    }

    /// Lock in every bot whose turn has come. Bots never take the hero
    /// you have highlighted, and prefer a type their team lacks.
    pub fn advance_picks(&mut self) {
        let elapsed = PICK_SECONDS - self.countdown;
        for (team, row, at) in PICK_SCHEDULE {
            if at <= elapsed {
                self.bot_pick(team, row);
            }
        }
    }

    /// Lock in every bot still choosing (you locked in early).
    pub fn fill_picks(&mut self) {
        for (team, row, _) in PICK_SCHEDULE {
            self.bot_pick(team, row);
        }
    }

    fn bot_pick(&mut self, team: Team, row: usize) {
        let slots = match team {
            Team::Blue => &self.blue,
            Team::Red => &self.red,
        };
        if slots.get(row).is_none_or(|slot| slot.champion.is_some()) {
            return;
        }
        let have: Vec<&str> = slots
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| {
                if team == Team::Blue && i == 0 {
                    Some(self.selected)
                } else {
                    slot.champion
                }
            })
            .map(|index| self.champions[index].class.as_str())
            .collect();
        let free: Vec<usize> = (0..self.champions.len())
            .filter(|&i| i != self.selected && !self.is_picked(i))
            .collect();
        let wanted: Vec<usize> = free
            .iter()
            .copied()
            .filter(|&i| {
                !have
                    .iter()
                    .any(|c| c.eq_ignore_ascii_case(&self.champions[i].class))
            })
            .collect();
        let pool = if wanted.is_empty() { &free } else { &wanted };
        if pool.is_empty() {
            return;
        }
        let salt = (team as u32) * 5 + row as u32;
        let roll = crate::menu::hash01(self.seed, salt);
        let choice = pool[((roll * pool.len() as f32) as usize).min(pool.len() - 1)];
        let slots = match team {
            Team::Blue => &mut self.blue,
            Team::Red => &mut self.red,
        };
        slots[row].champion = Some(choice);
        // A highlighted hero is never taken, but a filter fallback can land
        // on one: move off anything picked.
        if self.is_picked(self.selected) {
            self.cycle(1);
        }
    }

    /// The final lineups (champion indices), you first.
    pub fn lineups(&self) -> (Vec<usize>, Vec<usize>) {
        let blue = self
            .blue
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| {
                if i == 0 {
                    Some(self.selected)
                } else {
                    slot.champion
                }
            })
            .collect();
        let red = self.red.iter().filter_map(|slot| slot.champion).collect();
        (blue, red)
    }

    /// Open the picker for a fresh match: full timer, all heroes shown.
    pub fn begin(&mut self, mode: MatchMode) {
        self.open = true;
        self.mode = mode;
        self.countdown = PICK_SECONDS;
        self.time = 0.0;
        self.hover = None;
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() ^ d.as_secs() as u32)
            .unwrap_or(0);
        self.reset_picks(seed);
        self.set_filter(0);
    }

    /// Count the pick timer down. True on the frame it runs out.
    pub fn tick_countdown(&mut self, dt: f32) -> bool {
        if !self.open || self.countdown <= 0.0 {
            return false;
        }
        self.countdown = (self.countdown - dt).max(0.0);
        self.advance_picks();
        self.countdown <= 0.0
    }

    /// Whole seconds shown on the timer (rounded up, so 0.2 s shows 1).
    pub fn seconds_left(&self) -> u32 {
        self.countdown.ceil().max(0.0) as u32
    }

    /// Recompute the visible roster for the active filter, grouped by type
    /// (melee, ranged, magic) and in file order inside each type.
    pub fn apply_filter(&mut self) {
        let mut visible: Vec<usize> = self
            .champions
            .iter()
            .enumerate()
            .filter(|(_, champion)| self.matches(champion))
            .map(|(index, _)| index)
            .collect();
        visible.sort_by_key(|&index| (class_rank(&self.champions[index].class), index));
        self.visible = visible;
        if !self.visible.contains(&self.selected) || self.is_picked(self.selected) {
            if let Some(&first) = self.visible.iter().find(|&&i| !self.is_picked(i)) {
                self.selected = first;
            }
        }
    }

    fn matches(&self, champion: &ChampionConfig) -> bool {
        match self.filter {
            1 => champion.class.eq_ignore_ascii_case("MELEE"),
            2 => champion.class.eq_ignore_ascii_case("RANGED"),
            3 => champion.class.eq_ignore_ascii_case("MAGIC"),
            _ => true,
        }
    }

    pub fn set_filter(&mut self, filter: usize) {
        self.filter = filter.min(FILTERS.len() - 1);
        self.apply_filter();
    }

    pub fn cycle(&mut self, delta: isize) {
        if self.visible.is_empty() {
            return;
        }
        let count = self.visible.len() as isize;
        let mut current = self
            .visible
            .iter()
            .position(|&index| index == self.selected)
            .unwrap_or(0) as isize;
        // Step over heroes the bots have taken.
        for _ in 0..count {
            current = (current + delta.signum().max(-1)).rem_euclid(count);
            let index = self.visible[current as usize];
            if !self.is_picked(index) {
                self.selected = index;
                return;
            }
        }
    }

    /// Select by the slot shown in the roster grid (taken heroes cannot be
    /// chosen).
    pub fn select_slot(&mut self, slot: usize) {
        if let Some(&index) = self.visible.get(slot)
            && !self.is_picked(index)
        {
            self.selected = index;
        }
    }

    pub fn selected_champion(&self) -> Option<&ChampionConfig> {
        self.champions.get(self.selected)
    }

    /// The type of each visible slot, for the grouped roster layout.
    pub fn visible_classes(&self) -> Vec<&str> {
        self.visible
            .iter()
            .map(|&index| self.champions[index].class.as_str())
            .collect()
    }
}

/// Outer margin of the select screen.
const MARGIN: f32 = 12.0;
/// Gap between the screen's stone windows.
const GAP: f32 = 8.0;
/// Height of the title bar window.
const TITLE_H: f32 = 72.0;
/// Height of the filter tab bar over the roster.
const TABS_H: f32 = 58.0;
/// Height of the bottom bar with Lock in.
const BOTTOM_H: f32 = 92.0;
/// Inner padding of a stone window (the border is 7px).
const PAD: f32 = 14.0;
/// Gap between roster cards.
const CARD_GAP: f32 = 6.0;
/// Widest a roster card grows.
const CARD_MAX: f32 = 96.0;
/// The roster window takes this share of the centre column's free height;
/// the parchment detail takes the rest.
const ROSTER_SHARE: f32 = 0.38;

type Rect = (f32, f32, f32, f32);

/// Top of the body windows, under the title bar.
fn body_top() -> f32 {
    MARGIN + TITLE_H + GAP
}

/// Width of each team column.
fn side_width(width: f32) -> f32 {
    (width * 0.19).clamp(200.0, 360.0)
}

/// The title bar across the top: mode, title and timer, Leave.
pub fn title_rect(width: f32, _height: f32) -> Rect {
    (MARGIN, MARGIN, width - MARGIN * 2.0, TITLE_H)
}

/// The Leave button at the title bar's right end.
pub fn leave_rect(width: f32, height: f32) -> Rect {
    let (x, y, w, h) = title_rect(width, height);
    let bw = 108.0;
    let bh = 34.0;
    (x + w - PAD - bw, y + (h - bh) * 0.5, bw, bh)
}

pub fn leave_at(width: f32, height: f32, cursor: (f64, f64)) -> bool {
    point_in_rect(leave_rect(width, height), cursor)
}

/// A team's column window: blue down the left, red down the right.
pub fn team_column(width: f32, height: f32, team: Team) -> Rect {
    let w = side_width(width);
    let top = body_top();
    let x = match team {
        Team::Blue => MARGIN,
        Team::Red => width - MARGIN - w,
    };
    (x, top, w, height - top - MARGIN)
}

/// The centre column between the team windows.
fn centre(width: f32, height: f32) -> Rect {
    let side = side_width(width);
    let x = MARGIN + side + GAP;
    let top = body_top();
    (
        (x),
        top,
        (width - 2.0 * x).max(200.0),
        height - top - MARGIN,
    )
}

/// The filter tab bar over the roster.
pub fn tabs_rect(width: f32, height: f32) -> Rect {
    let (x, y, w, _) = centre(width, height);
    (x, y, w, TABS_H)
}

/// The roster window (kept as `roster_column` for its callers).
pub fn roster_column(width: f32, height: f32) -> Rect {
    let (x, y, w, h) = centre(width, height);
    let free = h - TABS_H - BOTTOM_H - GAP * 3.0;
    (x, y + TABS_H + GAP, w, (free * ROSTER_SHARE).max(120.0))
}

/// The parchment with the highlighted hero's art, stats and abilities.
pub fn detail_rect(width: f32, height: f32) -> Rect {
    let (x, y, w, h) = roster_column(width, height);
    let (_, by, _, _) = bottom_rect(width, height);
    let top = y + h + GAP;
    (x, top, w, (by - GAP - top).max(120.0))
}

/// The bottom bar: hero stats and the Lock in button.
pub fn bottom_rect(width: f32, height: f32) -> Rect {
    let (x, y, w, h) = centre(width, height);
    (x, y + h - BOTTOM_H, w, BOTTOM_H)
}

/// The animated hero's recess on the parchment's left (3:4, like the
/// spotlight render), leaving room under it for the name and tags.
pub fn spotlight_rect(width: f32, height: f32) -> Rect {
    let (x, y, _, h) = detail_rect(width, height);
    let ph = (h - 96.0).max(80.0);
    let pw = ph * 0.75;
    (x + 22.0, y + 18.0, pw, ph)
}

/// Width of the "Your lane:" label at the bar's left.
const LANE_LABEL: f32 = 96.0;
/// Width of the "Show:" label before the class filters.
const SHOW_LABEL: f32 = 56.0;
/// The class filters are compact icon tabs at the bar's right end, as
/// wide as this at most and as narrow as the smallest tab an icon fits.
const FILTER_W: f32 = 52.0;
const TAB_MIN: f32 = 36.0;
/// Gap between tabs, and the wider gap between the two groups.
const TAB_GAP: f32 = 8.0;
const GROUP_GAP: f32 = 20.0;

/// The bar's shared tab height and top.
fn tab_row(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let (x, y, w, h) = tabs_rect(width, height);
    (x, y + 10.0, w, h - 20.0)
}

/// Class filter tabs, right-aligned in the bar after the "Show:" label. On
/// a narrow bar they shrink first, so the lane tabs keep room for icons.
pub fn filter_rects(width: f32, height: f32) -> Vec<Rect> {
    let (x0, y, w, th) = tab_row(width, height);
    let n = FILTERS.len() as f32;
    let lanes = Lane::ALL.len() as f32;
    let lanes_min = TAB_MIN * lanes + TAB_GAP * (lanes - 1.0);
    let spare = w - PAD * 2.0 - LANE_LABEL - GROUP_GAP - SHOW_LABEL - lanes_min;
    let tab = ((spare - TAB_GAP * (n - 1.0)) / n).clamp(TAB_MIN, FILTER_W);
    let start = x0 + w - PAD - (tab * n + TAB_GAP * (n - 1.0));
    (0..FILTERS.len())
        .map(|index| (start + index as f32 * (tab + TAB_GAP), y, tab, th))
        .collect()
}

/// Where the "Show:" label sits: just before the filter tabs.
pub fn show_label_x(width: f32, height: f32) -> f32 {
    filter_rects(width, height)[0].0 - SHOW_LABEL + 6.0
}

/// The lane tabs after the "Your lane:" label, in [`Lane::ALL`] order,
/// sharing the bar with the filters and taking whatever width they leave.
pub fn lane_rects(width: f32, height: f32) -> Vec<Rect> {
    let (x0, y, _, th) = tab_row(width, height);
    let start = x0 + PAD + LANE_LABEL;
    let end = show_label_x(width, height) - GROUP_GAP;
    let n = Lane::ALL.len() as f32;
    let tab = ((end - start - TAB_GAP * (n - 1.0)) / n).clamp(TAB_MIN, 170.0);
    (0..Lane::ALL.len())
        .map(|index| (start + index as f32 * (tab + TAB_GAP), y, tab, th))
        .collect()
}

/// Which lane tab the cursor is over.
pub fn lane_at(width: f32, height: f32, cursor: (f64, f64)) -> Option<Lane> {
    lane_rects(width, height)
        .iter()
        .position(|&rect| point_in_rect(rect, cursor))
        .map(|index| Lane::ALL[index])
}

pub fn filter_at(width: f32, height: f32, cursor: (f64, f64)) -> Option<usize> {
    filter_rects(width, height)
        .iter()
        .position(|&rect| point_in_rect(rect, cursor))
}

/// The roster: a card rectangle per visible slot, in one grid (the class
/// order already groups them). The header list is kept for callers and is
/// empty: the tabs filter by class instead.
#[allow(clippy::type_complexity)]
pub fn roster_layout(width: f32, height: f32, classes: &[&str]) -> (Vec<Rect>, Vec<(String, f32)>) {
    let (rx, ry, rw, rh) = roster_column(width, height);
    let inner_w = rw - PAD * 2.0;
    let inner_h = rh - PAD * 2.0;
    let count = classes.len().max(1);
    // The column count that gives the largest cards that still fit every
    // row in the window; the name takes 18px under each portrait.
    let mut best = (4usize, 0.0f32);
    for columns in 4..=16usize {
        let rows = count.div_ceil(columns);
        let by_w = (inner_w - CARD_GAP * (columns as f32 - 1.0)) / columns as f32;
        let by_h = (inner_h - CARD_GAP * (rows as f32 - 1.0)) / rows as f32 - 18.0;
        let size = by_w.min(by_h).min(CARD_MAX);
        if size > best.1 + 0.5 {
            best = (columns, size);
        }
    }
    let (columns, card_w) = (best.0, best.1.max(36.0));
    let card_h = card_w + 18.0;
    let used = columns as f32 * card_w + CARD_GAP * (columns as f32 - 1.0);
    let x0 = rx + PAD + ((inner_w - used) * 0.5).max(0.0);
    let cards = (0..classes.len())
        .map(|k| {
            let column = k % columns;
            let row = k / columns;
            (
                x0 + column as f32 * (card_w + CARD_GAP),
                ry + PAD + row as f32 * (card_h + CARD_GAP),
                card_w,
                card_h,
            )
        })
        .collect();
    (cards, Vec::new())
}

/// Which visible slot the cursor is over.
pub fn card_at(width: f32, height: f32, cursor: (f64, f64), classes: &[&str]) -> Option<usize> {
    roster_layout(width, height, classes)
        .0
        .iter()
        .position(|&rect| point_in_rect(rect, cursor))
}

/// The timer under the title, centred in the title bar.
pub fn timer_rect(width: f32, height: f32) -> Rect {
    let (x, y, w, h) = title_rect(width, height);
    (x + w * 0.5 - 40.0, y + h * 0.5, 80.0, h * 0.5 - 6.0)
}

/// The red "Lock in" button at the bottom bar's right end.
pub fn confirm_rect(width: f32, height: f32) -> Rect {
    let (x, y, w, h) = bottom_rect(width, height);
    let bw = 240.0f32.min(w * 0.3);
    (x + w - PAD - bw, y + PAD, bw, h - PAD * 2.0)
}

pub fn confirm_at(width: f32, height: f32, cursor: (f64, f64)) -> bool {
    point_in_rect(confirm_rect(width, height), cursor)
}

fn point_in_rect(rect: Rect, cursor: (f64, f64)) -> bool {
    let (x, y, w, h) = rect;
    let mx = cursor.0 as f32;
    let my = cursor.1 as f32;
    mx >= x && mx <= x + w && my >= y && my <= y + h
}

/// The five player rows of a team column, under its header.
pub fn team_slot_rects(width: f32, height: f32, team: Team) -> Vec<Rect> {
    let (x, y, w, h) = team_column(width, height, team);
    let header = 44.0;
    let gap = 6.0;
    let top = y + header;
    let row_h = ((h - header - PAD - gap * 4.0) / 5.0).max(40.0);
    (0..5)
        .map(|i| (x + 10.0, top + i as f32 * (row_h + gap), w - 20.0, row_h))
        .collect()
}

/// The two five-strong lineups shown on the lobby screen.
pub fn lobby_teams(champions: &[ChampionConfig], selected: usize) -> (Vec<usize>, Vec<usize>) {
    let count = champions.len();
    if count == 0 {
        return (Vec::new(), Vec::new());
    }
    let selected = selected.min(count - 1);
    let mut radiant = vec![selected];
    for step in 1..5 {
        radiant.push((selected + step) % count);
    }
    let mut dire = Vec::new();
    for step in 0..5 {
        dire.push((selected + step + 1) % count);
    }
    (radiant, dire)
}

/// Lobby card grid: two rows of five.
pub fn lobby_card_rects(width: f32, height: f32) -> Vec<(f32, f32, f32, f32)> {
    let gap = 12.0;
    let side = (width * 0.072).clamp(50.0, 92.0);
    let area_x = side + 12.0;
    let area_w = (width - side * 2.0 - 24.0).max(240.0);
    let card_w = (area_w - gap * 4.0) / 5.0;
    let card_h = (height * 0.245).clamp(132.0, 186.0);
    let top = (height * 0.235).max(126.0);
    (0..10)
        .map(|index| {
            let row = index / 5;
            let column = index % 5;
            (
                area_x + column as f32 * (card_w + gap),
                top + row as f32 * (card_h + gap + 26.0),
                card_w,
                card_h,
            )
        })
        .collect()
}

/// Radiant and dire side tabs.
#[allow(clippy::type_complexity)]
pub fn lobby_side_tabs(width: f32, height: f32) -> ((f32, f32, f32, f32), (f32, f32, f32, f32)) {
    let w = (width * 0.066).clamp(46.0, 84.0);
    let h = (height * 0.42).clamp(210.0, 310.0);
    let top = (height * 0.235).max(126.0);
    ((6.0, top, w, h), (width - 6.0 - w, top, w, h))
}

/// The red "loading" button under the lobby cards.
pub fn lobby_button_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let w = 360.0f32.min(width * 0.32);
    let h = 44.0;
    ((width - w) * 0.5, height - h - 52.0, w, h)
}

pub fn lobby_button_at(width: f32, height: f32, cursor: (f64, f64)) -> bool {
    point_in_rect(lobby_button_rect(width, height), cursor)
}

/// Wrap a description into fixed-width lines for the detail panel.
pub fn wrap_text(text: &str, columns: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + word.chars().count() + 1 > columns {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::champion::ChampionConfig;

    /// A stub hero with neutral numbers (also used by `main`'s tests).
    pub(crate) fn champion(id: &str) -> ChampionConfig {
        ChampionConfig {
            id: id.to_string(),
            name: id.to_string(),
            role: String::new(),
            class: String::new(),
            abilities: Vec::new(),
            ability_icons: Vec::new(),
            kit: None,
            sounds: Default::default(),
            tagline: String::new(),
            model: 0,
            attachments: Vec::new(),
            idle: 0,
            walk: 0,
            scale: 1.0,
            model_yaw: 0.0,
            walk_sway: 1.0,
            walk_arm: 1.0,
            attack_windup: 0.0,
            attack_anim: None,
            attack_hit: 0.5,
            cast_anim: None,
            projectile: None,
            health: 1.0,
            mana: 1.0,
            damage: 1.0,
            armor: 0.0,
            attack_range: 1.0,
            attack_interval: 1.0,
            speed: 1.0,
            ranged_auto: false,
            ranged: 0.0,
            q_damage: 1.0,
            r_damage: 1.0,
            e_heal: 1.0,
            e_shield: 1.0,
            w_distance: 1.0,
            weight: None,
            turn_time: None,
            accel_time: None,
            stop_time: None,
            stride: 0.0,
            bounce: 1.0,
            weapon: None,
        }
    }

    #[test]
    fn filters_pick_the_right_class() {
        let mut select =
            HeroSelect::new(vec![champion("a"), champion("b"), champion("c")], 0, true);
        select.champions[0].class = "MELEE".to_string();
        select.champions[1].class = "RANGED".to_string();
        select.champions[2].class = "MAGIC".to_string();
        select.set_filter(2);
        assert_eq!(select.visible, vec![1]);
        select.set_filter(0);
        assert_eq!(select.visible.len(), 3);
    }

    #[test]
    fn card_slots_map_to_champions() {
        let mut select = HeroSelect::new(vec![champion("a"), champion("b")], 0, true);
        select.select_slot(1);
        assert_eq!(select.selected, 1);
        select.cycle(1);
        assert_eq!(select.selected, 0);
    }

    #[test]
    fn the_roster_groups_heroes_by_type() {
        let mut heroes = vec![champion("a"), champion("b"), champion("c"), champion("d")];
        heroes[0].class = "MAGIC".to_string();
        heroes[1].class = "MELEE".to_string();
        heroes[2].class = "RANGED".to_string();
        heroes[3].class = "MELEE".to_string();
        let select = HeroSelect::new(heroes, 0, true);
        assert_eq!(select.visible, vec![1, 3, 2, 0]);
        let (cards, headers) = roster_layout(1280.0, 720.0, &select.visible_classes());
        assert_eq!(cards.len(), 4);
        // One grid in reading order; the class tabs replace the headers.
        assert!(headers.is_empty());
        for pair in cards.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            assert!(b.1 > a.1 || (b.1 == a.1 && b.0 > a.0));
        }
    }

    #[test]
    fn sixteen_heroes_fit_the_roster_column_at_720p() {
        let classes: Vec<&str> = std::iter::repeat_n("MELEE", 9)
            .chain(std::iter::repeat_n("RANGED", 3))
            .chain(std::iter::repeat_n("MAGIC", 4))
            .collect();
        for (w, h) in [(1280.0, 720.0), (1920.0, 1080.0)] {
            let (cx, cy, cw, ch) = roster_column(w, h);
            let (cards, _) = roster_layout(w, h, &classes);
            for &(x, y, cwid, chgt) in &cards {
                assert!(x >= cx && x + cwid <= cx + cw + 0.5, "{w}x{h}");
                assert!(y >= cy && y + chgt <= cy + ch + 0.5, "{w}x{h} y {y}");
            }
        }
    }

    #[test]
    fn the_countdown_locks_in_once() {
        let mut select = HeroSelect::new(vec![champion("a")], 0, false);
        select.begin(MatchMode::Public);
        assert_eq!(select.seconds_left(), PICK_SECONDS as u32);
        assert!(!select.tick_countdown(PICK_SECONDS - 0.5));
        assert_eq!(select.seconds_left(), 1);
        assert!(select.tick_countdown(1.0));
        assert!(!select.tick_countdown(1.0), "fires only once");
        assert_eq!(select.mode, MatchMode::Public);
    }

    fn roster() -> Vec<ChampionConfig> {
        let classes = ["MELEE", "RANGED", "MAGIC"];
        (0..16)
            .map(|i| {
                let mut c = champion(&format!("h{i}"));
                c.class = classes[i % 3].to_string();
                c
            })
            .collect()
    }

    #[test]
    fn bots_pick_on_schedule_without_taking_your_hero() {
        let mut select = HeroSelect::new(roster(), 4, true);
        select.reset_picks(1234);
        select.countdown = PICK_SECONDS;
        select.advance_picks();
        assert!(
            select.red.iter().all(|s| s.champion.is_none()),
            "nobody at 0 s"
        );
        select.countdown = PICK_SECONDS - 5.0;
        select.advance_picks();
        assert!(select.red[0].champion.is_some(), "red opens at 2.5 s");
        assert!(select.blue[1].champion.is_some(), "blue answers at 4.5 s");
        assert!(select.red[1].champion.is_none(), "not yet 7 s");
        select.countdown = 1.0;
        select.advance_picks();
        let (blue, red) = select.lineups();
        assert_eq!((blue.len(), red.len()), (5, 5));
        assert_eq!(blue[0], 4, "you are first, with your highlight");
        let mut all: Vec<usize> = blue.iter().chain(&red).copied().collect();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), 10, "ten different heroes");
        assert!(!select.is_picked(4), "your highlight is never taken");
    }

    #[test]
    fn bots_fill_missing_types_first() {
        let mut select = HeroSelect::new(roster(), 0, true);
        select.reset_picks(99);
        select.fill_picks();
        for team in [&select.blue, &select.red] {
            let mut classes: Vec<&str> = team
                .iter()
                .enumerate()
                .filter_map(|(i, slot)| {
                    if std::ptr::eq(team, &select.blue) && i == 0 {
                        Some(select.selected)
                    } else {
                        slot.champion
                    }
                })
                .map(|i| select.champions[i].class.as_str())
                .collect();
            classes.sort();
            classes.dedup();
            assert_eq!(classes.len(), 3, "each team has melee, ranged and magic");
        }
    }

    #[test]
    fn picked_heroes_cannot_be_chosen() {
        let mut select = HeroSelect::new(roster(), 0, true);
        select.reset_picks(7);
        select.red[0].champion = Some(1);
        let slot = select.visible.iter().position(|&i| i == 1).unwrap();
        select.select_slot(slot);
        assert_eq!(select.selected, 0, "click on a picked card is ignored");
        for _ in 0..40 {
            select.cycle(1);
            assert_ne!(select.selected, 1, "cycling steps over it");
        }
        select.reset_picks(7);
        assert!(!select.is_picked(1), "a new pick phase clears the columns");
    }

    #[test]
    fn blue_bots_call_the_lanes_you_leave_open() {
        use std::collections::HashSet;
        // With no call they leave mid for you, as before the picker.
        assert_eq!(
            bot_lanes(None),
            [Lane::Top, Lane::Carry, Lane::Support, Lane::Jungle]
        );
        for lane in Lane::ALL {
            let bots = bot_lanes(Some(lane));
            assert!(!bots.contains(&lane), "{lane:?} left to you");
            let all: HashSet<_> = bots.iter().copied().chain([lane]).collect();
            assert_eq!(all.len(), 5, "{lane:?}: one of each lane");
        }
    }

    #[test]
    fn default_calls_play_the_old_draft_roles() {
        use crate::animated::bots::roles_for;
        let blue: Vec<_> = bot_lanes(None).map(Lane::bot_role).to_vec();
        assert_eq!(blue, roles_for(0, 4));
        let red: Vec<_> = RED_LANES.map(Lane::bot_role).to_vec();
        assert_eq!(red, roles_for(1, 5));
    }

    #[test]
    fn a_lane_call_is_a_note_not_a_rule() {
        let mut select = HeroSelect::new(roster(), 0, true);
        select.begin(MatchMode::Bot);
        assert_eq!(select.blue[0].lane, None, "nobody calls for you");
        select.toggle_lane(Lane::Jungle);
        assert_eq!(select.blue[0].lane, Some(Lane::Jungle));
        assert!(
            select.blue[1..]
                .iter()
                .all(|slot| slot.lane != Some(Lane::Jungle)),
            "bots move off your lane"
        );
        // Clicking your lane again takes the call back.
        select.toggle_lane(Lane::Jungle);
        assert_eq!(select.blue[0].lane, None);
        // A call changes nothing about picking or locking in.
        select.set_lane(Some(Lane::Carry));
        let before = select.selected;
        select.fill_picks();
        let (blue, red) = select.lineups();
        assert_eq!((blue.len(), red.len()), (5, 5));
        assert_eq!(blue[0], before);
        // It carries over to the next pick phase.
        select.begin(MatchMode::Bot);
        assert_eq!(select.blue[0].lane, Some(Lane::Carry));
        assert_eq!(select.lanes().1, RED_LANES.map(Some).to_vec());
    }

    #[test]
    fn lane_names_parse_in_any_case() {
        assert_eq!(Lane::parse("MID"), Some(Lane::Mid));
        assert_eq!(Lane::parse(" support "), Some(Lane::Support));
        assert_eq!(Lane::parse("bot"), None);
    }

    #[test]
    fn lane_and_class_tabs_share_the_bar_without_overlap() {
        for (w, h) in [(1280.0, 720.0), (1920.0, 1080.0), (1024.0, 600.0)] {
            let (bx, by, bw, bh) = tabs_rect(w, h);
            let lanes = lane_rects(w, h);
            let filters = filter_rects(w, h);
            for &(x, y, rw, rh) in lanes.iter().chain(&filters) {
                assert!(x >= bx && x + rw <= bx + bw, "{w}x{h} inside the bar");
                assert!(y >= by && y + rh <= by + bh, "{w}x{h} inside the bar");
            }
            let last = lanes[lanes.len() - 1];
            assert!(
                last.0 + last.2 < show_label_x(w, h),
                "{w}x{h} lanes end before Show:"
            );
            assert!(show_label_x(w, h) < filters[0].0, "{w}x{h}");
            for (index, &(x, y, rw, rh)) in lanes.iter().enumerate() {
                let centre = ((x + rw * 0.5) as f64, (y + rh * 0.5) as f64);
                assert_eq!(lane_at(w, h, centre), Some(Lane::ALL[index]));
                assert_eq!(filter_at(w, h, centre), None);
            }
        }
    }

    #[test]
    fn team_rows_fit_inside_their_columns() {
        for (w, h) in [(1280.0, 720.0), (1920.0, 1080.0), (1024.0, 600.0)] {
            for team in [Team::Blue, Team::Red] {
                let (cx, cy, cw, ch) = team_column(w, h, team);
                for (x, y, rw, rh) in team_slot_rects(w, h, team) {
                    assert!(x >= cx && x + rw <= cx + cw, "{w}x{h} x");
                    assert!(y >= cy && y + rh <= cy + ch + 0.01, "{w}x{h} y");
                }
            }
            let blue = team_column(w, h, Team::Blue);
            let red = team_column(w, h, Team::Red);
            let (rx, _, rw, _) = roster_column(w, h);
            assert!(
                blue.0 + blue.2 < rx && rx + rw < red.0,
                "{w}x{h} columns apart"
            );
        }
    }

    #[test]
    fn the_spotlight_sits_on_the_parchment_under_the_roster() {
        for (w, h) in [(1280.0, 720.0), (1920.0, 1080.0)] {
            let (dx, dy, dw, dh) = detail_rect(w, h);
            let (sx, sy, sw, sh) = spotlight_rect(w, h);
            assert!(sx > dx && sx + sw < dx + dw, "{w}x{h} x");
            assert!(sy > dy && sy + sh < dy + dh, "{w}x{h} y");
            let (_, ry, _, rh) = roster_column(w, h);
            assert!(dy > ry + rh, "{w}x{h} under the roster");
            let (bx, by, bw, bh) = confirm_rect(w, h);
            let (fx, fy, fw, fh) = bottom_rect(w, h);
            assert!(bx >= fx && bx + bw <= fx + fw && by >= fy && by + bh <= fy + fh);
            assert!(by + bh < h, "{w}x{h} lock-in on screen");
            let (lx, ly, lw, lh) = leave_rect(w, h);
            assert!(
                lx + lw < w && ly + lh < dy,
                "{w}x{h} leave in the title bar"
            );
        }
    }

    #[test]
    fn descriptions_wrap_to_the_panel_width() {
        let lines = wrap_text("one two three four five six seven eight", 12);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| line.chars().count() <= 12));
    }
}
