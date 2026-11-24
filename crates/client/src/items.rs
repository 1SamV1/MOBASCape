use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ItemConfig {
    pub id: String,
    pub name: String,
    /// Cache item id, used to bake the OSRS inventory icon.
    #[serde(default)]
    pub obj: Option<u32>,
    #[serde(default = "default_category")]
    pub category: String,
    pub color: [u8; 3],
    pub cost: u32,
    #[serde(default)]
    pub damage: f32,
    #[serde(default)]
    pub health: f32,
    #[serde(default)]
    pub mana: f32,
    #[serde(default)]
    pub speed: f32,
    #[serde(default)]
    pub ability: f32,
    /// Shop section override (0 starter .. 3 late); by cost when absent.
    #[serde(default)]
    pub tier: Option<u8>,
    /// Item ids this one upgrades from. Owning them makes it cheaper by
    /// their cost, and buying it uses them up.
    #[serde(default)]
    pub builds_from: Vec<String>,
}

impl ItemConfig {
    /// The shop section this item is listed under: the file's `tier` when
    /// given, else by cost (starter up to 500 gold, early up to 1200, core
    /// up to 3000, late above).
    pub fn tier(&self) -> u8 {
        self.tier
            .unwrap_or(match self.cost {
                0..=500 => 0,
                501..=1200 => 1,
                1201..=3000 => 2,
                _ => 3,
            })
            .min(3)
    }
}

fn default_category() -> String {
    "misc".to_string()
}

/// Selling refunds this share of an item's cost (whole gold, rounded down).
pub const SELL_REFUND_PERCENT: u32 = 70;

/// What selling an item of this cost gives back.
pub fn sell_price(cost: u32) -> u32 {
    cost * SELL_REFUND_PERCENT / 100
}

/// One purchase made on the current shop visit, which Undo can take back
/// for its full price.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Purchase {
    pub index: usize,
    /// True when the item went to the chest to wait for delivery instead of
    /// into the bag.
    pub stored: bool,
    /// Gold actually paid (less than the cost for an upgrade).
    pub paid: u32,
    /// Bag items the upgrade used up; Undo puts them back.
    pub consumed: Vec<usize>,
}

/// What buying an item costs right now, given the bag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upgrade {
    /// Bag items that buying it would use up.
    pub consumed: Vec<usize>,
    /// The item's cost less the used-up items' costs.
    pub price: u32,
}

pub struct ItemShop {
    pub items: Vec<ItemConfig>,
    pub owned: [Option<usize>; 6],
    /// This visit's purchases, newest last. Cleared when the shop closes.
    pub undo: Vec<Purchase>,
    pub open: bool,
    pub selected: Option<usize>,
    pub category: usize,
    /// The live overlay's item-list scroll, in interface pixels.
    pub scroll: i32,
    /// The search box text; empty shows everything.
    pub query: String,
    /// True while keys go to the search box instead of the game.
    pub typing: bool,
    /// The Recommended tab for the current hero (indices into `items`).
    pub recommended: Vec<usize>,
    /// `data/items/recommended.json`: key (champion id, role word or
    /// class) to item ids.
    pub recommendations: std::collections::BTreeMap<String, Vec<String>>,
}

/// Where the Recommended lists live. Optional: without it the tab is empty
/// and the shop opens on ALL.
pub const RECOMMENDED_PATH: &str = "data/items/recommended.json";

/// Parse the Recommended file, dropping `_`-prefixed comment keys and
/// rejecting unknown item ids.
pub fn parse_recommendations(
    text: &str,
    items: &[ItemConfig],
) -> Result<std::collections::BTreeMap<String, Vec<String>>> {
    let raw: std::collections::BTreeMap<String, serde_json::Value> = serde_json::from_str(text)?;
    let mut out = std::collections::BTreeMap::new();
    for (key, value) in raw {
        if key.starts_with('_') {
            continue;
        }
        let ids: Vec<String> = serde_json::from_value(value)
            .with_context(|| format!("{RECOMMENDED_PATH}: {key} must be a list of item ids"))?;
        for id in &ids {
            if !items.iter().any(|item| &item.id == id) {
                anyhow::bail!("{RECOMMENDED_PATH}: {key} names unknown item {id}");
            }
        }
        out.insert(key.to_uppercase(), ids);
    }
    Ok(out)
}

pub const CATEGORY_LABELS: [&str; 8] = [
    "RECOMMENDED",
    "ALL",
    "MELEE",
    "RANGED",
    "MAGIC",
    "ARMOUR",
    "JEWELLERY",
    "BOOTS",
];

impl ItemShop {
    pub fn load() -> Result<Self> {
        let data = std::fs::read_to_string("data/items/items.json")
            .context("reading data/items/items.json")?;
        let items: Vec<ItemConfig> = serde_json::from_str(&data)?;
        validate_recipes(&items)?;
        let recommendations = match std::fs::read_to_string(RECOMMENDED_PATH) {
            Ok(text) => parse_recommendations(&text, &items)?,
            Err(_) => Default::default(),
        };
        Ok(Self {
            items,
            owned: [None; 6],
            undo: Vec::new(),
            recommended: Vec::new(),
            recommendations,
            open: false,
            selected: Some(0),
            category: 1,
            scroll: 0,
            query: String::new(),
            typing: false,
        })
    }

    /// Clear the loadout and close the window for a fresh match.
    pub fn reset(&mut self) {
        self.owned = [None; 6];
        self.undo.clear();
        self.open = false;
        self.selected = Some(0);
        self.category = self.default_category();
        self.scroll = 0;
        self.query.clear();
        self.typing = false;
    }

    /// The item the detail pane shows and Buy/Sell act on: the selection
    /// when the current tab and search show it, else the first item shown.
    pub fn shown(&self) -> Option<usize> {
        let shown = self.filtered_items();
        self.selected
            .filter(|index| shown.contains(index))
            .or_else(|| shown.first().copied())
    }

    /// The tab the shop opens on: Recommended when there is a list.
    pub fn default_category(&self) -> usize {
        if self.recommended.is_empty() { 1 } else { 0 }
    }

    /// Pick the Recommended list for a hero: its id, then each word of its
    /// role, then its class. Returns the key used, if any.
    pub fn recommend_for(&mut self, id: &str, role: &str, class: &str) -> Option<String> {
        let keys = std::iter::once(id.to_uppercase())
            .chain(role.split_whitespace().map(str::to_uppercase))
            .chain(std::iter::once(class.to_uppercase()));
        for key in keys {
            if let Some(ids) = self.recommendations.get(&key) {
                self.recommended = ids
                    .iter()
                    .filter_map(|wanted| self.items.iter().position(|item| &item.id == wanted))
                    .collect();
                self.category = self.default_category();
                return Some(key);
            }
        }
        self.recommended.clear();
        self.category = self.default_category();
        None
    }

    /// The items a bot of this champion buys, cheapest tier first: its
    /// recommended list (champion id, then role words, then class).
    pub fn build_for(&self, id: &str, role: &str, class: &str) -> Vec<ItemConfig> {
        let keys = std::iter::once(id.to_uppercase())
            .chain(role.split_whitespace().map(str::to_uppercase))
            .chain(std::iter::once(class.to_uppercase()));
        for key in keys {
            if let Some(ids) = self.recommendations.get(&key) {
                let mut build: Vec<ItemConfig> = ids
                    .iter()
                    .filter_map(|wanted| self.items.iter().find(|item| &item.id == wanted))
                    .cloned()
                    .collect();
                build.sort_by_key(|item| (item.tier(), item.cost));
                build.dedup_by(|a, b| a.id == b.id);
                return build;
            }
        }
        Vec::new()
    }

    pub fn is_owned(&self, index: usize) -> bool {
        self.owned.iter().flatten().any(|&i| i == index)
    }

    pub fn owned_count(&self, index: usize) -> usize {
        self.owned.iter().flatten().filter(|&&i| i == index).count()
    }

    pub fn has_free_slot(&self) -> bool {
        self.owned.iter().any(Option::is_none)
    }

    /// Record a completed purchase in the first free inventory slot.
    pub fn add_owned(&mut self, index: usize) -> bool {
        if let Some(slot) = self.owned.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(index);
            true
        } else {
            false
        }
    }

    /// Remove the first matching inventory slot.
    pub fn remove_owned(&mut self, index: usize) -> bool {
        if let Some(slot) = self.owned.iter_mut().find(|slot| **slot == Some(index)) {
            *slot = None;
            true
        } else {
            false
        }
    }

    fn index_of(&self, id: &str) -> Option<usize> {
        self.items.iter().position(|item| item.id == id)
    }

    /// The price of `index` given what is in the bag, and which bag items
    /// it would use up (each component once, even with duplicates owned).
    pub fn upgrade(&self, index: usize) -> Upgrade {
        let Some(item) = self.items.get(index) else {
            return Upgrade {
                consumed: Vec::new(),
                price: 0,
            };
        };
        let mut consumed: Vec<usize> = Vec::new();
        for id in &item.builds_from {
            if let Some(part) = self.index_of(id) {
                let used = consumed.iter().filter(|&&c| c == part).count();
                if self.owned_count(part) > used {
                    consumed.push(part);
                }
            }
        }
        let discount: u32 = consumed.iter().map(|&c| self.items[c].cost).sum();
        Upgrade {
            consumed,
            price: item.cost.saturating_sub(discount),
        }
    }

    /// Items that list `index` in their `builds_from`, cheapest first.
    pub fn builds_into(&self, index: usize) -> Vec<usize> {
        let Some(id) = self.items.get(index).map(|item| item.id.as_str()) else {
            return Vec::new();
        };
        let mut into: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.builds_from.iter().any(|from| from == id))
            .map(|(i, _)| i)
            .collect();
        into.sort_by_key(|&i| self.items[i].cost);
        into
    }

    /// End the visit once the window is closed: Undo history and the
    /// search are forgotten.
    pub fn sync_visit(&mut self) {
        if !self.open {
            self.undo.clear();
            self.query.clear();
            self.typing = false;
        }
    }

    /// Change the search text. Keeps the selection when it still matches,
    /// else moves it to the first match, and scrolls back to the top.
    pub fn set_query(&mut self, query: &str) {
        self.query = query.chars().take(MAX_QUERY).collect();
        self.scroll = 0;
        let shown = self.filtered_items();
        if !self.selected.is_some_and(|index| shown.contains(&index)) {
            self.selected = shown.first().copied();
        }
    }

    /// One typed character (letters, digits, space, apostrophe, hyphen).
    pub fn type_char(&mut self, ch: char) {
        if ch.is_alphanumeric() || matches!(ch, ' ' | '\'' | '-') {
            let mut query = self.query.clone();
            query.push(ch);
            self.set_query(&query);
        }
    }

    pub fn backspace(&mut self) {
        let mut query = self.query.clone();
        query.pop();
        self.set_query(&query);
    }

    /// Forget the newest Undo entry for `index` (after selling it, Undo
    /// must not refund it a second time).
    pub fn forget_purchase(&mut self, index: usize) {
        if let Some(pos) = self
            .undo
            .iter()
            .rposition(|p| p.index == index && !p.stored)
        {
            self.undo.remove(pos);
        }
    }

    pub fn affordable(&self, gold: u32) -> Vec<bool> {
        self.items.iter().map(|item| item.cost <= gold).collect()
    }

    pub fn item_count(&self) -> usize {
        self.owned.iter().flatten().count()
    }

    pub fn filtered_items(&self) -> Vec<usize> {
        self.items
            .iter()
            .enumerate()
            .filter(|(_, item)| matches_query(item, &self.query))
            .filter(|(index, item)| match CATEGORY_LABELS.get(self.category) {
                Some(&"RECOMMENDED") => self.recommended.contains(index),
                Some(&"ALL") | None => true,
                Some(label) => item.category.eq_ignore_ascii_case(label),
            })
            .map(|(index, _)| index)
            .collect()
    }
}

/// Longest search text the box accepts.
pub const MAX_QUERY: usize = 24;

/// True when every word of `query` is found in the item's name, id or
/// category, or names a stat it gives ("damage", "health", "mana", "speed",
/// "ability"). Case does not matter; an empty query matches everything.
pub fn matches_query(item: &ItemConfig, query: &str) -> bool {
    let name = item.name.to_lowercase();
    let id = item.id.replace('_', " ");
    let mut stats = String::new();
    for (word, value) in [
        ("damage", item.damage),
        ("health", item.health),
        ("mana", item.mana),
        ("speed", item.speed),
        ("ability power", item.ability),
    ] {
        if value != 0.0 {
            stats.push_str(word);
            stats.push(' ');
        }
    }
    query.to_lowercase().split_whitespace().all(|word| {
        name.contains(word)
            || id.contains(word)
            || item.category.contains(word)
            || stats.contains(word)
    })
}

/// Every `builds_from` id must name another item that costs less, so an
/// upgrade never pays the player.
pub fn validate_recipes(items: &[ItemConfig]) -> Result<()> {
    for item in items {
        for id in &item.builds_from {
            let Some(part) = items.iter().find(|other| &other.id == id) else {
                anyhow::bail!("{} builds from unknown item {id}", item.id);
            };
            if part.id == item.id || part.cost >= item.cost {
                anyhow::bail!("{} builds from {id}, which must cost less", item.id);
            }
        }
    }
    Ok(())
}

/// HUD button that calls the Armadylean delivery guard.
pub fn delivery_button_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    (width - 338.0, height - 132.0, 160.0, 24.0)
}

pub fn delivery_button_at(width: f32, height: f32, cursor: (f64, f64)) -> bool {
    point_in_rect(delivery_button_rect(width, height), cursor)
}

pub fn point_in_rect(rect: (f32, f32, f32, f32), cursor: (f64, f64)) -> bool {
    let (x, y, w, h) = rect;
    let mx = cursor.0 as f32;
    let my = cursor.1 as f32;
    mx >= x && mx <= x + w && my >= y && my <= y + h
}

pub fn shop_window_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let w = 760.0f32.min(width - 40.0);
    let h = 430.0f32.min(height - 40.0);
    ((width - w) * 0.5, (height - h) * 0.5, w, h)
}

pub fn shop_close_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let (x, y, w, _) = shop_window_rect(width, height);
    (x + w - 30.0, y + 8.0, 22.0, 22.0)
}

pub fn shop_category_rect(width: f32, height: f32, index: usize) -> (f32, f32, f32, f32) {
    let (x, y, _, _) = shop_window_rect(width, height);
    (x + 12.0, y + 44.0 + index as f32 * 27.0, 140.0, 23.0)
}

pub fn shop_card_rect(width: f32, height: f32, index: usize) -> (f32, f32, f32, f32) {
    let (x, y, _, _) = shop_window_rect(width, height);
    let col = index % 3;
    let row = index / 3;
    (
        x + 166.0 + col as f32 * 118.0,
        y + 44.0 + row as f32 * 116.0,
        110.0,
        108.0,
    )
}

pub fn shop_detail_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let (x, y, _, _) = shop_window_rect(width, height);
    (x + 526.0, y + 44.0, 222.0, 248.0)
}

pub fn shop_buy_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let (x, y, _, _) = shop_detail_rect(width, height);
    (x + 10.0, y + 194.0, 202.0, 30.0)
}

pub fn shop_sell_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let (x, y, _, _) = shop_detail_rect(width, height);
    (x + 10.0, y + 228.0, 202.0, 24.0)
}

pub fn shop_button_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    (width - 454.0, height - 132.0, 104.0, 24.0)
}

pub fn shop_category_at(width: f32, height: f32, cursor: (f64, f64)) -> Option<usize> {
    (0..CATEGORY_LABELS.len())
        .find(|&index| point_in_rect(shop_category_rect(width, height, index), cursor))
}

pub fn shop_card_at(width: f32, height: f32, cursor: (f64, f64), shop: &ItemShop) -> Option<usize> {
    shop.filtered_items()
        .into_iter()
        .enumerate()
        .find(|(slot, _)| point_in_rect(shop_card_rect(width, height, *slot), cursor))
        .map(|(_, item)| item)
}

pub fn shop_buy_at(width: f32, height: f32, cursor: (f64, f64)) -> bool {
    point_in_rect(shop_buy_rect(width, height), cursor)
}

pub fn shop_sell_at(width: f32, height: f32, cursor: (f64, f64)) -> bool {
    point_in_rect(shop_sell_rect(width, height), cursor)
}

pub fn shop_close_at(width: f32, height: f32, cursor: (f64, f64)) -> bool {
    point_in_rect(shop_close_rect(width, height), cursor)
}

pub fn shop_button_at(width: f32, height: f32, cursor: (f64, f64)) -> bool {
    point_in_rect(shop_button_rect(width, height), cursor)
}

#[cfg(test)]
mod tests {

    fn item(id: &str, cost: u32, from: &[&str]) -> ItemConfig {
        ItemConfig {
            id: id.to_string(),
            name: id.to_string(),
            obj: None,
            category: "melee".to_string(),
            color: [0, 0, 0],
            cost,
            damage: 0.0,
            health: 0.0,
            mana: 0.0,
            speed: 0.0,
            ability: 0.0,
            tier: None,
            builds_from: from.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn upgrades_cost_the_difference_and_use_up_owned_parts_once() {
        let mut shop = ItemShop {
            items: vec![
                item("bronze", 300, &[]),
                item("iron", 550, &["bronze"]),
                item("pair", 1000, &["bronze", "bronze"]),
            ],
            owned: [None; 6],
            undo: Vec::new(),
            open: true,
            selected: None,
            category: 0,
            scroll: 0,
            query: String::new(),
            typing: false,
            recommended: Vec::new(),
            recommendations: Default::default(),
        };
        assert_eq!(
            shop.upgrade(1),
            Upgrade {
                consumed: vec![],
                price: 550
            }
        );
        shop.add_owned(0);
        assert_eq!(
            shop.upgrade(1),
            Upgrade {
                consumed: vec![0],
                price: 250
            }
        );
        assert_eq!(shop.upgrade(2).price, 700, "one bronze owned, one missing");
        shop.add_owned(0);
        assert_eq!(
            shop.upgrade(2),
            Upgrade {
                consumed: vec![0, 0],
                price: 400
            }
        );
        assert_eq!(shop.builds_into(0), vec![1, 2]);
        assert!(validate_recipes(&shop.items).is_ok());
    }

    #[test]
    fn search_words_match_names_categories_and_stats() {
        let mut staff = item("staff_of_fire", 850, &[]);
        staff.name = "Staff of fire".to_string();
        staff.category = "magic".to_string();
        staff.ability = 10.0;
        assert!(matches_query(&staff, ""));
        assert!(matches_query(&staff, "FIRE"));
        assert!(matches_query(&staff, "staff magic"));
        assert!(matches_query(&staff, "ability"));
        assert!(!matches_query(&staff, "health"), "it gives no health");
        assert!(
            !matches_query(&staff, "fire sword"),
            "every word must match"
        );
    }

    #[test]
    fn typing_filters_and_keeps_a_matching_selection() {
        let mut shop = ItemShop {
            items: vec![item("bronze_sword", 300, &[]), item("oak_bow", 500, &[])],
            owned: [None; 6],
            undo: Vec::new(),
            open: true,
            selected: Some(1),
            category: 1,
            scroll: 120,
            query: String::new(),
            typing: true,
            recommended: Vec::new(),
            recommendations: Default::default(),
        };
        for ch in "sw!".chars() {
            shop.type_char(ch);
        }
        assert_eq!(shop.query, "sw", "symbols are ignored");
        assert_eq!(shop.filtered_items(), vec![0]);
        assert_eq!(shop.selected, Some(0), "selection moved to a match");
        assert_eq!(shop.scroll, 0);
        shop.backspace();
        shop.backspace();
        assert_eq!(shop.filtered_items(), vec![0, 1]);
        assert_eq!(shop.selected, Some(0), "still matches, so it stays");
        shop.set_query(&"x".repeat(40));
        assert_eq!(shop.query.len(), MAX_QUERY);
        shop.open = false;
        shop.sync_visit();
        assert!(shop.query.is_empty() && !shop.typing);
    }

    #[test]
    fn recommendations_pick_id_then_role_word_then_class() {
        let items = vec![
            item("bronze_sword", 300, &[]),
            item("staff_of_air", 300, &[]),
            item("bronze_platebody", 350, &[]),
        ];
        let text = r#"{"_doc": "x", "melee": ["bronze_sword"], "MAGIC": ["staff_of_air"],
                       "TANK": ["bronze_platebody"], "jad": ["staff_of_air", "bronze_sword"]}"#;
        let mut shop = ItemShop {
            recommendations: parse_recommendations(text, &items).expect("parse"),
            items,
            owned: [None; 6],
            undo: Vec::new(),
            recommended: Vec::new(),
            open: false,
            selected: None,
            category: 1,
            scroll: 0,
            query: String::new(),
            typing: false,
        };
        assert!(
            !shop.recommendations.contains_key("_DOC"),
            "comments dropped"
        );
        assert_eq!(
            shop.recommend_for("kril", "DEMON BRUISER", "Melee")
                .as_deref(),
            Some("MELEE")
        );
        assert_eq!(shop.recommended, vec![0]);
        assert_eq!(shop.category, 0, "opens on Recommended");
        assert_eq!(shop.filtered_items(), vec![0]);
        shop.selected = Some(1);
        assert_eq!(
            shop.shown(),
            Some(0),
            "a selection the tab hides is not acted on"
        );
        assert_eq!(
            shop.recommend_for("kq", "SWARM TANK", "MELEE").as_deref(),
            Some("TANK")
        );
        assert_eq!(
            shop.recommend_for("jad", "TANK", "RANGED").as_deref(),
            Some("JAD")
        );
        assert_eq!(shop.recommended, vec![1, 0]);
        assert_eq!(shop.recommend_for("x", "", "CHEF"), None);
        assert_eq!(shop.category, 1, "no list: opens on ALL");
        assert!(parse_recommendations(r#"{"MELEE": ["ghost"]}"#, &shop.items).is_err());
    }

    #[test]
    fn the_shipped_recommendations_cover_every_class() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let items: Vec<ItemConfig> = serde_json::from_str(
            &std::fs::read_to_string(root.join("data/items/items.json")).expect("items"),
        )
        .expect("parse items");
        let text = std::fs::read_to_string(root.join(RECOMMENDED_PATH)).expect("recommended");
        let lists = parse_recommendations(&text, &items).expect("recommended parses");
        for class in ["MELEE", "RANGED", "MAGIC"] {
            assert!(lists.get(class).is_some_and(|l| l.len() >= 8), "{class}");
        }
    }

    #[test]
    fn recipes_must_name_cheaper_known_items() {
        assert!(validate_recipes(&[item("a", 100, &["ghost"])]).is_err());
        assert!(validate_recipes(&[item("a", 100, &[]), item("b", 90, &["a"])]).is_err());
    }

    #[test]
    fn the_shipped_recipes_are_valid() {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/items/items.json"),
        )
        .expect("items.json");
        let items: Vec<ItemConfig> = serde_json::from_str(&text).expect("parse");
        validate_recipes(&items).expect("recipes");
        assert!(items.iter().any(|i| !i.builds_from.is_empty()));
    }

    #[test]
    fn selling_refunds_seventy_percent_rounded_down() {
        assert_eq!(sell_price(300), 210);
        assert_eq!(sell_price(1450), 1015);
        assert_eq!(sell_price(1), 0);
    }

    #[test]
    fn undo_history_lasts_one_visit_and_skips_sold_items() {
        let mut shop = ItemShop {
            items: Vec::new(),
            owned: [None; 6],
            undo: vec![
                Purchase {
                    index: 3,
                    stored: false,
                    paid: 300,
                    consumed: vec![],
                },
                Purchase {
                    index: 5,
                    stored: true,
                    paid: 300,
                    consumed: vec![],
                },
                Purchase {
                    index: 3,
                    stored: false,
                    paid: 300,
                    consumed: vec![],
                },
            ],
            open: true,
            selected: None,
            category: 0,
            scroll: 0,
            query: String::new(),
            typing: false,
            recommended: Vec::new(),
            recommendations: Default::default(),
        };
        shop.forget_purchase(3);
        assert_eq!(
            shop.undo,
            vec![
                Purchase {
                    index: 3,
                    stored: false,
                    paid: 300,
                    consumed: vec![]
                },
                Purchase {
                    index: 5,
                    stored: true,
                    paid: 300,
                    consumed: vec![]
                }
            ],
            "selling drops the newest bag purchase of that item"
        );
        shop.forget_purchase(5);
        assert_eq!(
            shop.undo.len(),
            2,
            "stored items are not in the bag to sell"
        );
        shop.sync_visit();
        assert_eq!(shop.undo.len(), 2, "still open");
        shop.open = false;
        shop.sync_visit();
        assert!(shop.undo.is_empty(), "closing ends the visit");
    }

    #[test]
    fn shop_tiers_follow_cost_unless_the_file_names_one() {
        let item = |cost: u32, tier: Option<u8>| -> ItemConfig {
            let mut value = serde_json::json!({
                "id": "x", "name": "x", "color": [0, 0, 0], "cost": cost
            });
            if let Some(tier) = tier {
                value["tier"] = tier.into();
            }
            serde_json::from_value(value).unwrap()
        };
        assert_eq!(item(300, None).tier(), 0);
        assert_eq!(item(500, None).tier(), 0);
        assert_eq!(item(501, None).tier(), 1);
        assert_eq!(item(3000, None).tier(), 2);
        assert_eq!(item(5200, None).tier(), 3);
        assert_eq!(item(5200, Some(1)).tier(), 1, "the file wins");
        assert_eq!(item(100, Some(9)).tier(), 3, "clamped to the last tier");
    }

    use super::*;

    fn test_shop() -> ItemShop {
        let json = r#"[
            {"id":"a","name":"A","category":"melee","color":[1,2,3],"cost":100},
            {"id":"b","name":"B","category":"armour","color":[4,5,6],"cost":200},
            {"id":"c","name":"C","category":"ranged","color":[7,8,9],"cost":300},
            {"id":"d","name":"D","category":"magic","color":[10,11,12],"cost":400},
            {"id":"e","name":"E","category":"jewellery","color":[13,14,15],"cost":500},
            {"id":"f","name":"F","category":"boots","color":[16,17,18],"cost":600}
        ]"#;
        ItemShop {
            items: serde_json::from_str(json).expect("item fixture"),
            owned: [None; 6],
            undo: Vec::new(),
            open: false,
            selected: Some(0),
            category: 1,
            scroll: 0,
            query: String::new(),
            typing: false,
            recommended: Vec::new(),
            recommendations: Default::default(),
        }
    }

    #[test]
    fn shop_cards_hit_test() {
        let height = 720.0;
        let (x, y, w, h) = shop_card_rect(1280.0, height, 0);
        assert!(point_in_rect(
            (x, y, w, h),
            ((x + 10.0) as f64, (y + 10.0) as f64)
        ));
        assert!(!point_in_rect((x, y, w, h), (10.0, 10.0)));
        let (cx, cy, _, _) = shop_category_rect(1280.0, height, 0);
        assert_eq!(
            shop_category_at(1280.0, height, ((cx + 4.0) as f64, (cy + 4.0) as f64)),
            Some(0)
        );
    }

    #[test]
    fn shop_controls_hit_test() {
        let width = 1280.0;
        let height = 720.0;
        let center = |rect: (f32, f32, f32, f32)| {
            (
                (rect.0 + rect.2 * 0.5) as f64,
                (rect.1 + rect.3 * 0.5) as f64,
            )
        };
        assert!(shop_close_at(
            width,
            height,
            center(shop_close_rect(width, height))
        ));
        assert!(shop_buy_at(
            width,
            height,
            center(shop_buy_rect(width, height))
        ));
        assert!(shop_sell_at(
            width,
            height,
            center(shop_sell_rect(width, height))
        ));
        assert!(shop_button_at(
            width,
            height,
            center(shop_button_rect(width, height))
        ));
        let shop = test_shop();
        let card = shop_card_rect(width, height, 0);
        assert_eq!(shop_card_at(width, height, center(card), &shop), Some(0));
    }

    #[test]
    fn category_filtering_works() {
        let mut shop = test_shop();
        shop.category = 1;
        assert_eq!(shop.filtered_items(), vec![0, 1, 2, 3, 4, 5]);
        shop.category = 2;
        assert_eq!(shop.filtered_items(), vec![0]);
        shop.category = 3;
        assert_eq!(shop.filtered_items(), vec![2]);
        shop.category = 4;
        assert_eq!(shop.filtered_items(), vec![3]);
        shop.category = 5;
        assert_eq!(shop.filtered_items(), vec![1]);
        shop.category = 6;
        assert_eq!(shop.filtered_items(), vec![4]);
        shop.category = 7;
        assert_eq!(shop.filtered_items(), vec![5]);
        shop.category = 0;
        assert!(shop.filtered_items().is_empty(), "no Recommended list yet");
    }

    #[test]
    fn inventory_fills_and_removes_six_slots() {
        let mut shop = test_shop();
        for _ in 0..6 {
            assert!(shop.add_owned(0));
        }
        assert!(!shop.has_free_slot());
        assert!(!shop.add_owned(1));
        assert_eq!(shop.item_count(), 6);
        assert_eq!(shop.owned_count(0), 6);
        assert!(shop.remove_owned(0));
        assert!(shop.has_free_slot());
        assert_eq!(shop.owned_count(0), 5);
        assert!(!shop.remove_owned(2));
    }

    #[test]
    fn affordability_tracks_gold() {
        let shop = test_shop();
        assert_eq!(
            shop.affordable(300),
            vec![true, true, true, false, false, false]
        );
        assert_eq!(
            shop.affordable(0),
            vec![false, false, false, false, false, false]
        );
    }
}
