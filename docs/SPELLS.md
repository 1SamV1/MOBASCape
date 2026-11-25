# Spells: graphics, battle spells and teleport scrolls

This is the design and asset reference for magic in MobaScape: the cache
graphics that spells use, the battle spells that magic heroes pick before a
match, and the teleport scroll every hero carries. Kit abilities themselves
are documented in [KITS.md](KITS.md).

Every visual here comes from the pinned OSRS cache. Nothing is drawn by hand
and no cache art is committed.

## Decisions

- **Battle spells are for MAGIC-class heroes only** (`"class": "MAGIC"`).
  Today that's Kolodion, the Wise Old Man, the Ancient Wyvern, TzTok-Jad and
  the Chaos Elemental. Each picks two before the match, on D and F.
- **Every hero carries a teleport scroll** that works like Dota 2's Town
  Portal: its own slot on T, a channel to an allied structure, a cooldown.
- **Lane calls never force a pick.** The lane only pre-fills the two spell
  slots; any spell can go in either.

## Checking a graphic by eye

Spot animations carry no names in the cache, so an id is only "verified"
once someone has watched it play. Two tools do that:

```sh
# A still contact sheet, one tile per id (first frame of the clip).
target/release/rift-tools spotanims --from 340 --to 395 --sheet out.png

# The clips playing in the arena, in a labelled grid by the hero, looping.
target/release/rift-client --ui-live --champion kolodion --distance 1500 \
    --demo-gfx 76,77,78,363,369 --screenshot out.png --screenshot-after 60
```

`rift-tools spotanim <id>` prints an id's model, sequence and recolours.
Ids sharing a model and differing only by recolour (for example 550 and 553)
are colour variants of one effect. In `--demo-gfx`, a label ending in `?`
means that id failed to load.

## Verified spot animations

Checked on contact sheets and in `--demo-gfx`. "Sheet" is what the first
frame shows; "Use" is where MobaScape puts it.

| Id | Sheet | OSRS source | Use |
|---|---|---|---|
| 76 | white zigzag bolt | Saradomin Strike | Kolodion E hit |
| 77 | purple cage of claws | Claws of Guthix | Kolodion Q hit |
| 78 | pillar of fire | Flames of Zamorak | Kolodion W land |
| 80 | small yellow stars | stun | stun hold |
| 85 | grey splash flecks | splash | spell on a shield or untargetable unit |
| 111 | purple teleport shape | standard teleport | teleport scroll arrival, Teleport Other hit |
| 177 | green spikes rising | Bind / Entangle | Entangle cast and root hold |
| 178 | green stars | Snare projectile | (spare) |
| 179-181 | red discs | not bind | not used |
| 342 | purple bars | Tele Block | Tele Block hold |
| 343 | purple starburst | Tele Block cast | Tele Block cast |
| 344 | dark streaks | projectile | Tele Block projectile |
| 345 | purple flecks | Tele Block hit | (spare) |
| 359-362 | ice spray and shards | Ice Rush/Burst | (spare) |
| 363 | ice spike pillar | Ice Burst | Ancient Wyvern Q hit |
| 368 | ice flecks | Ice Blitz projectile | Ice Barrage projectile |
| 369 | ice block | Ice Barrage | freeze hold |
| 372 | red streaks | Blood projectile | (second wave) |
| 374 | red blob | Blood Blitz | (second wave) |
| 377 | blood drip from above | Blood Barrage | Blood Barrage hit (second wave) |
| 378-383 | black shadow shapes | Shadow spells | (spare) |
| 384-391 | smoke | Smoke spells | (spare) |
| 392 | rising purple blocks | Ancient teleport | Ancient Teleport, both ends |
| 550 / 551 / 552 | green orb / with trail / green ring | Chaos Elemental | Confusion projectile and hit |
| 553 / 554 / 555 | red orb / with trail / red ring | Chaos Elemental (recolour of 550-552) | Madness projectile and hit |
| 556 / 557 / 558 | three-colour swirl | Chaos Elemental | Discord projectile and hit |
| 559 | red spikes | Chaos Elemental | Disarm hold |
| 678 | blue crystal shards | teleport tablet break | teleport scroll, origin |
| 725 | white skull | Vengeance Other | (spare) |
| 726 | red skull | Vengeance | Vengeance cast |
| 744 / 745 | white sparkle burst | Heal Other / Heal Group | Heal Group cast and ally hit, Wise Old Man E |
| 747 | purple teleport shape | Lunar teleport | (spare) |
| 750 | blue beam | Lunar spell | teleport scroll destination marker |
| 1290 | purple swirl portal | Arceuus teleport | (spare) |
| 1568 / 1569 | black fly cloud (two loops) | Pestilent Bloat's flies | Bloat W: 1568 held on the Bloat, 1569 on each enemy bitten |
| 1574 | thrown limb (no clip) | Theatre of Blood | `bloat_hook` projectile (Bloat Q) |
| 1576 | red blood splat | Theatre of Blood | `bloat_hook` impact, Bloat R hit |
| 1579 | blood pool, 6 s | Theatre of Blood | Bloat E cast |

## Part A: spell visual language

- Every kit action can carry a `gfx` block with these graphics:
  - `cast` plays on the caster.
  - `land` plays at an area or zone centre.
  - `hit` plays on each unit hit.
  - `hold` stays on a unit while a status lasts.

  Ids are preloaded with the match.
- Statuses show these held graphics:

  | Status | Graphic |
  |---|---|
  | Ice root | 369 |
  | Entangle root | 177 |
  | Stun | 80 |
  | Tele Block | 342 |
  | Disarm | 559 |
- **Overhead chat**: OSRS yellow text with a black shadow above a unit for
  about 2.4 s ("Taste vengeance!"). Both HUDs draw it. `AnimatedScene::say`
  adds a line, and a new line replaces the speaker's old one.
- **Splash**: 85 plays when a spell lands on a shield or an untargetable
  unit.
- **Current magic heroes**:

  | Hero | Ability | Graphic |
  |---|---|---|
  | Kolodion | Q Claws of Guthix | hit 77 |
  | Kolodion | W Flames of Zamorak | land 78 |
  | Kolodion | E Saradomin Strike | hit 76 |
  | Ancient Wyvern | Q | hit 363 |
  | Ancient Wyvern | R | freeze hold 369 |
  | Wise Old Man | W | hit 111 |
  | Wise Old Man | E | cast 744 |

## Part B: teleport scroll (every hero)

- **Key and slot**: T, in its own slot, with an OSRS teleport scroll icon.
- **Channel**:
  - Lasts 3 s, then you arrive beside an allied tower, inhibitor or the
    nexus.
  - You pick the destination by clicking a structure in the world or on the
    minimap. With no click, it goes to the structure nearest the cursor.
- **What breaks it**:
  - Stun, knock-up, fear, taunt, knockback and pull break the channel.
  - Moving or casting cancels it.
- **Cooldown**: 80 s, with no purchase. It's tuned in
  `data/spells/teleport_scroll.json`.
- **Counterplay**: both teams see the arrival.
  - The destination shows beam 750 in a team-coloured ring and a minimap
    ping.
  - When you arrive, 111 plays.
  - The origin shows the tablet break, 678.
- **Bots** teleport back to their lane after respawning or a long walk home.

## Part C: battle spells (MAGIC heroes, two picks, D and F)

On hero select, a magic hero gets two spell slots in the detail panel. Each
opens a picker laid out like the OSRS spellbook. A non-magic hero's slots are
greyed out ("Magic heroes only"), and team rows show each hero's spells.

The lane call pre-fills the two slots:

| Lane | Default spells |
|---|---|
| Mid | Ancient Teleport + Ice Barrage |
| Support | Heal Group + Vengeance |
| Anything else | Ancient Teleport + Heal Group |

Camera follow moves from F to Y to free F.

### How it is built

- `data/spells/battle_spells.json` holds the spells. Each has an `effect`
  (`blink`, `heal_group`, `vengeance`, `bolt` or `nova`), a cooldown, an
  OSRS spellbook icon sprite and a `gfx` block like a kit action's. Bolt and
  nova effects carry ordinary kit statuses (`on_hit`). The client also
  builds the file in, so a missing copy never empties the book.
- `crate::spells` is pure and tested: loadouts, refusals, targeting,
  Vengeance. `animated/battle.rs` runs them for any caster, so bots cast the
  same spells as the player. The kit runtime is player-only, which is why
  battle spells don't run through it.
- Tele Block is a new status, `no_mobility`. It stops kit dashes and blinks,
  refuses the teleport scroll, and breaks a scroll being read. Its held
  graphic is 342.
- Bolts are homing projectiles with two new looks: `ice_barrage` (368) and
  `tele_block` (344, landing 345). They pick the enemy hero nearest the cursor
  within range, and within 450 of the cursor.
- Vengeance fires on the first damage from an enemy hero. It sends the share
  back as pure damage, and that damage can't set off another Vengeance.
- Icons are cache sprites: 341 Ancient Teleport, 565 Heal Group,
  564 Vengeance, 328 Ice Barrage, 321 Entangle, 352 Tele Block. They are
  baked as `icon.spell.<id>` for both HUDs.
- Hovering D, F or T in the assembled interface explains the spell or the
  scroll. Non-magic heroes see greyed D and F slots ("Magic heroes only").

### Bots

Magic bots carry their lane call's pair and check it once a second, on their
think tick, casting at most one spell then. The rules live in
`spells::bot_aim` and are tested:

| Spell | When a bot casts it |
|---|---|
| Ancient Teleport | Under 25% health with an enemy hero within 900. It blinks toward its fountain. |
| Heal Group | Under 35% health, or when two allied heroes nearby are under 60% |
| Vengeance | An enemy hero within 300 |
| Ice Barrage, Tele Block | An enemy hero in range under half health, or the bot is under 40% (to peel) |
| Entangle | An enemy hero and at least one other enemy inside the radius |

### On hero select

- The two slots sit under the hero's name and tags on the parchment. The
  recess gave up 40 px for them. A magic hero's slot opens a stone picker
  over the centre column, grouped Standard, Ancient, Lunar like the OSRS
  spellbooks. Hovering a spell explains it.
- Picking the spell already in the other slot swaps the two, so they never
  match.
- The lane call refills both slots until you pick one yourself. After that
  your picks stay put.
- Each team row shows its hero's two spells in the top right corner, or the
  teleport scroll for a hero without them. Magic bots show their lane call's
  pair, which is what they carry into the match.
- `--spells` presets the picks; `--spell-picker 0|1` opens the picker for
  screenshots.

### First wave

| Spell | Book | Effect | Cooldown | Graphics |
|---|---|---|---|---|
| Ancient Teleport | Ancient | Blink up to 450 toward the cursor | 110 s | 392 at both ends |
| Heal Group | Lunar | Heal 15% max health; allies within 600 heal 10% | 100 s | cast 744, 745 on each ally |
| Vengeance | Lunar | For 6 s, the next hero hit on you reflects 75% back | 60 s | cast 726, "Taste vengeance!" when it fires |
| Ice Barrage | Ancient | Bolt at a hero within 700: 90 damage and a 1.5 s freeze | 90 s | projectile 368, hold 369 |
| Entangle | Standard | Root enemies within 320 of you for 1.25 s | 90 s | cast and hold 177 |
| Tele Block | Standard | Enemy hero within 700: no dashes, blinks or scroll, and half healing, for 4 s | 90 s | cast 343, projectile 344, hold 342 |

### Second wave (later)

- **Blood Barrage**: damage from missing health, with lifesteal. Graphic 377.
- **Cure Me**: cleanse, then 1 s immune to crowd control.
- **Ward of Arceuus**: a shield.
- **Smite**: true damage to jungle monsters.
- **Charge**: faster attacks.
- **Resurrect Thrall**: a summon.

## Part D: Chaos Elemental (npc 2054)

Shipped in `data/champions/chaos_elemental.json`. A MAGIC hero (role "CHAOS
MAGE") built from the boss's three OSRS attacks: Discord (a hit), Confusion
(teleports you nearby) and Madness (strips your gear).

| | |
|---|---|
| Models | 11216 and 57329 (`rift-tools npc 2054`), scale 0.6 |
| Animations | idle 3144, walk 3145, attack 3146 |
| Stats | 520 health, 300 mana, 600 range, ranged basic attacks |
| Basic attack | the `chaos` look: the Discord swirl (557) landing in its ring (558) |
| Icons | 586 vortex, 16 Confuse spiral, 120 red brain, 578 storm cloud |

Its kit:

| Slot | Name | What it does | Graphics |
|---|---|---|---|
| Passive | Pure chaos | Every third ability hit also lands a random 30% slow, venom or 25 mana drain (`Passive::Chaos`) | 558 on the proc |
| Q | Discord | Skillshot, 80 damage and 30 mana drain | `chaos` look: 557, 558 |
| W | Confusion | Burst at the cursor (radius 220): 50 damage, and each enemy is thrown 350 in a random direction (`scramble`) | 550 where it lands, 552 at both ends of each throw |
| E | Madness | Skillshot: 45 damage and the first enemy hit is disarmed for 2 s (`disarm`) | `madness` look: 554, 555, then holds 559 |
| R | Chaos Storm | 1.2 s channel, a three-bolt fan every 0.4 s, 45 damage each; every bolt lands one of a 40 mana drain, a 250 throw or a 1.5 s disarm (`one_of`) | `chaos` look |

Two things changed from the first design, after watching them play:

- Madness is a skillshot rather than a point-and-click on a hero within 650,
  so it matches its OSRS projectile and has counterplay.
- Confusion's throw uses the Chaos Elemental's own green splash (552) rather
  than the teleport sparkle (111). 111 is a solid model that reads as a
  purple blob at camp-monster size.

Bots cast it from the rank tables like every hero (ADR-055).

