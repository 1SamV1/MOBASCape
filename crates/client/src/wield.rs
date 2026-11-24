//! Wielded weapons: an item's male wield models merged onto a hero.
//!
//! OSRS draws a held weapon by merging the item's wield model into the
//! player's body before animation. The wield model's vertex groups follow
//! the player skeleton (framemap 0), so the weapon swings with the hand for
//! free. Only heroes animated on that skeleton (the humanoid heroes) can
//! wield; boss models have their own skeletons and keep the weapons their
//! models already hold.

use anyhow::{Context, Result};
use rift_cache::{AnimationClip, Cache, ItemDefinition, Model};

/// The framemap every player animation (and so every wield model) uses.
pub const PLAYER_FRAMEMAP: u16 = 0;

/// Item categories in `data/items/items.json` that are held weapons.
pub const WEAPON_CATEGORIES: [&str; 3] = ["melee", "ranged", "magic"];

/// True when `idle` animates the player skeleton, so wield models attach.
pub fn can_wield(idle: &AnimationClip) -> bool {
    idle.frames
        .first()
        .is_some_and(|frame| frame.framemap_id == PLAYER_FRAMEMAP)
}

/// The item's male wield models, recoloured like the item and shifted by its
/// wield offset, ready to merge onto a body.
pub fn load_weapon(cache: &Cache, obj: u32) -> Result<Vec<Model>> {
    let data = cache
        .config(10, obj)
        .with_context(|| format!("loading item {obj}"))?;
    let def = ItemDefinition::parse(obj, &data).with_context(|| format!("parsing item {obj}"))?;
    let mut parts = Vec::new();
    for &id in def.male_models.iter().filter(|&&id| id != 0) {
        let group = cache
            .group(7, id)
            .with_context(|| format!("loading wield model {id} of item {obj}"))?;
        let mut model = Model::load(id, &group.data)?;
        prepare(&mut model, &def);
        parts.push(model);
    }
    if parts.is_empty() {
        anyhow::bail!("item {obj} ({}) has no wield model", def.name);
    }
    Ok(parts)
}

/// Apply the item's palette swaps and wield offset to one wield model.
fn prepare(model: &mut Model, def: &ItemDefinition) {
    if !def.recolors.is_empty() {
        let find: Vec<i16> = def.recolors.iter().map(|&(f, _)| f as i16).collect();
        let replace: Vec<i16> = def.recolors.iter().map(|&(_, r)| r as i16).collect();
        model.recolor(&find, &replace);
    }
    if def.male_offset != 0 {
        for y in &mut model.vertex_y {
            *y += def.male_offset as i32;
        }
    }
}

/// The body with the weapon merged in (weapon parts last, like the client),
/// every part bound to the skeleton so the weapon stays in the hand.
pub fn armed_body(
    body: &[Model],
    weapon: &[Model],
    clips: &[std::sync::Arc<rift_cache::AnimationClip>],
) -> Model {
    let parts: Vec<Model> = body.iter().chain(weapon).cloned().collect();
    crate::animated::bind_parts(&parts, clips, "armed body")
}

/// The weapon a hero should hold: the first held weapon in the inventory,
/// else the champion's cosmetic default. Returns the item's cache id.
pub fn chosen_weapon(
    owned: &[Option<usize>],
    items: &[crate::items::ItemConfig],
    default: Option<u32>,
) -> Option<u32> {
    owned
        .iter()
        .flatten()
        .filter_map(|&index| items.get(index))
        .find(|item| WEAPON_CATEGORIES.contains(&item.category.as_str()))
        .and_then(|item| item.obj)
        .or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::ItemConfig;

    fn item(category: &str, obj: u32) -> ItemConfig {
        serde_json::from_value(serde_json::json!({
            "id": format!("{category}_{obj}"),
            "name": "x",
            "obj": obj,
            "category": category,
            "color": [0, 0, 0],
            "cost": 1
        }))
        .unwrap()
    }

    #[test]
    fn the_first_weapon_in_the_inventory_wins_over_the_default() {
        let items = vec![item("boots", 1), item("magic", 1381), item("melee", 1277)];
        let owned = [Some(0), None, Some(2), Some(1), None, None];
        assert_eq!(chosen_weapon(&owned, &items, Some(9)), Some(1277));
    }

    #[test]
    fn without_a_weapon_the_cosmetic_default_is_held() {
        let items = vec![item("boots", 1), item("armour", 2)];
        let owned = [Some(0), Some(1), None, None, None, None];
        assert_eq!(chosen_weapon(&owned, &items, Some(1381)), Some(1381));
        assert_eq!(chosen_weapon(&owned, &items, None), None);
    }

    #[test]
    fn the_wield_offset_and_recolours_are_applied() {
        let mut model = Model {
            vertex_y: vec![10, -4],
            face_colors: vec![5, 7],
            ..empty_model()
        };
        let def = ItemDefinition {
            male_offset: 6,
            recolors: vec![(5, 9)],
            ..Default::default()
        };
        prepare(&mut model, &def);
        assert_eq!(model.vertex_y, vec![16, 2]);
        assert_eq!(model.face_colors, vec![9, 7]);
    }

    fn empty_model() -> Model {
        Model {
            id: 0,
            vertex_count: 0,
            face_count: 0,
            num_texture_faces: 0,
            vertex_x: Vec::new(),
            vertex_y: Vec::new(),
            vertex_z: Vec::new(),
            faces: Vec::new(),
            face_colors: Vec::new(),
            face_render_types: Vec::new(),
            face_textures: None,
            texture_coords: None,
            face_normals: Vec::new(),
            vertex_groups: None,
            follow_groups: None,
            priority: 0,
            face_alphas: None,
        }
    }
}
