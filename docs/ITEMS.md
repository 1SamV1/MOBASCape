# Items

The item roster in `data/items/items.json`, the rules it follows, and the
mechanics the engine gives it. The tables below are generated from the
JSON; the JSON is the source of truth.

## Design rules

- **Commons are stat sticks.** The metal ladders (bronze to rune), the
  bow ladder, the elemental staffs and the platebody ladder only give
  stats. They are the parts everything else is built from.
- **Every late item (over 3000 gold) has a passive or an active.** A
  test (`every_rare_item_does_something_beyond_stats`) keeps it that way.
  Rare items give somewhat fewer raw stats than before; their power is in
  what they do.
- **Each class has its own verbs.**
  - *Melee*: on-hit brawling. Cleave, bleed and venom, armor break,
    stuns and shoves, lifesteal, and OSRS **special attacks** (actives
    that empower the next swing: Puncture, The Judgement, Smash).
  - *Ranged*: crits (enchanted bolts), attack speed that ramps (Rapid
    fire), extra range, ricochets, and big-game damage for sieging.
  - *Magic*: ability power plus **ability-hit effects** (burn, ice,
    blight, mana drain), spell vamp, ability amplification, cooldown
    recovery, mana regeneration and targeted bolts.
  - *Armour*: health and **armor**, damage reduction, ranged ward,
    reflect and regeneration.
  - *Jewellery*: the odd jobs. Recoil, Ring of life's last stand,
    Wealth's bounty, auras, lifesteal and armor penetration.
  - *Boots*: move speed, and the late boots each have an active (shield,
    stomp, sprint, restore).
- **Fusions are the mega-rares.** A fusion cannot be bought. Its parts and
  its recipe scroll fuse into it automatically once all of them are in the
  bag at the same time. Every fusion has its own passive, and most have an
  active as well.
- **Duplicates do not stack.** A second copy of an item gives its stats
  but not its passives.

## Fusion mechanics

- A fusion lists `fuse` (its part ids) and `recipe_cost`. At load,
  `items::finish_items` adds a recipe scroll (`recipe_<id>`, a clue-scroll
  icon) and sets the fusion's cost to its parts plus the recipe.
- In the shop a fusion shows what is still missing as its price. **Buy**
  buys the next missing part (the recipe last), so pressing Buy until it
  is done works. The shop's Fusions tab lists every fusion as a
  `part + part + recipe > fusion` row.
- When the bag holds every part and the recipe (including deliveries from
  the chest), they fuse at once (`fuse_ready` in `main.rs`): the parts'
  stats come off, the fusion's go on, and there is a burst and a
  "FUSED" banner. Fusing is final, so it clears this visit's Undo history.
- Bots buy their recommended list cheapest tier first. A fusion uses up
  the parts a bot already carries and costs that much less. With a full
  bag, a bot sells its cheapest item for the usual refund to make room.

## Effect reference

Passives (`item_effects::ItemPassive`, JSON `type`): `crit`, `lifesteal`,
`spell_vamp`, `cleave` (melee only), `ricochet` (ranged only), `on_hit`
(every Nth attack, chance, statuses, and flat, max-health or
current-health bonus damage with a cap against non-heroes),
`ability_hit` (statuses on ability damage, with an internal gap),
`armor_pen`, `berserk`, `execute`, `giant_slayer`, `big_game`,
`ability_amp`, `elysian`, `damage_reduction`, `ranged_ward`, `reflect`,
`regen`, `last_stand`, `bounty`, `frenzy`, `spellblade`, `thrall_master`,
`aura`.

Actives (`item_effects::ActiveEffect`): `special` (the next basic attack
lands extra hits, statuses, a heal or a splash), `nova`, `bolt`,
`shield`, `haste`, `rapid`, `restore`, `refresh`. Use one with its bag
slot key (1-6) while the shop is closed. A slot greys out on cooldown and
shows its key in gold when the active is ready. Bots use offensive
actives on the hero they are fighting, and defensive ones when they are
below 40% health with an enemy hero near.

Statuses are the kit `Status` type (`slow`, `stun`, `root`, `venom`,
`bleed`, `heal_cut`, `mana_drain`, `armor_shred`, `chill`, `silence`,
`taunt`, `knockback` and so on), so items and abilities share one set
of crowd control.

New item stats: `armor`, `attack_speed` (%), `haste` (% cooldown
recovery) and `range`.

Engine hooks (`animated/gear.rs`): the damage path takes a basic-attack
flag set by every attack path (player, bots, kit heroes, projectiles),
asks `gear_outgoing_mult`, `gear_incoming_mult` and `gear_armor_pen`,
then calls `gear_after_damage`. Item procs run at `gear_depth > 0` and
never trigger further procs. `gear_tick` runs cooldowns, timers,
regeneration and auras, and arms Spellblade whenever an ability
cooldown jumps up (a cast).

## Catalogue

### Melee

| Item | Cost | Stats | Identity |
|---|---:|---|---|
| Bronze sword | 300 | +10 dmg | - |
| Iron sword (from Bronze sword) | 550 | +16 dmg | - |
| Steel sword (from Iron sword) | 850 | +23 dmg | - |
| Mithril sword (from Steel sword) | 1200 | +30 dmg | - |
| Adamant sword (from Mithril sword) | 1700 | +38 dmg | - |
| Rune sword (from Adamant sword) | 2400 | +46 dmg, +5% AS | **Keen edge** (on hit) |
| Dragon scimitar | 3200 | +42 dmg, +10% AS | **Sever** (on hit) |
| Abyssal whip | 4200 | +50 dmg, +20% AS | **Energy drain** (on hit) |
| Ghrazi rapier | 5200 | +58 dmg, +15% AS | **Piercing stab** (armor pen) |
| Armadyl godsword | 6500 | +72 dmg, +100 hp | *Active:* **The Judgement** (special, 22s) |
| Elder maul | 7000 | +82 dmg, +150 hp, -15% AS | **Crushing weight** (on hit); **Siege breaker** (big game) |
| Scythe of vitur | 8200 | +70 dmg, +200 hp, +5% AS | **Sanguine sweep** (cleave); **Vitur's harvest** (big game) |
| Dragon dagger | 1800 | +22 dmg, +15% AS | *Active:* **Puncture** (special, 18s) |
| Rune 2h sword | 2600 | +46 dmg, -10% AS | **Wide swing** (cleave) |
| Granite maul | 2200 | +40 dmg, +80 hp, -10% AS | **Quick smash** (on hit) |
| Zamorakian spear | 2600 | +36 dmg, +150 hp | **Shove** (on hit) |
| Abyssal dagger | 3500 | +40 dmg, +15% AS | **Abyssal venom** (on hit); *Active:* **Abyssal puncture** (special, 16s) |
| Dragon warhammer | 3000 | +44 dmg, +120 hp | *Active:* **Smash** (special, 28s) |
| Dharok's greataxe | 3800 | +54 dmg, +150 hp, -10% AS | **Wretched strength** (berserk) |
| Guthan's warspear | 3400 | +40 dmg, +180 hp | **Infestation** (lifesteal) |
| Torag's hammers | 3400 | +36 dmg, +220 hp, +8 armor | **Corruption** (on hit) |
| Kraken tentacle | 1200 | +12 dmg, +60 hp | **Grasp** (on hit) |

### Ranged

| Item | Cost | Stats | Identity |
|---|---:|---|---|
| Shortbow | 300 | +9 dmg | - |
| Oak shortbow (from Shortbow) | 500 | +15 dmg | - |
| Willow shortbow (from Oak shortbow) | 800 | +21 dmg | - |
| Maple shortbow (from Willow shortbow) | 1200 | +27 dmg | - |
| Yew shortbow (from Maple shortbow) | 1700 | +34 dmg | - |
| Magic shortbow (from Yew shortbow) | 2400 | +38 dmg, +15% AS | *Active:* **Snapshot** (special, 14s) |
| Rune crossbow | 3200 | +42 dmg, +40 range | **Diamond bolts (e)** (crit) |
| Dragon crossbow | 4200 | +48 dmg, +40 range | *Active:* **Annihilate** (special, 22s) |
| Armadyl crossbow | 5200 | +52 dmg, +60 range | **Armadyl eye** (crit) |
| Toxic blowpipe | 6000 | +44 dmg, +25% AS | **Rapid fire** (frenzy); **Toxic darts** (on hit); *Active:* **Toxic siphon** (special, 20s) |
| Heavy ballista | 6500 | +84 dmg, +160 range, -25% AS | **Siege bolts** (big game) |
| Twisted bow | 8000 | +68 dmg, +60 range | **Giant's bane** (giant slayer) |
| Bow of faerdhinen | 7500 | +62 dmg, +100 hp, +10% AS | **Crystal shards** (ricochet) |
| Karil's crossbow | 3000 | +34 dmg, +20% AS | **Tainted shot** (on hit) |
| Dark bow | 3800 | +50 dmg, +40 range, -20% AS | *Active:* **Descent of darkness** (special, 18s) |

### Magic

| Item | Cost | Stats | Identity |
|---|---:|---|---|
| Staff of air | 300 | +6 dmg, +20 mana, +4% AP | - |
| Staff of water (from Staff of air) | 450 | +9 dmg, +30 mana, +6% AP | - |
| Staff of earth (from Staff of water) | 650 | +12 dmg, +40 mana, +8% AP | - |
| Staff of fire (from Staff of earth) | 850 | +15 dmg, +50 mana, +10% AP | - |
| Mystic fire staff (from Staff of fire) | 1400 | +18 dmg, +70 mana, +15% AP | **Mana flow** (regen) |
| Ancient staff | 2200 | +22 dmg, +90 mana, +20% AP | **Ice barrage** (ability hit) |
| Ahrim's staff | 3000 | +26 dmg, +110 mana, +28% AP | **Ahrim's blight** (ability hit) |
| Trident of the seas | 3800 | +30 dmg, +120 mana, +22% AP | *Active:* **Trident surge** (bolt, 14s) |
| Sanguinesti staff | 5000 | +34 dmg, +140 mana, +30% AP | **Blood spell** (spell vamp) |
| Kodai wand | 6200 | +36 dmg, +180 mana, +38% AP, +15% haste | **Infinite water** (regen) |
| Tumeken's shadow | 8500 | +40 dmg, +150 mana, +48% AP | **Shadow amplification** (ability amp) |
| Tome of fire | 1800 | +40 mana, +15% AP | **Burning pages** (ability hit) |
| Book of the dead | 1500 | +40 mana, +10% AP, +5% haste | **Arceuus tome** (thrall master) |
| Staff of the dead | 4000 | +24 dmg, +200 hp, +80 mana, +24% AP | *Active:* **Power of death** (shield, 40s) |

### Armour

| Item | Cost | Stats | Identity |
|---|---:|---|---|
| Bronze platebody | 350 | +80 hp, +4 armor | - |
| Iron platebody (from Bronze platebody) | 550 | +120 hp, +6 armor | - |
| Steel platebody (from Iron platebody) | 850 | +170 hp, +9 armor | - |
| Mithril platebody (from Steel platebody) | 1200 | +230 hp, +12 armor | - |
| Adamant platebody (from Mithril platebody) | 1700 | +300 hp, +15 armor | - |
| Rune platebody (from Adamant platebody) | 2500 | +380 hp, +18 armor | - |
| Dragon platebody | 3500 | +420 hp, +24 armor | **Dragon scales** (ranged ward) |
| Fighter torso | 2800 | +8 dmg, +300 hp, +10 armor | **Barbarian assault** (regen) |
| Bandos chestplate | 4500 | +8 dmg, +460 hp, +26 armor | **Bandos might** (damage reduction) |
| Armadyl chestplate | 4500 | +320 hp, +14 armor, +10% AS | **Armadyl grace** (crit) |
| Ancestral robe top | 4800 | +300 hp, +60 mana, +20% AP | **Ancestral power** (ability amp) |
| Torva platebody | 6500 | +12 dmg, +620 hp, +32 armor | **Nex's wrath** (reflect) |
| Rune platelegs | 2200 | +280 hp, +14 armor | - |
| Dragon platelegs | 3000 | +340 hp, +18 armor | - |
| Bandos tassets | 4000 | +6 dmg, +380 hp, +18 armor | **War chief** (regen) |
| Armadyl chainskirt | 4000 | +300 hp, +10 armor, +10% AS | **Swift strikes** (frenzy) |
| Ahrim's robetop | 3200 | +320 hp, +40 mana, +18% AP | **Ahrim's robes** (regen) |
| Ahrim's robeskirt | 3000 | +280 hp, +30 mana, +14% AP, +10% haste | - |
| Mystic robe top | 1400 | +200 hp, +30 mana, +10% AP | - |
| Mystic robe bottom | 1200 | +180 hp, +25 mana, +8% AP | - |
| Black d'hide body | 1800 | +220 hp, +8 armor, +5% AS | - |
| Karil's leathertop | 3000 | +300 hp, +12 armor, +15 move, +10% AS | - |

### Jewellery

| Item | Cost | Stats | Identity |
|---|---:|---|---|
| Ring of recoil | 400 | +40 hp | **Recoil** (reflect) |
| Ring of life | 500 | +60 hp | **Ring of life** (last stand) |
| Ring of wealth | 600 | +40 hp | **Wealth** (bounty) |
| Warrior ring | 1800 | +10 dmg, +10% AS | - |
| Berserker ring | 2200 | +14 dmg | **Berserk** (berserk) |
| Seers ring | 2000 | +30 mana, +15% AP, +10% haste | - |
| Archers ring | 2000 | +8 dmg, +40 range, +10% AS | - |
| Ring of suffering (from Ring of recoil) | 3500 | +220 hp, +10 armor | **Suffering** (reflect) |
| Lightbearer | 3000 | +20 mana, +10% AP, +25% haste | - |
| Ultor ring | 5000 | +22 dmg | **Ultor's bite** (armor pen) |
| Venator ring | 5000 | +16 dmg, +15% AS | **Venator's eye** (crit) |
| Magus ring | 5000 | +40 mana, +25% AP | **Magus focus** (ability amp) |
| Amulet of accuracy | 300 | +5 dmg | - |
| Amulet of glory | 900 | +6 dmg, +40 hp, +10 mana | **Charged** (regen) |
| Amulet of fury (from Amulet of glory) | 1800 | +10 dmg, +80 hp, +20 mana, +10% AP | **Fury** (regen) |
| Amulet of torture | 3200 | +20 dmg, +10% AS | **Torture** (lifesteal) |
| Necklace of anguish | 3200 | +14 dmg, +50 range, +15% AS | **Anguish** (on hit) |
| Occult necklace | 2800 | +50 mana, +25% AP | **Occult** (ability amp) |
| Salve amulet(ei) | 2400 | +150 hp, +10% AP | **Salve aura** (aura) |
| Fire cape | 2500 | +10 dmg, +150 hp, +6 armor | **TzHaar resilience** (regen) |

### Boots

| Item | Cost | Stats | Identity |
|---|---:|---|---|
| Leather boots | 200 | +25 move | - |
| Mystic boots | 900 | +20 mana, +20 move, +8% AP | - |
| Ranger boots (from Leather boots) | 1500 | +5 dmg, +35 move, +10% AS | - |
| Dragon boots | 1800 | +8 dmg, +50 hp, +25 move | - |
| Bandos boots | 2200 | +100 hp, +6 armor, +25 move | - |
| Infinity boots | 1600 | +40 mana, +25 move, +10% haste | - |
| Guardian boots (from Bandos boots) | 3000 | +160 hp, +12 armor, +30 move | *Active:* **Guardian's stand** (shield, 45s) |
| Primordial boots (from Dragon boots) | 4200 | +12 dmg, +100 hp, +35 move | *Active:* **Primordial stomp** (nova, 30s) |
| Pegasian boots (from Ranger boots) | 4200 | +8 dmg, +45 move, +10% AS | *Active:* **Pegasus sprint** (haste, 25s) |
| Eternal boots (from Infinity boots) | 4200 | +80 hp, +40 mana, +30 move, +12% AP | *Active:* **Arcane tide** (restore, 60s) |

## Fusions

| Fusion | Parts + recipe | Total | Stats | Identity |
|---|---|---:|---|---|
| Abyssal tentacle (melee) | Abyssal whip + Kraken tentacle + recipe (1000) | 6400 | +70 dmg, +150 hp, +25% AS | **Energy drain** (on hit); **Tentacle lash** (on hit); *Active:* **Tentacle bind** (special, 20s) |
| Inquisitor's mace (melee) | Elder maul + Bandos chestplate + recipe (1500) | 13000 | +90 dmg, +450 hp, +25 armor | **Inquisitor's crush** (on hit); **Bandos might** (damage reduction); *Active:* **Righteous smash** (nova, 30s) |
| Osmumten's fang (melee) | Ghrazi rapier + Abyssal dagger + recipe (1300) | 10000 | +80 dmg, +20% AS | **Double roll** (crit); **Jaws of Osmumten** (armor pen); *Active:* **Eviscerate** (special, 20s) |
| Dragon claws (melee) | Dragon dagger + Dragon warhammer + recipe (1200) | 6000 | +60 dmg, +30% AS | **Ferocity** (frenzy); *Active:* **Slice and dice** (special, 22s) |
| Voidwaker (melee) | Zamorakian spear + Ancient staff + recipe (1300) | 6100 | +55 dmg, +120 hp, +80 mana, +30% AP | **Void edge** (spellblade); *Active:* **Disrupt** (special, 24s) |
| Zaryte crossbow (ranged) | Armadyl crossbow + Karil's crossbow + recipe (1500) | 9700 | +70 dmg, +80 range, +10% AS | **Armadyl eye** (crit); **Ruby bolts (e)** (on hit); *Active:* **Evoke** (special, 25s) |
| Dragon hunter crossbow (ranged) | Dragon crossbow + Rune crossbow + recipe (1200) | 8600 | +64 dmg, +60 range | **Dragonbane** (big game); **Diamond bolts (e)** (crit); *Active:* **Annihilate** (special, 18s) |
| Venator bow (ranged) | Magic shortbow + Venator ring + recipe (1400) | 8800 | +56 dmg, +25% AS | **Venator's bounce** (ricochet); **Venator's eye** (crit); *Active:* **Hunting volley** (rapid, 30s) |
| Webweaver bow (ranged) | Yew shortbow + Necklace of anguish + recipe (1100) | 6000 | +46 dmg, +50 range, +20% AS | **Spider venom** (on hit); *Active:* **Webweave** (special, 22s) |
| Harmonised nightmare staff (magic) | Kodai wand + Lightbearer + recipe (1600) | 10800 | +40 dmg, +200 mana, +50% AP, +30% AS, +35% haste | **Harmony** (regen); *Active:* **Harmonised surge** (refresh, 40s) |
| Volatile nightmare staff (magic) | Trident of the seas + Tome of fire + recipe (1400) | 7000 | +34 dmg, +140 mana, +45% AP | **Volatile flames** (ability hit); *Active:* **Immolate** (bolt, 24s) |
| Eldritch nightmare staff (magic) | Ahrim's staff + Occult necklace + recipe (1400) | 7200 | +30 dmg, +160 mana, +50% AP | **Eldritch drain** (ability hit); **Occult** (ability amp); *Active:* **Invocate** (bolt, 22s) |
| Ancient sceptre (magic) | Ancient staff + Seers ring + recipe (1200) | 5400 | +26 dmg, +120 mana, +38% AP, +15% haste | **Ice barrage** (ability hit); *Active:* **Blood barrage** (nova, 28s) |
| Elysian spirit shield (armour) | Dragon platebody + Ring of life + recipe (1600) | 5600 | +520 hp, +30 armor | **Elysian** (elysian); **Ring of life** (last stand); *Active:* **Blessed ward** (shield, 50s) |
| Dinh's bulwark (armour) | Dragon platelegs + Bandos tassets + recipe (1500) | 8500 | +10 dmg, +600 hp, +35 armor | **Bulwark** (damage reduction); *Active:* **Shield bash** (nova, 35s) |
| Justiciar chestguard (armour) | Torva platebody + Fighter torso + recipe (1700) | 11000 | +14 dmg, +800 hp, +40 armor | **Justiciar** (damage reduction); **Nex's wrath** (reflect); *Active:* **Justice** (shield, 45s) |
| Infernal cape (jewellery) | Fire cape + Amulet of torture + recipe (1500) | 7200 | +26 dmg, +220 hp, +10 armor, +10% AS | **TzKal fury** (lifesteal); **Inferno heart** (berserk); *Active:* **Inferno** (nova, 30s) |
| Amulet of rancour (jewellery) | Amulet of torture + Amulet of fury + recipe (1200) | 6200 | +28 dmg, +100 hp, +15% AS | **Rancour** (execute); **Torture** (lifesteal) |
| Amulet of blood fury (jewellery) | Amulet of fury + Ring of life + recipe (1000) | 3300 | +14 dmg, +120 hp, +20 mana, +15% AP | **Blood fury** (lifesteal); **Blood rite** (spell vamp) |
