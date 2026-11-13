# Spectating, replays, and the connected world

Status: **proposed design. Nothing in this document is implemented.**

It records decisions that are cheap to make now and expensive to reverse
later, so that match code written today does not have to be rewritten when
spectating arrives.

## The idea

The MOBA is a place in a persistent world. A player walks into the MobaScape
Arena, reads the boards, picks a live match, climbs into the stands and
watches the real match below. It is not a video and not a second simulation:
it is the same authoritative match, seen from another camera.

On top of that sit teams and guilds, tournaments, schedules and live results.

## Order of work

1. **MobaScape first.** A complete playable match.
2. **Spectating a match,** as a read-only connection. This is the feature this
   document makes room for.
3. **The persistent first-person world,** which will contain the arena.
   Accounted for in the architecture; not built until the MOBA plays.
4. **The extraction RPG,** later, on the same foundation.

Three games are never in progress at once.

## Where the code stands today

This section is here so the rules below are read against the real code.

- `crates/sim` holds the tile grid and A* pathfinding. It is integer-only.
- Everything else in a match (units, minions, towers, projectiles, abilities,
  buffs, XP and gold, fog of war) lives in `crates/client/src/animated.rs`,
  in floating point, advanced by the render loop's frame time.
- There is no server, no protocol crate, no state hash and no recording.

None of that is wrong for a prototype, and ADR-016 already says the fog must
move to the server. But spectating, replays and multiplayer all need the same
thing: **the match has to run somewhere other than the client that draws it.**
Each milestone that adds match logic to the client adds to what must move.
STATUS.md already lists "Deterministic 30 Hz sim tick in `crates/sim` + state
hash" as a next task, and ROADMAP M3 lists a deterministic 10-bot match with a
state hash. This document asks for that step to come before more match
systems are added, not after.

## Rules

### 1. One authoritative match

A match is one simulation instance on a match server. Players and spectators
observe that one instance. There is never a second copy that could disagree.

### 2. A spectator is a role, not a player

Every connection to a match has a role:

| Role | Sends commands | Sees |
|---|---|---|
| `Player` | yes, for its own champion | what its team can see |
| `Spectator` | never | everything, subject to the delay in rule 4 |

`crates/proto` carries the role from its first commit. No message format may
assume that everyone connected is one of the ten players. A spectator's
commands are rejected by the server, not merely unsent by the client.

### 3. Visibility is a filter, not part of the state

The simulation holds the whole truth. What a connection receives is that truth
passed through a filter chosen by its role and team:

- `Player`: entities its team can see, and nothing about the rest.
- `Spectator`: everything.

So the visibility grid is computed by the simulation for each team, and fog is
never baked into match state or into the snapshot format. NETCODE.md already
says the server never sends a team what it cannot see; this generalises it to
"the server sends each recipient what its filter allows".

### 4. Spectator delay is set per competition

A spectator who sees everything can tell a player where the enemy is. The
defence is a delay: spectators receive the match a fixed time behind.

The delay is a property of the match, set when it is created:

| Competition | Delay |
|---|---|
| Casual | none (live) |
| Ranked, tournament | configured, e.g. 60-180 s |

A delayed match is still the actual match. The delay is applied by the
broadcast relay (rule 6), never by the simulation.

### 5. Every match is recorded, and replay is a test

A recording is the commands that took effect, the tick each was applied on,
and the state hash after every tick. Replaying feeds those commands to the
starting state and checks every hash. A replay either reproduces the match
exactly or reports the first tick that disagreed.

Server authority does not make replays work. Determinism does, and determinism
is only as real as its test. So, once the simulation tick exists:

- the match loop runs through a recorder;
- a recorded match that replays to the same hashes is part of the test suite;
- every system added to the simulation gets a recorded match that covers it;
- every piece of simulation state is included in the state hash, or a
  divergence in it goes unseen.

A recording plus the map and content versions it was made with is a complete
replay. Tournament archives and "watch again" are built from it.

### 6. Spectators never connect to the match server

```
match server (sim) ──► broadcast relay ──► spectators
        ▲                    │
     players             delay buffer, fan-out
```

The match server sends one full-information stream to a relay. The relay holds
the delay buffer and serves any number of spectators. Five hundred people in
the stands cost the match nothing.

### 7. The renderer can draw a match inside another scene

The arena is part of the world; the match is drawn in its pit from the
spectator stream. So the renderer takes a scene as a value with a transform,
not as a global, and the code that turns match state into drawn units is the
same for a player's client and a spectator's.

### 8. Identity and results live outside both games

Accounts, teams, guilds, schedules and results belong to a service of their
own. Match servers report results to it; the arena's boards read from it.

## A path from today's code

Each step ends with something that runs and a test that proves it.

1. **A simulation tick in `crates/sim`.** Fixed 30 Hz, integer or fixed-point
   positions, seeded RNG, commands in, state out. Start with movement, which
   already paths on the sim's grid.
2. **A state hash and a recorder,** with a replay test, while the simulation
   is still small. This is the cheapest moment to add them.
3. **Move match systems across one at a time:** minions, towers and
   projectiles, abilities and buffs, XP and gold, visibility. The client keeps
   animation, effects and the HUD, and draws what the simulation reports.
   Each move is covered by a recorded match.
4. **`crates/proto` with `Role`,** and snapshots built from full state and
   filtered per recipient.
5. **A server that owns the simulation,** with one player connected.
6. **A spectator connection:** read-only, full state, optional delay. At this
   point a second window can watch a live match.
7. **The relay,** when more than a handful of spectators is needed.
8. **The arena,** when there is a persistent world to put it in.

Step 6 is the first point at which "watch a live match" works. Steps 1 to 3
are the bulk of the work and are needed for multiplayer in any case.

## Not decided

- Whether spectating uses snapshots or re-simulates from the command stream.
  Snapshots are simpler; the command stream is far smaller. Start with
  snapshots.
- The delay for each competition type.
- Integer world units or fixed-point for simulation positions.
- How the persistent world is hosted and what it is built from. See below.

## A note on assets and the persistent world

AGENTS.md states that this project is not a RuneScape private server. A
persistent RuneScape-style world built on the Jagex cache is much closer to
one than a standalone MOBA is. That is a decision for the project owners, with
the distribution question in ASSET_PIPELINE.md still open. It does not affect
the MOBA work.
