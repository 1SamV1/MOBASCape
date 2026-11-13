# Movement

Status: **design proposed; locomotion implemented for the player's champion.**

Built so far (`crates/sim/src/motion.rs`, `crates/client/src/movement.rs`):
turn time, acceleration time, stopping time, the slow-while-turning rule, the
30 Hz integer tick with drawing between ticks, and a walk cycle driven by
distance. An attack now waits until the champion has turned to face its
target.

Not built yet: attack wind-up, live reload of champion files, the `--lab`
mode, recording and replay, and any of this for minions.

Moving a champion is what a player does every second of a match. If it does
not feel good with nothing else on screen, no later system will fix it. This
document says what "feels good" is made of, where the values live, and what
has to be true before movement counts as done.

The target is **weight**: champions are large creatures that take time to
turn, to get going and to swing. Each champion should feel different to move.

## Where movement stands today

- A unit moves at `speed * dt` along its path (`follow_path` in
  `crates/client/src/animated.rs`). It is at full speed on the first frame and
  stops dead on the last.
- Facing snaps to the direction of travel in the same frame
  (`unit.yaw = dir.x.atan2(dir.z)`). There is no turning.
- The walk animation advances with wall-clock time, not with distance
  covered, so feet slide whenever speed changes.
- A champion file (`data/champions/*.json`) carries one movement value,
  `speed`. Jad at 380, the monkey at 420 and Kree at 450 all move the same
  way, at slightly different speeds.
- Movement runs on the render loop in floating point, so it is not the same
  at 30 fps and 144 fps, and cannot be replayed.

## What weight is made of

Six settings. Each is a property of the champion, not of the engine.

| Setting | What it does | What the player feels |
|---|---|---|
| Turn time | Seconds to turn 180 degrees | Mass. The single biggest lever. |
| Acceleration time | Seconds from standing to full speed | Effort to get going |
| Stopping time | Seconds from full speed to standing | Momentum |
| Move speed | World units per second at full speed | Pace |
| Attack wind-up | Seconds from attack start to the hit | Commitment |
| Stride length | World units covered per walk cycle | Feet that grip the ground |

Two rules tie them together:

- **A champion moves while it turns, but slower the further it is from facing
  its direction of travel.** Speed is scaled by how well facing and direction
  agree, with a floor so a click always produces motion at once. A heavy
  champion asked to reverse therefore swings round in an arc; a light one
  pivots.
- **The walk animation is driven by distance, not time.** One cycle plays per
  stride length covered. Accelerating, slowed and hasted champions all keep
  their feet planted.

## Per-champion data

New fields in `data/champions/<name>.json`, beside the existing `speed` and
`attack_interval`:

```json
{
  "speed": 350,
  "turn_time": 0.45,
  "accel_time": 0.30,
  "stop_time": 0.20,
  "attack_windup": 0.50,
  "stride": 240
}
```

Starting values for the three champions that exist. These are places to begin
tuning, not measurements, and they are meant to be changed by playing.

| | monkey | jad | kree |
|---|---|---|---|
| Role in the set | light | heavy | fast flyer |
| `speed` | 420 (as now) | 350 | 450 (as now) |
| `turn_time` | 0.15 | 0.45 | 0.25 |
| `accel_time` | 0.10 | 0.30 | 0.20 |
| `stop_time` | 0.05 | 0.20 | 0.25 |
| `attack_windup` | 0.25 | 0.50 | 0.30 |
| `stride` | default | default | default |

Left out, `stride` defaults to top speed times the length of the walk cycle,
which plays the walk at its authored rate at top speed. Set it only where a
champion's feet visibly slide.

The point of three is contrast. If two of them feel alike, one is wrong.

## Rules for the implementation

1. **Movement runs in the simulation tick,** at 30 Hz, in integer or
   fixed-point units, from commands. This is step 1 of the path in
   SPECTATING.md; it is the same piece of work.
2. **One movement function serves tap-to-move and hold-to-steer** (ADR-005).
   They differ in where the desired direction comes from, not in how the
   champion responds to it.
3. **The first response is immediate.** On the tick a command arrives, facing
   starts to turn and the champion starts to move. Weight comes from how long
   the response takes to complete, never from a delay before it begins.
4. **The client draws between ticks.** Position and facing are interpolated,
   so 30 Hz simulation looks smooth at any frame rate.
5. **Modifiers multiply, they do not replace.** Buffs, slows and items scale
   `speed`; they never write over a champion's own values.
6. **No value is typed into code.** Every number above lives in the champion
   file and reloads while the game runs.

## The tick rate is not baked in

The simulation runs at 30 ticks a second today (ADR-009). That number is one
constant, `TICK_HZ` in `crates/sim/src/motion.rs`. Every value a champion
carries is in seconds or in units per second and is converted there, so the
rate can be changed without touching the movement model or any champion file.

Measured, the same heavy champion (speed 350, turn 0.45 s, acceleration
0.30 s, stop 0.20 s) at four rates:

| Rate | To top speed | Half turn | Covered in 2 s | Carried through a reversal |
|---|---|---|---|---|
| 20 Hz | 0.35 s | 0.50 s | 656 | 21.8 |
| 30 Hz | 0.33 s | 0.47 s | 653 | 24.5 |
| 60 Hz | 0.32 s | 0.47 s | 650 | 27.4 |
| 120 Hz | 0.32 s | 0.46 s | 646 | 28.7 |

Timings agree to within a tick and distances to within 2%. The arc of a
reversal shifts by a few world units, because at a lower rate the champion
comes round in bigger steps. So a change of rate needs a light retune of turn
times, not a rewrite. `a_champion_moves_the_same_whatever_rate_the_simulation_runs_at`
in `crates/sim/tests/motion.rs` holds these bounds.

The rate itself is still an open decision. Thirty is the working value.

## The movement lab

A way to tune movement with nothing else in the way.

- `--lab`: a flat, empty area, one champion, no minions, towers, fog or HUD.
- The champion file reloads when it is saved.
- A readout shows current speed, facing error and the six values.
- Keys switch between the champions, so contrast can be judged in seconds.

Five drills, each done with every champion:

1. **Reverse.** Run, then click directly behind. Watch the arc.
2. **Zig-zag.** Click left and right of the path in quick succession.
3. **Stop and go.** Short clicks with pauses. Watch the start and the stop.
4. **Orbit.** Hold-to-steer in a circle round a point.
5. **Strike.** Walk up to a target dummy and attack; walk away mid-swing.

## Done means

- [ ] All six values are read from the champion file and reload live.
- [ ] Movement runs in the simulation tick and is identical at any frame rate.
- [ ] A recorded movement session replays to the same state hash.
- [ ] Feet do not slide: distance per walk cycle is within 10% of `stride` at
      full speed, half speed and while accelerating.
- [ ] The three champions are told apart by movement alone, with models
      hidden, by someone who was not told which is which.
- [ ] The five drills have been played with each champion and the values
      written back to the champion files.
- [ ] The person who owns game feel has played it and signed off.

Until every box is ticked, attacks, abilities and items are not built on top
of the new movement.

## Not in this document

Unit-to-unit collision, dashes and blinks, knockbacks, terrain speed
modifiers, and camera feel. Each builds on this and gets its own design.

## Open questions

- Whether a champion should stop completely to make a very sharp turn. The
  speed-scaling rule above says no; playing the heavy champion will show
  whether that is right.
- Whether attack wind-up can be cancelled by a move command, and until when.
- Flyers: whether `kree` should ignore stopping time in a way that reads as
  gliding.
