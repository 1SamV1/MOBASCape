# Champion kits (data-driven abilities)

A champion file in `data/champions/` may carry a `kit`: one passive and four
abilities. Champions with a kit need no Rust code; the schema lives in
`crates/client/src/kit.rs` and the runtime in
`crates/client/src/animated/kit_runtime.rs`. Champions without a kit (King
Awowogei, Blowpipe Ranger) keep their hand-written abilities.

```json
"kit": {
  "passive": {"type": "nth_attack", "n": 3, "radius": 260, "damage": 0.7},
  "abilities": [
    {"cost": 30, "cooldown": 6, "color": [255, 90, 60], "charges": 1,
     "actions": [ {"type": "area", "at": "front", "radius": 230, "arc": 120,
                   "damage": 22, "on_hit": [{"type": "slow", "pct": 0.3, "duration": 1.5}]} ]},
    ...three more...
  ]
}
```

Ability damage is multiplied by `(1 + ability_power / 100) * (1 + 0.05 * (level - 1))`.
`charges` above 1 makes the cooldown a per-charge recharge.

## Actions

| Type | What it does | Main fields |
|---|---|---|
| `projectile` | Straight skillshot; `count` > 1 fans over `spread` degrees | `speed`, `range`, `damage`, `width`, `pierce`, `explode`, `visual`, `on_hit` |
| `area` | Circle or cone after `delay`; can `repeat` with `scatter` | `at`, `radius`, `arc`, `damage`, `range`, `interval`, `heal_per_hit`, `on_hit` |
| `zone` | Lingering pool that ticks; `count`/`spacing` lay a line | `radius`, `duration`, `damage`, `tick`, `grow`, `heal_caster`, `on_tick` |
| `dash` | Moves the caster, hitting what it passes; `land` actions make leaps | `distance`, `speed`, `to_cursor`, `backwards`, `untargetable`, `land` |
| `buff` | Heal, shield and timed buffs | `heal`, `heal_missing`, `shield`, `duration`, `damage_reduction`, `reflect`, `attack_speed`, `move_speed`, `untargetable`, `rooted` |
| `summon` | Cache NPCs that fight for `duration`; `npcs` gives each one a model | `npc`, `npcs`, `count`, `health`, `damage`, `range`, `interval`, `speed`, `scale` |
| `channel` | Repeats `action` every `interval` while standing still | `duration`, `interval`, `action` |
| `empower` | Strengthens the next basic attacks | `attacks`, `bonus`, `missing_health`, `reset_on_kill`, `on_hit` |
| `transform` | Takes another NPC's model with stat bonuses | `npc`, `duration`, `scale`, `damage`, `range`, `move_speed`, `heal` |

`at` is `point` (cursor, clamped to `range`), `caster`, or `front`.

`visual` names a projectile look from `crates/client/src/fx.rs` (for
example `fire_wave`, `dragonfire`, `trident`, `crystal`, `tongue`). Without
one the shot is a glowing orb in the ability's `color`.

## Attack and cast animations, projectiles

Every champion file (kit or not) may also carry:

| Field | Meaning |
|---|---|
| `attack_anim` | Cache sequence for basic attacks. It is time scaled so the blow lands when `attack_windup` ends. |
| `attack_hit` | Fraction of that clip where the blow connects (default 0.5). |
| `cast_anim` | Sequence played on a successful cast (defaults to `attack_anim`). |
| `projectile` | Basic-attack look for `ranged_auto` champions (defaults: `arrow` for RANGED, the arcane orb otherwise). |

Find a clip with `rift-tools skeleton-anims <idle> --model <model> --sheet
out.png`: it lists every sequence that animates the same skeleton and draws
three poses of each. Animaya-only clips (no frames) cannot be used yet.
Pick looks with `rift-tools spotanims --from A --to B --sheet out.png`.

Attack range is measured edge to edge on the ground: a unit attacks when
the planar distance to its target minus the target's footprint radius is
within `attack_range`. Basic-attack projectiles home and cannot miss;
ability projectiles stay skillshots.

## Ability ranks and the tooltip

Every entry in `abilities` carries hand-authored numbers per rank:

```json
{
  "key": "E", "name": "SWAMP CROAK", "description": "...",
  "damage_label": "Shield",
  "ranks": [
    { "damage": 90, "range": 340, "cooldown": 12 },
    { "damage": 115, "range": 340, "cooldown": 11.5 },
    { "damage": 140, "range": 340, "cooldown": 11 },
    { "damage": 165, "range": 340, "cooldown": 10 }
  ]
}
```

- Q, W and E have four ranks, R three. Every ability starts at rank 1, and
  rank 1 must match the kit (same cooldown, same headline damage). A test
  checks this, and that later ranks never get worse.
- Each level gained gives a skill point. While points are left, the slots
  that can take one glow gold and shimmer; clicking a slot spends the point
  there instead of casting (hotkeys still cast). R's ranks 2 and 3 need
  champion levels 6 and 9 (`ULTIMATE_RANK_LEVELS` in
  `animated/ranks.rs`).
- At runtime, `damage` scales every damage, heal and shield the ability
  deals by `damage / rank_1_damage`. `range` scales shot length, cast range
  and dash length by `range / rank_1_range`. `cooldown` replaces the kit's.
  Caster-centred sizes (cones, self areas) do not scale with range, so keep
  `range` flat across ranks for them.
- Ability damage also grows 5% per champion level for every hero, kit or
  hand-written (`kit::scaled_damage`, `AnimatedScene::hand_damage_mult`).
- After changing numbers, run `--balance-report docs/BALANCE.md` and check
  the flags.
- `damage_label` names the headline number ("Shield", "Heal", ...). Leave
  `damage` out when the ability has no number that scales (a blink, a
  percentage buff), and give `damage_note` instead, e.g. "25% of missing
  health". `range: 0` shows as "Self".
- Hovering a slot shows "Name: description", the rank with pips, and a
  table: the current rank's damage, range and cooldown in green, then each
  rank still to come. Values that would change are gold, and locked R ranks
  show the level they need. Damage includes the hero's current ability
  power and level, so it is what a cast would do now.

## Sounds

Each action plays a shared sound from `data/sounds.json` when it runs:
`kit_projectile`, `kit_dash`, `kit_zone`, `kit_summon`, `kit_channel`,
`kit_empower`, `kit_transform`, and for buffs `kit_heal`, `kit_shield`,
`kit_haste` or `kit_buff` (strongest effect wins). Areas play `kit_area`
when they land. Put an entry with the same name in the champion's `sounds`
to change it for that champion only. To score a whole slot by hand, name
any `q_*` (`w_*`, `e_*`, `r_*`) sound: the shared action sounds go quiet
for that slot and `q_cast` plays on cast. Check an id with
`rift-tools sounds --ids <id>[,<id>...] --out <dir>`.

## Statuses (`on_hit`, `on_tick`)

`slow {pct, duration}`, `root`, `stun`, `knockup`, `fear`, `taunt`
(all `{duration}`), `knockback {distance}`, `pull {distance}`,
`venom` / `bleed {damage, duration}` (damage per second), `heal_cut`,
`mana_drain {amount}`, `armor_shred {amount, duration}`,
`current_health {pct}` (extra pure damage), and `chill {pct, duration}`
(a stacking slow; the third stack roots for 1.2 s). Structures take damage
but ignore crowd control.

## Passives

`minion_aura {radius, damage}`, `nth_attack {n, radius, damage}`,
`second_form {threshold, heal, npc, duration, scale, range, damage}`,
`on_hit {statuses}`, `thick_hide {reduction}`,
`swift {move_speed, dash_bonus}`, `spell_combo {per_stack, max, window}`,
`splash {radius, pct}`, `scholar {xp}`, `frost_bite {pct, duration}`.

## Testing a kit

```sh
cargo run --release -p rift-client -- --champion cerberus --ui-live --demo-abilities
```

The demo stands the hero beside a jungle camp, casts Q, W, E and R one
second apart and logs a report (enemies hurt, killed and afflicted, live
summons, shield, range and scale). `cargo test -p rift-client
every_champion_file_parses` checks every champion file.

## Models

Use the NPC's model list from `rift-tools npc <id>`, but leave out flat
ground-shadow planes (zero height) and translucent aura spheres: OSRS draws
them translucent, this renderer draws them solid.
