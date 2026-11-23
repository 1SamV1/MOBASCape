use crate::champion::ChampionConfig;

/// Filter tabs across the top of the hero-select screen.
pub const FILTERS: [&str; 4] = ["ALL", "MELEE", "RANGED", "MAGIC"];

/// State for the pre-match champion picker.
pub struct HeroSelect {
    pub open: bool,
    pub selected: usize,
    pub hover: Option<usize>,
    pub champions: Vec<ChampionConfig>,
    /// Active filter tab.
    pub filter: usize,
    /// Champion indices shown by the current filter.
    pub visible: Vec<usize>,
    /// Seconds spent on the screen, used for the selected-card pulse.
    pub time: f32,
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
        };
        select.apply_filter();
        select
    }

    /// Recompute the visible roster for the active filter.
    pub fn apply_filter(&mut self) {
        self.visible = self
            .champions
            .iter()
            .enumerate()
            .filter(|(_, champion)| self.matches(champion))
            .map(|(index, _)| index)
            .collect();
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

    /// The other champions, shown in the rival roster panel.
    pub fn rival_roster(&self) -> Vec<usize> {
        self.champions
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != self.selected)
            .map(|(index, _)| index)
            .take(5)
            .collect()
    }
}

/// Filter tab rectangles.
pub fn filter_rects(width: f32, height: f32) -> Vec<(f32, f32, f32, f32)> {
    let w = 148.0f32.min(width * 0.13);
    let h = 30.0;
    let gap = 8.0;
    let total = FILTERS.len() as f32 * w + (FILTERS.len() - 1) as f32 * gap;
    let x0 = (width - total) * 0.5;
    let y = (height * 0.108).max(62.0);
    (0..FILTERS.len())
        .map(|index| (x0 + index as f32 * (w + gap), y, w, h))
        .collect()
}

pub fn filter_at(width: f32, height: f32, cursor: (f64, f64)) -> Option<usize> {
    filter_rects(width, height)
        .iter()
        .position(|&rect| point_in_rect(rect, cursor))
}

/// Grid area used by the roster cards.
fn grid_area(width: f32, height: f32) -> (f32, f32, f32) {
    let side = (width * 0.215).clamp(190.0, 280.0);
    let x = side + 16.0;
    let w = (width - side * 2.0 - 32.0).max(200.0);
    let y = (height * 0.163).max(94.0);
    (x, y, w)
}

/// Card rectangle `(x, y, w, h)` for each visible hero.
pub fn card_rects(width: f32, height: f32, count: usize) -> Vec<(f32, f32, f32, f32)> {
    let (area_x, area_y, area_w) = grid_area(width, height);
    let gap = 8.0;
    let cols = 8usize;
    let card_w = ((area_w - gap * (cols as f32 - 1.0)) / cols as f32).clamp(64.0, 118.0);
    let card_h = (card_w * 1.34).clamp(86.0, 152.0);
    (0..count)
        .map(|index| {
            let column = index % cols;
            let row = index / cols;
            (
                area_x + column as f32 * (card_w + gap),
                area_y + row as f32 * (card_h + gap),
                card_w,
                card_h,
            )
        })
        .collect()
}

/// Which visible slot the cursor is over.
pub fn card_at(width: f32, height: f32, cursor: (f64, f64), count: usize) -> Option<usize> {
    card_rects(width, height, count)
        .iter()
        .position(|&rect| point_in_rect(rect, cursor))
}

/// Left and right roster panels.
#[allow(clippy::type_complexity)]
pub fn roster_panels(width: f32, height: f32) -> ((f32, f32, f32, f32), (f32, f32, f32, f32)) {
    let (_, area_y, _) = grid_area(width, height);
    let w = (width * 0.215).clamp(190.0, 280.0);
    let h = (height * 0.46).clamp(250.0, 340.0);
    ((14.0, area_y, w, h), (width - 14.0 - w, area_y, w, h))
}

/// Rows of roster cards the layout reserves room for (8 per row).
const GRID_ROWS: f32 = 2.0;

/// Detail panel under the roster grid.
pub fn detail_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let (area_x, area_y, area_w) = grid_area(width, height);
    let card_h = card_rects(width, height, 1)
        .first()
        .map_or(152.0, |rect| rect.3);
    let top = area_y + GRID_ROWS * (card_h + 8.0) + 6.0;
    let h = (height - top - 64.0).clamp(190.0, 320.0);
    (area_x, top, area_w, h)
}

/// Gold "lock in" button, top-right of the detail panel so it never sits on
/// top of the ability list.
pub fn confirm_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let (dx, dy, dw, _) = detail_rect(width, height);
    let w = 188.0f32.min(dw * 0.4);
    let h = 38.0;
    (dx + dw - w - 16.0, dy + 14.0, w, h)
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
    fn descriptions_wrap_to_the_panel_width() {
        let lines = wrap_text("one two three four five six seven eight", 12);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| line.chars().count() <= 12));
    }
}
