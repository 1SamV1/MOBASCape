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

/// One row of a team column: who sits there and what they have picked.
#[derive(Debug, Clone, PartialEq)]
pub struct PickSlot {
    pub name: String,
    /// The locked hero (a champion index); `None` while still choosing.
    /// The player's own row stays `None`: it shows the highlighted hero.
    pub champion: Option<usize>,
    /// Seconds into the countdown when this bot locks in.
    pub at: f32,
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
        let row = |name: String, team: Team, index: usize| PickSlot {
            name,
            champion: None,
            at: PICK_SCHEDULE
                .iter()
                .find(|(t, i, _)| *t == team && *i == index)
                .map(|(_, _, at)| *at)
                .unwrap_or(f32::INFINITY),
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
const MARGIN: f32 = 14.0;
/// Height of a roster section header ("MELEE", ...).
const HEADER_H: f32 = 20.0;
/// Gap between roster cards.
const CARD_GAP: f32 = 6.0;
/// Roster columns.
const COLUMNS: usize = 4;

/// Top of the three columns (below the title and the timer).
fn body_top(height: f32) -> f32 {
    (height * 0.15).max(96.0)
}

/// Left column: type tabs and the grouped roster.
pub fn roster_column(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let w = (width * 0.29).clamp(300.0, 470.0);
    let top = body_top(height);
    (MARGIN, top, w, height - top - MARGIN)
}

/// Right column: the selected hero's stats and abilities.
pub fn detail_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let w = (width * 0.29).clamp(300.0, 470.0);
    let top = body_top(height);
    (width - MARGIN - w, top, w, height - top - MARGIN)
}

/// Centre stage where the spotlight hero stands.
pub fn spotlight_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let (lx, _, lw, _) = roster_column(width, height);
    let (rx, top, _, _) = detail_rect(width, height);
    let x = lx + lw + 16.0;
    let w = (rx - 16.0 - x).max(120.0);
    let h = (height - top - MARGIN - 128.0).max(160.0);
    (x, top, w, h)
}

/// Filter tab rectangles, across the top of the roster column.
pub fn filter_rects(width: f32, height: f32) -> Vec<(f32, f32, f32, f32)> {
    let (x0, y, w, _) = roster_column(width, height);
    let inner = w - 16.0;
    let gap = 6.0;
    let tab = (inner - gap * (FILTERS.len() as f32 - 1.0)) / FILTERS.len() as f32;
    (0..FILTERS.len())
        .map(|index| (x0 + 8.0 + index as f32 * (tab + gap), y + 8.0, tab, 28.0))
        .collect()
}

pub fn filter_at(width: f32, height: f32, cursor: (f64, f64)) -> Option<usize> {
    filter_rects(width, height)
        .iter()
        .position(|&rect| point_in_rect(rect, cursor))
}

/// The grouped roster: a card rectangle per visible slot, and a header
/// (type label, y) above each run of one type.
#[allow(clippy::type_complexity)]
pub fn roster_layout(
    width: f32,
    height: f32,
    classes: &[&str],
) -> (Vec<(f32, f32, f32, f32)>, Vec<(String, f32)>) {
    let (cx, cy, cw, ch) = roster_column(width, height);
    let x0 = cx + 8.0;
    let top = cy + 44.0;
    let inner_w = cw - 16.0;
    // Count rows and headers to size the cards so everything fits.
    let mut rows = 0usize;
    let mut headers = 0usize;
    let mut i = 0;
    while i < classes.len() {
        let run = classes[i..]
            .iter()
            .take_while(|c| c.eq_ignore_ascii_case(classes[i]))
            .count();
        headers += 1;
        rows += run.div_ceil(COLUMNS);
        i += run;
    }
    let by_width = (inner_w - CARD_GAP * (COLUMNS as f32 - 1.0)) / COLUMNS as f32;
    let avail_h = (cy + ch - 8.0 - top - headers as f32 * HEADER_H).max(60.0);
    let by_height = if rows == 0 {
        by_width
    } else {
        (avail_h - CARD_GAP * rows as f32) / rows as f32 / 1.22
    };
    let card_w = by_width.min(by_height).clamp(40.0, 118.0);
    let card_h = card_w * 1.22;

    let mut cards = Vec::with_capacity(classes.len());
    let mut labels = Vec::new();
    let mut y = top;
    let mut i = 0;
    while i < classes.len() {
        let class = classes[i];
        let run = classes[i..]
            .iter()
            .take_while(|c| c.eq_ignore_ascii_case(class))
            .count();
        labels.push((class.to_ascii_uppercase(), y));
        y += HEADER_H;
        for k in 0..run {
            let column = k % COLUMNS;
            let row = k / COLUMNS;
            cards.push((
                x0 + column as f32 * (card_w + CARD_GAP),
                y + row as f32 * (card_h + CARD_GAP),
                card_w,
                card_h,
            ));
        }
        y += run.div_ceil(COLUMNS) as f32 * (card_h + CARD_GAP);
        i += run;
    }
    (cards, labels)
}

/// Which visible slot the cursor is over.
pub fn card_at(width: f32, height: f32, cursor: (f64, f64), classes: &[&str]) -> Option<usize> {
    roster_layout(width, height, classes)
        .0
        .iter()
        .position(|&rect| point_in_rect(rect, cursor))
}

/// The countdown dial at the top centre.
pub fn timer_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let size = 54.0;
    let y = (body_top(height) - size - 6.0).max(height * 0.028 + 44.0);
    ((width - size) * 0.5, y, size, size)
}

/// Gold "lock in" button under the spotlight.
pub fn confirm_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let (sx, sy, sw, sh) = spotlight_rect(width, height);
    let w = 220.0f32.min(sw * 0.8);
    let h = 42.0;
    (sx + (sw - w) * 0.5, sy + sh + 74.0, w, h)
}

pub fn confirm_at(width: f32, height: f32, cursor: (f64, f64)) -> bool {
    point_in_rect(confirm_rect(width, height), cursor)
}

fn point_in_rect(rect: (f32, f32, f32, f32), cursor: (f64, f64)) -> bool {
    let (x, y, w, h) = rect;
    let mx = cursor.0 as f32;
    let my = cursor.1 as f32;
    mx >= x && mx <= x + w && my >= y && my <= y + h
}

/// Team column rows inside the spotlight: five down the left edge (your
/// team) and five down the right (enemies), sized to fit the stage height.
pub fn team_slot_rects(width: f32, height: f32, team: Team) -> Vec<(f32, f32, f32, f32)> {
    let (sx, sy, sw, sh) = spotlight_rect(width, height);
    let gap = 6.0;
    let top = sy + 22.0;
    let w = (sw * 0.15).clamp(44.0, 68.0);
    let h = (w + 14.0).min((sh - 22.0 - gap * 4.0) / 5.0);
    let x = match team {
        Team::Blue => sx + 4.0,
        Team::Red => sx + sw - 4.0 - w,
    };
    (0..5)
        .map(|i| (x, top + i as f32 * (h + gap), w, h))
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
mod tests {
    use super::*;
    use crate::champion::ChampionConfig;

    fn champion(id: &str) -> ChampionConfig {
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
        let labels: Vec<&str> = headers.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(labels, ["MELEE", "RANGED", "MAGIC"]);
        // Each header sits above its first card.
        assert!(headers[1].1 < cards[2].1 && headers[2].1 < cards[3].1);
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
    fn team_rows_fit_inside_the_stage() {
        for (w, h) in [(1280.0, 720.0), (1920.0, 1080.0), (1024.0, 600.0)] {
            let (sx, sy, sw, sh) = spotlight_rect(w, h);
            let blue = team_slot_rects(w, h, Team::Blue);
            let red = team_slot_rects(w, h, Team::Red);
            for &(x, y, rw, rh) in blue.iter().chain(&red) {
                assert!(x >= sx && x + rw <= sx + sw, "{w}x{h} x");
                assert!(y >= sy && y + rh <= sy + sh + 0.01, "{w}x{h} y");
            }
            assert!(blue[0].0 + blue[0].2 < red[0].0, "{w}x{h} columns apart");
        }
    }

    #[test]
    fn the_spotlight_sits_between_the_columns() {
        for (w, h) in [(1280.0, 720.0), (1920.0, 1080.0)] {
            let (lx, _, lw, _) = roster_column(w, h);
            let (rx, _, _, _) = detail_rect(w, h);
            let (sx, _, sw, _) = spotlight_rect(w, h);
            assert!(sx > lx + lw && sx + sw < rx);
            let (bx, by, _, bh) = confirm_rect(w, h);
            assert!(bx > sx && by + bh < h, "{w}x{h} lock-in on screen");
        }
    }

    #[test]
    fn descriptions_wrap_to_the_panel_width() {
        let lines = wrap_text("one two three four five six seven eight", 12);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| line.chars().count() <= 12));
    }
}
