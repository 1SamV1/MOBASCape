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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Purchase {
    pub index: usize,
    /// True when the item went to the chest to wait for delivery instead of
    /// into the bag.
    pub stored: bool,
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
}

pub const CATEGORY_LABELS: [&str; 7] = [
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
        Ok(Self {
            items: serde_json::from_str(&data)?,
            owned: [None; 6],
            undo: Vec::new(),
            open: false,
            selected: Some(0),
            category: 0,
            scroll: 0,
        })
    }

    /// Clear the loadout and close the window for a fresh match.
    pub fn reset(&mut self) {
        self.owned = [None; 6];
        self.undo.clear();
        self.open = false;
        self.selected = Some(0);
        self.category = 0;
        self.scroll = 0;
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

    /// End the visit's Undo history once the window is closed.
    pub fn sync_visit(&mut self) {
        if !self.open {
            self.undo.clear();
        }
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
            .filter(|(_, item)| match self.category {
                1 => item.category == "melee",
                2 => item.category == "ranged",
                3 => item.category == "magic",
                4 => item.category == "armour",
                5 => item.category == "jewellery",
                6 => item.category == "boots",
                _ => true,
            })
            .map(|(index, _)| index)
            .collect()
    }
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
                },
                Purchase {
                    index: 5,
                    stored: true,
                },
                Purchase {
                    index: 3,
                    stored: false,
                },
            ],
            open: true,
            selected: None,
            category: 0,
            scroll: 0,
        };
        shop.forget_purchase(3);
        assert_eq!(
            shop.undo,
            vec![
                Purchase {
                    index: 3,
                    stored: false
                },
                Purchase {
                    index: 5,
                    stored: true
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
            category: 0,
            scroll: 0,
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
        assert_eq!(shop.filtered_items(), vec![0, 1, 2, 3, 4, 5]);
        shop.category = 1;
        assert_eq!(shop.filtered_items(), vec![0]);
        shop.category = 2;
        assert_eq!(shop.filtered_items(), vec![2]);
        shop.category = 3;
        assert_eq!(shop.filtered_items(), vec![3]);
        shop.category = 4;
        assert_eq!(shop.filtered_items(), vec![1]);
        shop.category = 5;
        assert_eq!(shop.filtered_items(), vec![4]);
        shop.category = 6;
        assert_eq!(shop.filtered_items(), vec![5]);
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
