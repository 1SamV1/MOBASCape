# Ability rework — proposals for approval

Status: **proposal, nothing below is built yet** except where marked DONE.
Every ability has an ID (`AWO-Q`, `BLT-R`, ...). Approve, change or reject
by ID; an ability is only built once its description is approved. The quoted
line under each ability is the in-game tooltip text that would ship.

DONE this round (asked for directly): Vine Swing's full-range speed curve
(`AWO-W`) and the Flesh Hook's Barrelchest anchor on a rope (`BLT-Q`).

Names are OSRS or original. Where a mechanic comes from a Dota hero the
entry says so, but no Dota ability name is reused.

## Approval log (2026-11-25)

Decisions override the proposal text below.

- **SYS-1** approved (all seven modes). **SYS-2** approved.
- **SYS-3** approved: per-rank durations and counts, plus a max-rank bonus.
  Abilities start at rank 0 and the **level cap rises to 25**.
- **SYS-4**, **SYS-5** approved. **SYS-6** approved: silence, hex, evasion,
  mark, duel, gear-lock.
- **SYS-7** approved: R ranks at 6/11/16, a pick-one-of-two talent at
  10/15/20/25, small stat bumps on the remaining levels.
- **SYS-8** yes, designed after the kits. **SYS-9** first, before any new
  ability is built.
- **AWO-P** approved. **AWO-Q** changed: the max-rank bonus is a ricochet,
  where the banana bounces once to a second enemy on the way back (replaces
  the spin). **AWO-W** approved (two charges at max rank). **AWO-E**
  approved.
- **AWO-R** changed: text kept, plus his attacks become automatic area slams
  that mini-stun (0.25 s) every enemy in each slam, and an enemy hero kill
  while a gorilla extends the form by 6/7/8 s. Max-rank bonus: lasts 10 s
  (the knock-up is dropped since every slam stuns).
- **BLT-P Flesh Heap** changed: no more max health. Each enemy death nearby
  banks bonus damage (+3 per minion, +30 per hero) that the next hit spends
  in full, then the bank resets.
- **BLT-Q** approved (pure damage, reel-in, wall-grapple bonus). **BLT-W
  Pestilence** approved.
- **BLT-E Meat Stomp** (replaces Play Dead and Bloated Hide): "The Bloat
  stomps his right foot, making a stomp sound and a small puff of dust just
  above the ground, and dismembered hands fall from above in front of him
  (a shadow on the ground shows first), dealing pure damage and mini-stuns
  to enemies." A cone 600 long in front over 1 s, each hand 40 pure damage
  and a 0.3 s stun. Max-rank bonus: the hands rain in a ring around the
  Bloat with a longer reach. Assets: spot animations 1570-1573 (four models on one falling clip, beside the Bloat's flies) are the falling-limb candidates, confirmed on a sheet before use.
- **BLT-R** approved.
- **WYV-P** approved. **WYV-Q** changed: a 900-long line of frost instead
  of the cone (chill, third chill freezes); max-rank shatter bonus kept.
  **WYV-W Ice Shard** approved. **WYV-R** approved.
- **WYV-E Wyvern Flight** (replaces Ancient Ward; Glacial Tomb rejected):
  "Takes off and glides to the cursor, chilling enemies under the landing."
  900/950/1000/1050 range, landing chill within 300. Max-rank bonus: the
  landing freezes instead of chilling.
- **FRG-P** approved. **FRG-Q** rejected: Tongue Lash stays today's pull;
  max-rank bonus roots the pulled enemy 0.8 s. **FRG-W**, **FRG-E**,
  **FRG-R** approved.
- **CAL-P**, **CAL-Q**, **CAL-W**, **CAL-E**, **CAL-R** approved.
- **CER-P**, **CER-Q**, **CER-W**, **CER-E** approved. **CER-R** changed:
  the howl also summons 2/3/4 hellhounds that chase feared enemies for 6 s;
  fear 1.6/1.9/2.2 s and the +20% damage max-rank bonus stay.
- **CHA-P**, **CHA-Q**, **CHA-W**, **CHA-R** approved. **CHA-E** rejected:
  Madness stays disarm-only; max-rank bonus slows 30%. Gear-lock is
  unused until something needs it.
- **GRA-P**: the passive's effect changes (to be decided). **GRA-Q**,
  **GRA-W**, **GRA-E**, **GRA-R** approved.
- **JAD-P**, **JAD-Q**, **JAD-W** approved (Jad loses his blink).
- **JAD-E** approved with a change: a healer dies to any single hit from a
  hero's attack or ability.
- **JAD-R Inferno Unleashed** approved with changes: lasts 10/13/17 s, the
  wind-up is 0.3 s, and the boulder lands sooner after the warning than in
  Jad's Rhythm.
- **GRA-P Bone Crusher** (replaces the minion aura): his attacks on slowed
  or taunted enemies deal +4% of their max health (capped against minions).
- **KQ-P** changed: the winged form has less armour but more move speed.
- **KQ-Q** changed: max-rank bonus heals her for 20% of the damage, and the
  bitten enemy takes 30% more damage from her for 4 s.
- **KQ-W Undermine** changed: no damage along the path; when she bursts out
  she damages and mini-stuns enemies in an area around her.
- **KQ-E** approved: each cast reflects and stuns each attacker once, and
  the next cast can do it to the same attacker again.
- **KQ-R Kalphite Swarm** changed (unit target): workers burrow up around
  an enemy hero, close in and attack it for the duration. They have heavy
  armour and little health, so they can be killed.
- **KOL**, **KRE**, **KRL**, **RNG**, **VEN**, **VOR**, **WOM**, **ZIL**:
  approved as proposed below ("go ahead with your recommendations").
  **WOM-R** is A, Party Room Balloons.
- Everything approved is to be built into the game.

## 1. What Dota 2 does (research)

| Topic | Dota 2 today | Source |
|---|---|---|
| Targeting | Behaviour flags: no target (fires on key press), unit target, point, AoE (point plus a radius ring), channelled (broken by moving or silence), autocast, toggle, passive, immediate (skips the order queue), directional. Vector targeting (press at a start point, drag for a direction: Pangolier's Swashbuckle) is layered on point targeting. | Valve workshop docs, Liquipedia |
| AoE shapes | Circle at a point or around the caster, line (width), cone, wave. AoE-radius bonuses widen circles, lines and cones, except hooks and chains (Meat Hook, Timber Chain). | Liquipedia Area of Effect |
| Levels | Level cap 30. Basic abilities 4 ranks, ultimate 3 ranks at 6/12/18. Talent pairs at 10/15/20/25 (pick one of two); the whole tree unlocks at 30. Attribute points can be bought from level 2 (7.38). | dota2.com 7.00, Liquipedia 7.38 |
| Innates | Every hero has an innate ability with no skill points. Since 7.41 (March 2026) innates no longer scale with other abilities: they give fixed values or scale with hero level (Beastmaster's Inner Beast: 7 + 3 attack speed per level). | 7.41 coverage |
| Facets | Added in 7.36 (May 2024), removed in 7.41; their effects moved into basic abilities, Shard or Scepter upgrades. Lesson: per-hero build choices live better inside abilities and items than in a separate pick. | 7.41 coverage |
| Item upgrades | Aghanim's Scepter and Shard give every hero a custom upgrade: a better ultimate, an improved ability or a new one. | Liquipedia Aghanim's Shard |
| Damage | Physical (armour), magical (25% base magic resistance) and pure (unreduced). Debuff immunity (Black King Bar) blocks most disables since 7.33. Basic and strong dispels remove statuses. | Liquipedia damage types |
| Builds | Spell amplification multiplies ability damage; attack modifiers scale with attack damage. 7.42 (18 Nov 2026) adds Item Potency so on-hit item effects scale per attack, and cuts spell amp, AoE and cast range from items. | Liquipedia 7.42 |

What we take from it:

1. **A wider set of targeting modes.** Unit-target, ally-target, vector,
   hold-to-charge, toggle, autocast and recast join quick-cast skillshots.
   That gives heroes different skill floors: point-and-click for new
   players, skillshots and vectors for experts.
2. **Ranks that change behaviour, not only numbers.** Maxing an ability
   adds a twist (its *mastery*), the way Shard upgrades do.
3. **Innates that grow with hero level**, the 7.41 way, so every hero has
   one and none costs a skill point.
4. **Build identity from scaling.** Each ability says what it scales with,
   so the items you buy decide which abilities hit hard.

## 2. What is wrong now (audit of all 18 kits)

- **Seven heroes share one E**: a self-shield with damage reduction, a heal
  or empowered hits on top (Ancient Ward, Swamp Croak, Miner's Endurance,
  Flechtalon Guard, Infernal Shield, Bloated Hide, Ward of Ages).
  Callisto's Hibernate and the Kalphite Queen's Chitin are close.
- **Three pulls** (Flesh Hook, Tongue Lash, Telekinetic Grab) and **six
  near-identical leaps or dashes** (Big Leap, Pounce, Burrow, Bandos'
  Wrath, Kree's Leap, Jad's Teleport).
- **Jad and Kree have generic kits**: a nova or bolt, a blink or leap, a
  shield and a falling-meteor ultimate. Awowogei, Jad, Kree and the Ranger
  have no passive.
- **Four names are used twice**: Swarm (Bloat W, Kalphite Queen R), Flames
  of Zamorak (Kolodion W, K'ril Q), Saradomin Strike (Kolodion E, Zilyana
  Q) and War Cry (Graardor's passive and his E). The god spells stay with
  Kolodion, who casts them in the Mage Arena; the others are renamed below.
- **Every ability scales the same way**: authored damage x (1 + ability
  power/100) x (1 + 5% per level). Weapons and armour do nothing for
  abilities; only ability power raises their damage, so builds barely
  differ.
- **Items carry five stats**: attack damage, ability power, health, mana and
  move speed. There are no armour, magic resistance, attack speed,
  cooldown or on-hit stats and no item passives. The proposals use only
  these five; a later item pass can add more.
- **Levels:** the cap is 11 and each level gives one point. Every ability
  starts at rank 1, so by level 11 all but one of the 15 ranks are taken.
  R's ranks need levels 1/6/9. A rank scales damage, heals, shields, reach
  and cooldown; durations, counts and slows stay at their rank 1 values.
- **Bots don't use the kits.** They cast a generic reading of each hero's
  rank table (bolt, nova, strike, dash) because the kit runtime serves one
  caster, the player. New mechanics will only reach bots once that
  changes (`SYS-9`).

## 3. Shared systems (approve these first)

**SYS-1 Targeting modes.** Quick-cast stays the default: the key casts at
the cursor. New modes:

| Mode | Input | Used by |
|---|---|---|
| Unit | Key casts on the enemy under the cursor (nearest within a small pick radius). Out of range, the hero walks into range first. | WYV-W, FRG-Q, FRG-E, CAL-W, BLT-R, GRA-R, KRE-R, WOM-Q |
| Ally | As unit, on an ally or yourself | WYV-E, FRG-Q, WOM-E |
| Vector | Press at the start point, move, release for the direction (a tap uses hero to cursor) | CER-W, VOR-W |
| Charge | Hold to charge, release to fire | KRE-Q |
| Toggle | Press for on, press again for off | BLT-W |
| Autocast | Right-click the slot toggles auto-applying it to attacks | KRL-E |
| Recast | A second press inside the window does the follow-up | BLT-E, CAL-E, WOM-Q |

**SYS-2 AoE shapes and indicators.** Keep circle, self circle, cone, line,
dash, boomerang and swing. Add a vector line (two points), a unit ring
(target highlight plus cast range), a growing charge line and a travelling
zone (Tornado-style area that moves along a path).

**SYS-3 Rank scaling and masteries.** Ranks keep their authored numbers
and can also scale durations and counts (the "1.25/1.5/1.75/2 s" values
below); today only damage, heals, shields, reach and cooldown grow. The
last rank (Q/W/E rank 4, R rank 3) adds one twist, listed as **Mastery**
below. The order you max abilities in then changes how the hero plays
mid-game.

*Optional:* abilities start at rank 0, pick one per level at levels 1-3,
and R unlocks at level 3 (ranks at 3/6/9). That is Dota's 6/12/18 scaled
to our cap of 11. Today R is live from level 1.

**SYS-4 Innates.** Every hero gets a passive that grows with hero level
(base + per level) and costs no points. Existing kit passives keep their
effect and gain the per-level growth listed with them; Awowogei, Jad, Kree
and the Ranger get new ones.

**SYS-5 Scaling tags.** These replace the blanket ability power
multiplier. Each ability names one or two tags, and the tag decides which
items power it:

| Tag | Formula | Items that feed it |
|---|---|---|
| Attack | + x% attack damage (physical) | swords, whip, godswords, bows, blowpipe, berserker/ultor rings, torture |
| Spell | x (1 + ability power / 100), as today | staves, wands, robes, magus ring, occult, eternal boots |
| Bulk | + x% max health | platebodies, legs, bandos/torva, guardian boots, suffering |
| Mana | + x% max mana | staves, Tumeken's shadow, seers ring, occult |
| Speed | + x% move speed | boots, whip, blowpipe, d'hide, Karil's |

The +5% per hero level stays on every ability.

**SYS-6 New statuses.**

| Status | Effect |
|---|---|
| Silence | No abilities |
| Hex | Turned into a frog: no attacks or abilities, slow fixed speed |
| Evasion | Attacks aimed at the unit miss |
| Mark | Takes x% more damage |
| Duel | Two units can only attack each other |
| Gear-lock | Items give no damage, ability power or speed |

**SYS-7 (optional) Talents.** One of two per hero at levels 4/7/10, as in
Dota. Designed after kits are approved.

**SYS-8 (optional) An Aghanim's-style item** that upgrades each hero's
ultimate. Belongs to the item pass; designed after kits.

**SYS-9 Bots on real kits.** Make the kit runtime serve any caster, so bots
(and later networked players) cast the actual abilities. This is a large
refactor and the next step for the server-authoritative plan.

### Roster at a glance

| Hero | Role | Difficulty | Builds (main / second) | Signature |
|---|---|---|---|---|
| King Awowogei | trickster skirmisher | Medium | Attack / Bulk | longest swing in the game, banana traps |
| Pestilent Bloat | hook tank | Medium | Bulk / Spell | anchor hook, play dead, devour |
| Ancient Wyvern | frost controller | Medium | Spell / Bulk | freeze combos, ice tomb save |
| Big Frog | hexer bruiser | Easy | Bulk / Speed | frog kiss hex, hop chains |
| Callisto | charging initiator | Easy | Speed / Bulk | cross-map unit charge |
| Cerberus | zone tank | Medium | Bulk / Spell | lava vectors, three souls |
| Chaos Elemental | disruptor | Hard | Mana / Spell | Madness locks enemy gear |
| General Graardor | war commander | Easy | Bulk / Attack | Bandos' Challenge duel |
| JalRek-Jad | telegraphed titan | Medium | Spell / Bulk | Jad attacks, healers |
| Kalphite Queen | burrowing tank | Medium | Bulk / Spell | undermine line stun, two lives |
| Kolodion | god-caster | Hard | Spell / Mana | four shapeshift forms |
| Kree'arra Jr. | wind marksman | Medium | Attack / Speed | charged gale shot, feather flurry |
| K'ril Tsutsaroth | infernal executioner | Easy | Attack / Bulk | burning blade, YARRRRRRR! |
| Blowpipe Ranger | venom sniper | Medium | Attack / Speed | venom stacks, deadshot |
| Venenatis | web stalker | Hard | Attack / Speed | web zones, brood frenzy |
| Vorkath | siege dragon | Medium | Spell / Attack | acid vectors, rapidfire |
| Wise Old Man | trickster support | Hard | Spell / Mana | telekinetic throw, steal or party |
| Commander Zilyana | holy duelist | Medium | Attack / Speed | godsword heal, dash resets |

Assets: NPC and model ids below were looked up in the pinned cache with
`rift-tools`. Spot animations in docs/SPELLS.md are verified. Anything marked
*(pick)* is chosen from `rift-tools spotanims --sheet` while building, and
lands in SPELLS.md before a kit may use it. Sounds: each reworked ability
gets its own cache sounds in its champion file (`q_*`, `w_*`, ... names, as
Awowogei has today), picked from the sounds the OSRS NPC's own animations
play (`rift-tools sounds --sequences`). Until then the shared kit sounds in
`data/sounds.json` play. Done 2026-11-27 through per-action `gfx.sounds`
(docs/STATUS.md).

## 4. Heroes

### King Awowogei — trickster skirmisher

Strong: the longest mobility in the game, poke, traps. Weak: 500 health;
helpless while Vine Swing is down; peels are visible. Builds: **Attack**
(whip, scimitar, torture) for Bananarang and Monkey Madness; **Bulk** for
the gorilla.

**AWO-P Monkey Madness** (NEW innate)
> Hit the same enemy hero four times within 6 seconds with attacks or bananas, and the king's next four attacks deal bonus damage and heal him for 30% of it.

- Bonus 15 + 3 per hero level. Inspired by Jingu Mastery; named for the
  Ape Atoll quest.

**AWO-Q Bananarang** (KEEP, add mastery)
> Hurls a giant banana at the cursor. It curves out, hangs at full range, then loops home to the king's hand, striking enemies both ways.

- 45/65/85/105 + 60% attack damage; return hits deal 50%. Crits stun
  briefly (as now).
- **Mastery:** the banana spins at the apex for 0.8 s, hitting anything
  there every 0.2 s for 40%.
- Assets: banana model 2387 (in use). Engine: none.

**AWO-W Vine Swing** (DONE this round, add mastery)
> A long vine drops from the canopy. The king leaps on and swings far toward the cursor, gathering speed until he flies at full tilt.

- 1500/1600/1700/1800, always full range; untargetable until 0.5 s after
  landing.
- **Mastery:** two charges.

**AWO-E Banana Peel** (KEEP, add mastery)
> Throws a peel onto the target circle. Land it under an enemy to slip them at once, or leave it as a trap for the next one to step on it.

- **Mastery:** a slipping enemy that slides into another enemy slips it too.

**AWO-R Greegree** (KEEP, add mastery)
> The king turns into a hulking gorilla for 8 seconds, gaining health and trading speed for heavy slams that splash around his target.

- Bonus health 250/400/550 + 40% of health from items (Bulk).
- **Mastery:** lasts 10 s; every third slam knocks enemies up for 0.5 s.
- Assets: gorilla NPC 5275 models and clips (in use).

### Pestilent Bloat — hook tank

Strong: picks from the fog, durable, heals off Devour. Weak: slowest hero
(330), 170 attack range, long hook cooldown, Pestilence hurts it too.
Builds: **Bulk** (torva, bandos, platebodies) for Pestilence, Play Dead and
Devour;
**Spell** for the hook.

**BLT-P Flesh Heap** (KEEP)
> Gains maximum health whenever an enemy dies nearby.

- 2 per minion, 25 per hero, plus 0.2 / 2 per hero level.

**BLT-Q Flesh Hook** (anchor DONE; mechanic change and mastery proposed)
> Flings the Barrelchest anchor on a grapple rope. The first enemy it catches is dragged back along the rope to the Bloat.

- 80/115/150/185 (Spell) as pure damage; today armour reduces it, as it
  does every ability. Range 1100-1250.
- **Change:** the catch is reeled in at hook speed along the rope, as in
  Dota. Today it snaps in over 0.25 s.
- **Mastery:** the anchor bites into trees and walls, and the Bloat hauls
  itself to them.
- Assets: anchor model 23133 on a rope (in use), splat 1576. Engine:
  terrain hits.

**BLT-W Pestilence** (REWORK of Swarm, toggle)
> Toggle: the Bloat's flies swarm around it, biting and slowing every enemy close by every half second. They bite the Bloat too.

- 20/30/40/50 per second + 1.5% of max health (Bulk); the Bloat takes 40%
  of that; 25% slow.
- **Mastery:** bitten enemies heal 35% less.
- Assets: flies 1568 held, 1569 on hits (verified). Engine: toggle.

**BLT-E Play Dead** (NEW, replaces Bloated Hide)
> The Bloat collapses for 1.5 seconds, rooted, taking 40% less damage and regaining health. Then it heaves itself up with a stomp that stuns everything close by. Press again to stand early.

- Regains 4% max health per 0.5 s (Bulk). Stomp: 300 radius, 0.8 s stun,
  60/90/120/150 damage.
- **Mastery:** stomp radius 400, and it drags enemies 150 inward.
- OSRS: the Theatre of Blood Bloat's shutdown and stomp. Engine: recast.

**BLT-R Devour** (REWORK: unit target)
> Seizes an enemy in reach and chews on it for 2.5 seconds. It is stunned throughout, and the Bloat heals for all the damage.

- 25/35/45 + 1% of the Bloat's max health every 0.5 s (Bulk + Spell).
- **Mastery:** cast range 400; the target is dragged into reach first.
- Engine: unit target.

### Ancient Wyvern — frost controller

Strong: freeze combos, a save, zone ultimate. Weak: short breath, 520
health, no mobility. Builds: **Spell**; **Bulk** for Glacial Tomb.

**WYV-P Frozen Breath** (KEEP)
> Attacks chill for 2.5 seconds.

- Slow 15% + 1% per hero level.

**WYV-Q Icy Breath** (KEEP, add mastery)
> A cone of frost that chills enemies. A third chill freezes them solid.

- **Mastery:** frozen enemies shatter for 80 bonus damage (Spell) when the
  freeze ends.
- Assets: 363 (verified).

**WYV-W Ice Shard** (NEW, replaces Tail Sweep)
> Hurls an ice shard at an enemy. It shatters, slowing the target and splintering into every other enemy nearby.

- Splinters 70/100/130/160 within 400 and chill (Spell). The target is
  slowed 30% for 2 s.
- **Mastery:** the target takes the splinter damage too.
- Inspired by Winter Wyvern's Splinter Blast. Assets: 368 projectile, 363
  splinters (verified). Engine: unit target.

**WYV-E Glacial Tomb** (NEW, replaces Ancient Ward)
> Encases an ally or itself in ice for 2.5 seconds. Inside it cannot act or be harmed, and heals quickly.

- Heals 20 + 4% max health every 0.5 s (Bulk).
- **Mastery:** when the tomb breaks, it chills enemies within 300.
- Inspired by Cold Embrace. Assets: ice block 369 (verified). Engine: ally
  target.

**WYV-R Frost Storm** (KEEP, add mastery)
> A blizzard at the cursor that chills everything inside.

- **Mastery:** enemies still inside after 2 seconds are frozen for 1 s.

### Big Frog — hexer bruiser

Strong: the hex, sticky hops, a teamfight ultimate. Weak: melee, no
sustain, low damage. Builds: **Bulk**; **Speed** for hops.

**FRG-P Sticky Tongue** (KEEP)
> Attacks slow for 1 second.

- 15% + 1% per hero level.

**FRG-Q Tongue Lash** (REWORK: the frog travels, not the target)
> Lashes its tongue onto a unit and zips to it. An enemy is slowed; an ally is shielded.

- Range 700. Enemy: 45/65/85/105 + 30% move speed (Speed), 40% slow for
  1.5 s. Ally: 80/110/140/170 shield.
- **Mastery:** an enemy is also rooted for 0.8 s on arrival.
- Engine: unit and ally target.

**FRG-W Hop, Hop, Splash** (REWORK of Big Leap)
> Three short hops held as charges. Each lands with a splash; the third hop in a row lands a big splash that knocks enemies up.

- 300 per hop. Splash 30/40/50/60 + 20% move speed (Speed); the big splash
  deals double and knocks up for 0.6 s.
- **Mastery:** hops recharge 30% faster. Engine: charges (exists).

**FRG-E Frog Kiss** (NEW, replaces Swamp Croak)
> Turns an enemy into a frog for a moment. It cannot attack or cast, and can only hop slowly about.

- Range 600; 1.25/1.5/1.75/2 s.
- **Mastery:** a hexed enemy takes 15% more damage.
- OSRS: the Kiss the Frog random event. Assets: frog NPC 312 (model 6947)
  for the hexed body. Engine: unit target, hex, model swap on an enemy.

**FRG-R Plague of Frogs** (TWEAK)
> Frogs rain on a wide circle for three seconds. Each one hits and slows; an enemy hit by three frogs is turned into a frog.

- Radius 450. A three-frog hex lasts 1 s.
- **Mastery:** three frogs stay behind to fight for 8 s (today's summon).
- Assets: big frog NPC 8701 (in use).

### Callisto — charging initiator

Strong: catches anyone, displacement. Weak: telegraphed, interrupted by
stuns, weak wave clear, kited after the charge. Builds: **Speed**
(pegasian, whip, Karil's); **Bulk**.

**CAL-P Thick Hide** (KEEP)
> Takes 15% less damage.

**CAL-Q Fury Maul** (REWORK of Maul)
> Swipes in front. Every Maul or attack on the same enemy adds a Fury stack, and each stack adds damage to the next hits.

- 55/75/95/115 + 80% attack damage (Attack). Stacks +8/12/16/20 for 6 s,
  up to 5.
- **Mastery:** stacks don't expire while Callisto keeps hitting that enemy.
- Inspired by Ursa's Fury Swipes.

**CAL-W Wilderness Charge** (REWORK of Charge: unit target, long range)
> Charges at an enemy from far away, gaining speed as he goes and shoving aside anything in his path. On arrival he bashes and stuns the target.

- Range 2000/2400/2800/3200. Bash 40/60/80/100 + 50% move speed (Speed),
  1.2 s stun. Slows can't stop the charge; a stun or root does.
- **Mastery:** allies near him when the charge starts gain 20% speed for
  4 s.
- Inspired by Spirit Breaker. Engine: unit target.

**CAL-E Hibernate** (TWEAK)
> Settles for 1.5 seconds to heal a quarter of missing health, and nothing can push him. Press again to wake early with a roar that taunts nearby enemies.

- Taunt 300 radius, 1 s.
- **Mastery:** heals 35% of missing health. Engine: recast.

**CAL-R Callisto's Roar** (REWORK of Shockwave)
> A roar down a wide line knocks enemies back and stuns them. Anyone slammed into a wall or tree is stunned twice as long.

- 140/210/280; 1 s stun, 2 s if slammed into terrain.
- **Mastery:** the roar fans into a 70-degree cone.
- OSRS: Callisto's knockback roar. Engine: terrain hits.

### Cerberus — zone tank

Strong: zone control, mana denial, teamfight fear. Weak: slow (390),
melee, souls are skillshots. Builds: **Bulk**; **Spell** for lava and
souls.

**CER-P Three Heads** (KEEP)
> Every third attack bites everything around.

**CER-Q Triple Bite** (KEEP, add mastery)
> Three quick bites in front; the last one slows.

- **Mastery:** the third bite stuns for 0.6 s instead.

**CER-W Lava Pools** (TWEAK: vector)
> Spits three pools of lava along the line you draw. Enemies in them burn and slow; Cerberus heals while he stands in one.

- Heals 2% max health per tick (Bulk).
- **Mastery:** pools erupt for 120 (Spell) when they expire.
- Engine: vector targeting.

**CER-E Summoned Souls** (TWEAK: three different souls)
> A ranger, mage and warrior soul fly out in a fan. The ranger soul slows, the mage soul drains mana and the warrior soul stuns.

- 30% slow / 30 mana / 0.5 s stun; each enemy is hit once by each soul.
- **Mastery:** souls turn at the end and fly back through enemies.
- Assets: Summoned Soul NPCs 5867/5868/5869 (models 29267/29268/29266)
  flown as model projectiles (the `model` look field the anchor uses).

**CER-R Hellhound Howl** (KEEP, longer fear per rank, add mastery)
> Every nearby enemy flees in fear.

- Fear 1.6/1.9/2.2 s (today 1.6 s at every rank).
- **Mastery:** feared enemies take 20% more damage (Mark).

### Chaos Elemental — disruptor

Strong: shuts down item-heavy carries, mana denial, chaos. Weak: random
results, squishy (520 health, 20 armour), skillshots. Builds: **Mana**;
**Spell**.

**CHA-P Pure Chaos** (KEEP)
> Every third ability hit also lands a random slow, venom or mana drain.

**CHA-Q Discord** (TWEAK)
> A swirling bolt that burns the target's mana and deals that much again as damage.

- 80/110/140/170 (Spell), plus mana burned 30/45/60/75.
- **Mastery:** a target with no mana left takes 50% more.

**CHA-W Confusion** (KEEP, add mastery)
> A burst at the cursor throws every enemy in it somewhere nearby.

- **Mastery:** thrown enemies are silenced for 1 s when they land.

**CHA-E Madness** (REWORK: locks gear)
> A red bolt that throws its victim's gear into chaos. For a few seconds its items give no damage, ability power or speed, and it cannot attack.

- 2/2.5/3/3.5 s gear-lock and disarm.
- **Mastery:** also slows 30%.
- OSRS: Madness unequips your items. It counters item builds directly.
  Engine: gear-lock.

**CHA-R Chaos Storm** (KEEP, add mastery)
> Three fans of bolts, each carrying Discord, Confusion or Madness at random.

- **Mastery:** a fourth fan, and the bolts pierce.

### General Graardor — war commander

Strong: initiation, taunts, pushing with bodyguards, snowballing duels.
Weak: slow, no escape, a lost duel hurts. Builds: **Bulk**; **Attack** for
duels.

**GRA-P Warchief** (KEEP, renamed from Bandos' War Cry so it isn't confused with his E)
> Nearby allied minions deal 15% more damage.

**GRA-Q Smash** (KEEP, add mastery)
> Slams the ground in a cone, damaging and slowing enemies.

- **Mastery:** enemies at the tip of the cone are knocked up for 0.5 s.

**GRA-W Bodyguards** (TWEAK)
> Strongstack, Steelwill and Grimspike fight for him for 8 seconds: Strongstack taunts as he arrives, Steelwill's spells slow and Grimspike shoots from range.

- **Mastery:** they stay 12 seconds.
- Assets: NPCs 2216/2217/2218 (verified).

**GRA-E War Cry** (KEEP, add mastery)
> Takes 30% less damage for 4 seconds and taunts nearby enemies.

- **Mastery:** taunt lasts 1.75 s.

**GRA-R Bandos' Challenge** (REWORK of Bandos' Wrath)
> Leaps onto an enemy hero and challenges it. For a few seconds the two can only attack each other. If Graardor wins, he keeps extra attack damage for the rest of the match.

- Range 900; 3/3.75/4.5 s; +8 attack damage per duel won.
- **Mastery:** Graardor takes 25% less damage during the duel.
- Inspired by Legion Commander. Engine: unit target, duel.

### JalRek-Jad — telegraphed titan

Strong: huge telegraphed hits, healer sustain, siege. Weak: no mobility,
dodgeable telegraphs, healers can be killed. Builds: **Spell**; **Bulk**.

**JAD-P Jad's Rhythm** (NEW innate)
> Every fourth attack is a Jad attack. He rears up, then either breathes fire at his target or calls a boulder down on where it stood, alternating.

- 0.8 s wind-up; 60 + 12 per hero level (Spell). The fire hits a line; the
  boulder lands 1 s after the wind-up, so moving dodges it.
- OSRS: the TzTok-Jad telegraph players pray against. Assets: Jad's attack
  clips from NPC 3127 *(pick)*.

**JAD-Q Magma Breath** (NEW, replaces Jad's Guard)
> Breathes a line of fire that burns everything it touches.

- 900 long, 160 wide; 70/100/130/160 + a 3 s burn (Spell).
- **Mastery:** leaves burning ground for 3 s.

**JAD-W Boulder Rain** (NEW, replaces Teleport)
> Marks a circle. A moment later a boulder crashes down, damaging everything inside and stunning anyone at its centre.

- Range 1100, radius 250, 1.2 s delay; 90/130/170/210; 1 s stun.
- **Mastery:** three smaller boulders scatter around the impact.

**JAD-E Yt-HurKot Healers** (NEW, replaces Miner's Endurance)
> Calls healers from the Fight Caves for 10 seconds. They mend Jad until enemies kill them.

- 2 healers (3 from rank 3); each heals 2% max health per second (Bulk).
- **Mastery:** the healers also mend nearby allied heroes.
- Assets: Yt-HurKot NPC 3128 (verified). Engine: healer summon behaviour.

**JAD-R Inferno Unleashed** (NEW, replaces Earth Splitter)
> For several seconds every second attack is a Jad attack, with a shorter wind-up.

- 6/7/8 s; 0.5 s wind-up.
- **Mastery:** each Jad attack fires both styles at once.
- Alternative: keep Earth Splitter as R if you prefer it.

### Kalphite Queen — burrowing tank

Strong: line stun initiation, two lives, reflect. Weak: predictable second
form, slow, no ranged damage. Builds: **Bulk**; **Spell**.

**KQ-P Second Form** (KEEP)
> At low health she sheds her shell, heals and returns in her winged form.

**KQ-Q Mandible Crush** (KEEP, add mastery)
> A crushing bite in front that stuns.

- **Mastery:** heals her for 30% of the damage dealt.

**KQ-W Undermine** (REWORK of Burrow)
> Burrows to the cursor, knocking up and damaging every enemy along the path. She cannot be hit while underground.

- 750-900; knock-up 0.6/0.7/0.8/0.9 s.
- **Mastery:** leaves an acid trail that slows by 25% for 3 s.
- Inspired by Sand King.

**KQ-E Spiked Carapace** (REWORK of Chitin)
> For a moment, the first hit from each enemy is reflected back and stuns the attacker.

- 1.5 s window; 1 s stun, once per attacker.
- **Mastery:** 2 s window, reflects double.
- Engine: per-attacker reflect.

**KQ-R Kalphite Swarm** (KEEP, renamed from Swarm, add mastery)
> Kalphite workers burst out and charge in a line.

- **Mastery:** soldiers replace the workers and knock enemies back farther.
- Assets: workers NPC 955 (in use), soldiers NPC 138.

### Kolodion — god-caster

Strong: burst combos, spell variety. Weak: 440 health, immobile, delayed
claws. Builds: **Spell**; **Mana**.

**KOL-P Shapeshifter** (KEEP)
> Casting different spells in a row stacks bonus spell damage.

**KOL-Q Claws of Guthix** (KEEP, add mastery)
> Delayed claws rip the cursor area and shred armour.

- **Mastery:** the shred stacks twice.

**KOL-W Flames of Zamorak** (KEEP, add mastery)
> A pillar of fire bursts under the cursor.

- **Mastery:** the fire lingers for 4 s.

**KOL-E Saradomin Strike** (KEEP, add mastery)
> A fast bolt of holy light.

- **Mastery:** it bounces to a second enemy within 400.

**KOL-R Shapeshift** (REWORK: four forms)
> Each cast takes the next of his forms for 8 seconds: Jogre (more health, cleaving blows), Ice Spider (faster, attacks chill), Ghost (attacks pass through him) and Demon (fiery attacks that splash).

- **Mastery:** forms last 10 s.
- OSRS: the order he shifts through in the Mage Arena fight. Assets:
  Kolodion's form NPCs 1606 jogre (it uses the Jogre's own animations),
  1607 ice spider, 1608 ghost, 1609 demon (in use today). Engine: per-form
  stats (transform exists).

### Kree'arra Jr. — wind marksman

Strong: mobility, sustained damage with Feather Flurry, knockback poke. Weak:
420 health, single-target ultimate. Builds: **Attack** (bows, crossbows);
**Speed**.

**KRE-P Tailwind** (NEW innate)
> After a leap or Glide, the next three attacks fire an extra feather at a second enemy.

- The feather deals 40% + 2% per hero level.

**KRE-Q Gale Shot** (NEW, replaces Arcane Lance)
> Hold to draw the bow, release to loose a piercing gust that knocks enemies back. The longer the draw, the farther and harder it hits.

- Up to 1 s draw; 1100-1600 long; 60-180 + 90% attack damage (Attack);
  150 knockback.
- **Mastery:** enemies knocked into walls are stunned for 1 s.
- Inspired by Powershot. Engine: hold-to-charge.

**KRE-W Kree's Leap** (KEEP, add mastery)
> Kree leaps a long way across the ground.

- **Mastery:** two charges.

**KRE-E Glide** (NEW, replaces Flechtalon Guard)
> Kree glides on the wind: faster, and attacks aimed at him miss.

- 3/3.5/4/4.5 s; +30% speed.
- **Mastery:** enemies near him are slowed 25%.
- Engine: evasion.

**KRE-R Feather Flurry** (NEW, replaces Skyfire)
> Locks onto an enemy for 10 seconds. Kree fires feathers at it much faster, each at reduced damage.

- +200% attack speed against the target; 60/70/80% damage per shot.
- **Mastery:** each shot pushes the target back a little.
- Engine: unit target.

### K'ril Tsutsaroth — infernal executioner

Strong: venom pressure, an execute ultimate, sticky. Weak: little hard
crowd control, predictable charge. Builds: **Attack**; **Bulk**.

**KRL-P Demonic Poison** (KEEP)
> Attacks venom the target.

**KRL-Q Hellfire** (KEEP, renamed from Flames of Zamorak, add mastery)
> A line of hellfire that scorches everything it passes.

- **Mastery:** the line keeps burning for 2 s.

**KRL-W Demonic Charge** (KEEP, add mastery)
> Charges forward, damaging enemies in the way.

- **Mastery:** hitting a hero under 30% health resets the cooldown.

**KRL-E Brimstone Blade** (NEW autocast, replaces Infernal Shield)
> Autocast: each attack sets the target burning for a share of its maximum health. The first hit also stuns briefly.

- Burns 3% max health over 2 s; 0.4 s stun on the first hit; 15 mana per
  hit.
- **Mastery:** the burn deals 4.5%.
- Inspired by Doom. Engine: autocast.

**KRL-R YARRRRRRR!** (KEEP, add mastery)
> A crushing blow for 30% of the target's current health; their healing is cut.

- **Mastery:** a kill refunds half the cooldown.

### Blowpipe Ranger — venom sniper

Strong: range, venom stacking, an execute ultimate. Weak: 420 health, 16
armour, only one escape. Builds: **Attack** (blowpipe, twisted bow);
**Speed**.

**RNG-P Toxic Build-up** (NEW innate)
> Attacks and darts stack venom up to three times. At three stacks the target heals 30% less.

- Venom 4 + 0.5 per hero level per stack per second. Engine: venom
  stacks.

**RNG-Q Toxic Dart** (KEEP, add mastery)
> A fast poisoned dart that leaves the target venomed.

- **Mastery:** the dart pierces its first target.

**RNG-W Dart Volley** (KEEP, add mastery)
> A fan of five darts that poisons every enemy it hits.

- **Mastery:** seven darts.

**RNG-E Ranger's Footwork** (KEEP, add mastery)
> The ranger hops backwards and attacks faster for a few seconds.

- **Mastery:** the hop leaves caltrops that slow by 30% for 2 s.

**RNG-R Deadshot** (KEEP, add mastery)
> A piercing blowpipe shot that hits everything in its line and finishes off wounded targets.

- **Mastery:** applies three venom stacks.

### Venenatis — web stalker

Strong: web map control, burst, a brood to push with. Weak: squishy (520),
leans on her webs. Builds: **Attack**; **Speed**.

**VEN-P Venom Fangs** (KEEP)
> Attacks venom the target.

**VEN-Q Web** (KEEP, add mastery)
> Spits a web that roots the first enemy hit.

- **Mastery:** the web splits to a second enemy behind the first.

**VEN-W Pounce** (KEEP, add mastery)
> Leaps onto the cursor and bites everything she lands on.

- **Mastery:** landing on a rooted enemy halves the cooldown.

**VEN-E Web Nest** (NEW, replaces Spiderlings)
> Lays a web that lasts 40 seconds. Inside it she moves faster, passes through scenery and is hidden from enemies outside the web while she isn't attacking.

- Two charges; radius 550; +20% speed.
- **Mastery:** three charges, and webs slow enemies 15%.
- Inspired by Broodmother. Engine: web zones; hiding reuses the brush
  logic.

**VEN-R Brood Frenzy** (NEW, replaces Toxic Bloom)
> Spiderlings hatch to hunt for 8 seconds, and Venenatis feeds: bonus damage and lifesteal.

- 4/6/8 spiderlings; +20 damage; 30% lifesteal.
- **Mastery:** spiderlings burst into venom when they die.
- Assets: Venenatis spiderling NPC 495 (in use).

### Vorkath — siege dragon

Strong: long-range zone, sieging. Weak: slow (395), 480 health, the
ultimate can be interrupted. Builds: **Spell**; **Attack** (splash).

**VOR-P Dragonfire Splash** (KEEP)
> Attacks splash around their target.

**VOR-Q Dragonfire Bomb** (KEEP, add mastery)
> A lobbed fireball that bursts on landing.

- **Mastery:** leaves fire on the ground for 2 s.

**VOR-W Acid Spray** (TWEAK: vector)
> Sprays a line of acid pools along the line you draw.

- **Mastery:** six pools instead of four.
- Engine: vector targeting.

**VOR-E Zombified Spawn** (TWEAK)
> Releases a spawn that freezes the first enemy it reaches. Killed, it bursts and freezes everyone near it.

- **Mastery:** freezes last 2 s.
- OSRS: Crumble Undead on the spawn. Assets: NPC 8063 (in use).

**VOR-R Rapidfire** (KEEP, add mastery)
> Channels a 2 second barrage of fireballs.

- **Mastery:** the barrage follows the cursor while channelling.

### Wise Old Man — trickster support

Strong: displacement, saves, utility. Weak: low damage, 460 health.
Builds: **Spell**; **Mana**.

**WOM-P Old Wisdom** (KEEP)
> Earns 20% more experience.

**WOM-Q Telekinetic Grab** (REWORK: lift and throw instead of a pull)
> Lifts an enemy into the air, stunned. Press again to choose where it lands; the landing stuns enemies around it.

- Range 700; lift 1.2/1.4/1.6/1.8 s; drop up to 375 away; landing stun
  0.5 s.
- **Mastery:** the drop range is 500.
- Inspired by Rubick. Engine: unit target, recast.

**WOM-W Teleport Other** (KEEP, add mastery)
> Hurls nearby enemies away from him.

- **Mastery:** hurled enemies are slowed by 30% for 2 s.

**WOM-E Ward of Ages** (REWORK: ally target)
> Wards an ally or himself: a shield that also shakes off slows.

- 130/160/190/220 shield (Spell).
- **Mastery:** the ward adds 20% speed.
- Engine: ally target.

**WOM-R** (choose one)

- **A. Party Room Balloons** (KEEP): balloons pop across an area; enemies
  are stunned and he is healed.
  **Mastery:** popped balloons drop party hats that heal allies.
- **B. Draynor Heist** (NEW):
  > Robs the last ability an enemy hero cast and keeps it for 30 seconds to use himself.

  He did rob the Draynor bank. Needs `SYS-9` first.

### Commander Zilyana — holy duelist

Strong: mobility, heals, reset kills. Weak: 26 armour, relies on resets.
Builds: **Attack**; **Speed**.

**ZIL-P Swift Blessing** (KEEP)
> Moves 10% faster, and her dashes go farther.

**ZIL-Q Holy Lightning** (KEEP, renamed from Saradomin Strike, add mastery)
> Lightning strikes the cursor after a short delay.

- **Mastery:** it strikes twice, 0.4 s apart.

**ZIL-W Blessed Dash** (KEEP, add mastery)
> A quick dash; holds two charges.

- **Mastery:** three charges.

**ZIL-E Starlight** (TWEAK: heals allies too)
> Heals her and nearby allies.

- 500 radius.
- **Mastery:** also removes slows. Engine: heal allied heroes in an area
  (today's buff heals only her).

**ZIL-R Healing Blade** (TWEAK of Godsword Special: heals)
> Her next hit deals huge damage and heals her for half of it. A kill resets Blessed Dash.

- 150/225/300 + 15% of missing health.
- **Mastery:** assists reset Blessed Dash too.
- OSRS: Healing Blade is the Saradomin godsword's special attack; it heals
  for half the damage dealt.

## 5. Build order once approved

1. **Engine pieces most heroes need:** unit and ally targeting, the new
   statuses, scaling tags, masteries, innates. One commit each, each with
   tests.
2. **Heroes in batches of two or three,** in the order you approve them.
   Each is checked with `--fixed-dt` screenshots and a `--balance-report`
   pass.
3. **Vector, charge, toggle, autocast, recast,** as the heroes that need
   them come up.
4. **`SYS-9` bots on real kits.**

## Sources

- Valve, Abilities Data Driven:
  https://developer.valvesoftware.com/wiki/Dota_2_Workshop_Tools/Scripting/Abilities_Data_Driven
- Liquipedia: https://liquipedia.net/dota2/Target_Unit ·
  https://liquipedia.net/dota2/No_Target ·
  https://liquipedia.net/dota2/Area-of-effect ·
  https://liquipedia.net/dota2/Physical ·
  https://liquipedia.net/dota2/Magic_immunity ·
  https://liquipedia.net/dota2/Strong_Dispel ·
  https://liquipedia.net/dota2/Aghanim%27s_Shard ·
  https://liquipedia.net/dota2/Version_7.42 ·
  https://www.liquipedia.net/dota2/7.38
- Vector targeting library: https://github.com/erratic-pattern/vector_target
- Dota 2 7.00 gameplay (talents, ultimate levels): https://www.dota2.com/700/gameplay
- 7.41 Facets removed: https://www.pcgamesn.com/dota-2/7-41-patch-notes
- 7.41 innates: https://escorenews.com/en/dota-2/article/76937-all-new-and-changed-innate-abilities-of-dota-2-heroes-in-patch-7-41
- Kolodion's Mage Arena forms (OSRS Wiki): https://oldschool.runescape.wiki/w/Kolodion
