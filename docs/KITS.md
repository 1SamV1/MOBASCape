# Champion kits (data-driven abilities)

A champion file in `data/champions/` may carry a `kit`: one passive and four
abilities. Champions with a kit need no Rust code; the schema lives in
`crates/client/src/kit.rs` and the runtime in
`crates/client/src/animated/kit_runtime.rs`. Champions without a kit (King
Awowogei, Blowpipe Ranger, Jad, Kree'arra) keep their hand-written abilities.

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
| `projectile` | Straight skillshot; `count` > 1 fans over `spread` degrees | `speed`, `range`, `damage`, `width`, `pierce`, `explode`, `on_hit` |
| `area` | Circle or cone after `delay`; can `repeat` with `scatter` | `at`, `radius`, `arc`, `damage`, `range`, `interval`, `heal_per_hit`, `on_hit` |
| `zone` | Lingering pool that ticks; `count`/`spacing` lay a line | `radius`, `duration`, `damage`, `tick`, `grow`, `heal_caster`, `on_tick` |
| `dash` | Moves the caster, hitting what it passes; `land` actions make leaps | `distance`, `speed`, `to_cursor`, `backwards`, `untargetable`, `land` |
| `buff` | Heal, shield and timed buffs | `heal`, `heal_missing`, `shield`, `duration`, `damage_reduction`, `reflect`, `attack_speed`, `move_speed`, `untargetable`, `rooted` |
| `summon` | Cache NPCs that fight for `duration`; `npcs` gives each one a model | `npc`, `npcs`, `count`, `health`, `damage`, `range`, `interval`, `speed`, `scale` |
| `channel` | Repeats `action` every `interval` while standing still | `duration`, `interval`, `action` |
| `empower` | Strengthens the next basic attacks | `attacks`, `bonus`, `missing_health`, `reset_on_kill`, `on_hit` |
| `transform` | Takes another NPC's model with stat bonuses | `npc`, `duration`, `scale`, `damage`, `range`, `move_speed`, `heal` |

`at` is `point` (cursor, clamped to `range`), `caster`, or `front`.

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
