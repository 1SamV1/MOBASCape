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
        };
        select.apply_filter();
        select
    }

    /// Open the picker for a fresh match: full timer, all heroes shown.
    pub fn begin(&mut self, mode: MatchMode) {
        self.open = true;
        self.mode = mode;
        self.countdown = PICK_SECONDS;
        self.time = 0.0;
        self.hover = None;
        self.set_filter(0);
    }

    /// Count the pick timer down. True on the frame it runs out.
    pub fn tick_countdown(&mut self, dt: f32) -> bool {
        if !self.open || self.countdown <= 0.0 {
            return false;
        }
        self.countdown = (self.countdown - dt).max(0.0);
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
        if !self.visible.contains(&self.selected) {
            if let Some(&first) = self.visible.first() {
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
        let current = self
            .visible
            .iter()
            .position(|&index| index == self.selected)
            .unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(count) as usize;
        self.selected = self.visible[next];
    }

    /// Select by the slot shown in the roster grid.
    pub fn select_slot(&mut self, slot: usize) {
        if let Some(&index) = self.visible.get(slot) {
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
