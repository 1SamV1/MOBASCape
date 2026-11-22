# Engine road map

Last updated: 2026-11-21, after the 1080p 60 fps pass (albedo simplification,
dynamic-unit culling, colour LUTs and automatic scene resolution).

This is the working performance/architecture backlog. It records what the
engine does today, what was measured, and the next changes that should give
the largest wins per unit of risk.

## 1. Current baseline

- Target: 1080p/60 on the AMD 680M iGPU; current captures are 1280×720 and
  1920×1008.
- Map: 5×5 Karamja block, 196 terrain chunks (204,800 triangles), 169 model
  chunks (673,789 triangles), 27 water chunks, 2,690 placements, 157 animated
  units.
- Assets: 208 cache-native model textures in a 256-layer array, 6 ground
  texture layers, 4× MSAA HDR scene target, 1024px sun shadow map.
- Simulation: click-to-move, A* pathing, hybrid dynamic-unit meshes, three
  lanes, minions, camps, towers and inhibitors.
- Renderer: one offscreen HDR target, bloom bright/blur/final passes, FXAA +
  grade + dither post, HUD after post. `--render-scale auto` targets 1.2M
  scene pixels; post and HUD stay at native window resolution.

### Measured (release, `--present-mode immediate`, 1280×720)

| Configuration (release, immediate) | Avg | p50 | p95 | approx FPS |
|---|---:|---:|---:|---:|
| Shadows + 4× MSAA | 11.19 ms | 13.24 ms | 14.89 ms | 89 |
| Shadows off + 4× MSAA | 10.94 ms | 12.93 ms | 14.20 ms | 91 |
| Shadows + 1× MSAA | 10.60 ms | 12.17 ms | 13.88 ms | 94 |
| Dev FIFO before chunk culling | — | — | — | 40 |
| Dev FIFO after chunk culling | — | — | — | 60 (vsync) |

The averages include fast frames where the app is idle; p50/p95 are the useful
numbers. Background applications and compositor scheduling add noise, so run
the benchmark with a clean desktop before using these as hard budgets.

Interpretation: static vertex load was the biggest single bottleneck and chunk
culling recovered most of it. The shadow map is now the next large cost
(roughly 5 ms at these settings), followed by 4× MSAA (roughly 1.5 ms) and the
terrain fragment shader. The current numbers are a dev-laptop measurement, not
a certified benchmark; use the harness described below before treating them as
budgets.

## 2. What M5.22 changed

### Static chunk culling

`ChunkedMesh` splits terrain, static models, water, blob shadows and static
shadow casters into world-space 16-tile AABB chunks at upload time. Triangles
that span multiple chunks (the distant backdrop) go into an always-visible
overlay bucket. Each frame the renderer extracts six frustum planes from the
camera view-projection and submits only intersecting chunks.

The default chunk size is now **24 tiles**. Benchmarked at the same camera:
196 terrain + 112 model + 27 water chunks, p50 **11.98 ms** / p95 **13.62 ms**,
versus 400/228/49 chunks at 16 tiles, p50 12.49 ms / p95 14.17 ms. Use
`--chunk-tiles` to retune the culling/draw-call balance.

Current chunks/triangles:

| Mesh | Chunks | Triangles |
|---|---:|---:|
| Terrain | 400 | 204,800 |
| Static models | 212 | 483,606 |
| Water | 49 | 9,712 |
| Blob shadows | 173 | 3,662 |
| Static shadow casters | 170 | 413,513 |

### Shadow proxies and benchmark harness

The full static caster mesh was 413,513 triangles. `append_model_caster` now
emits a two-box trunk/canopy proxy for trees and a single AABB box for
buildings and ruins (38,028 triangles total). The camera-relative shadow pass
now costs roughly 0.3 ms p50 at the benchmarked view.

`--benchmark <seconds>` records frame times after a one-second warm-up and
prints avg/p50/p95/p99 plus approximate FPS. Combined with
`--present-mode immediate` and `--msaa 1|4`, this is the first P0 harness.

### Other recent engine work

- 4× MSAA on the HDR target with a single-sample resolve into post/bloom.
- Cache sprite (index 8) and texture-definition (index 9) decoders.
- Cache-native model textures with per-face planar UVs, alpha cutouts for
  leaves, and a bark-only path for dead/snowy trees.
- HD river surface with animated ripples, fresnel, sun specular and fog.
- Static scenery casters in the sun shadow map.
- `--present-mode fifo|immediate|mailbox` for uncapped profiling.
- `--msaa 1|4` for quality/performance comparisons.
- Water is walkable; structure footprints are automatically moved off lane
  centrelines; lane rendering is suppressed under water.

## 2b. 2026-11-21 — 1080p 60 fps pass

Release, immediate present, MSAA 4, sun shadows on, Ryzen 5 6600U / 680M:

| Window | Scene | p50 | p95 | fps |
|---|---|---:|---:|---:|
| 1280×720 | 1280×720 (1.00×) | **10.62 ms** | 11.76 ms | 93.9 |
| 1920×1008 | 1461×767 (0.76×) | **14.13 ms** | 15.86 ms | 70.2 |

The same 1920×1008 run before this pass measured p50 61.5 ms. Native
1920×1008 with `--msaa 1 --render-scale 1.0` is p50 17.3 ms; the auto scene
scale is what produces the 60 fps margin on this iGPU.

What moved the number:

- **Terrain albedo**: removed the per-fragment multi-octave macro noise,
  meadow/hollow/thicket/patch/noise warp and speckle stack. The cache grass
  sprites, biome vertex tint and classification remain. Fog banks and grass
  normal perturbation were removed for the same reason. The sun shadow lookup
  is now two taps instead of four.
- **Animated mesh culling**: units farther than 9,000 units from the camera
  focus skip animation, mesh and blob-shadow construction. At the default
  camera this leaves roughly 18 units instead of 157.
- **Mesh CPU**: face palette RGB is cached per model instead of converted
  from HSL every frame, and sRGB byte colours use a 256-entry lookup instead
  of `powf` per channel. Mesh build fell from ~10.4 ms to ~3.3–4.0 ms.
- **Resolution**: `--render-scale auto` targets 1.2M scene pixels, so a
  1920×1008 window renders the scene at 1461×767 and upscales through the
  linear post sampler. The HUD and post chain stay native.
- **Profiling**: `--benchmark` now prints `sim`, `mesh`, `hud`, `ui` and
  `submit` phase averages, plus the mesh shadow/build split and unit count.

The remaining frame is dominated by `submit` (scene + shadow + post) at
~9.0 ms, then mesh ~3.9 ms and HUD ~0.7 ms on this run. Native-resolution
1080p still needs a further ~0.7 ms from the scene pass to clear 60 without
the auto scale.

## 3. Current hot spots

1. **Shadow pass**: full static casters plus dynamic casters are submitted into
   a camera-relative 1024 map every other frame. Disabling shadows recovers
   roughly 5 ms in the release build.
2. **Terrain fragment cost**: base classification now samples four ground
   textures after dropping the redundant water fetch; dirt patch, road soil,
   slope and noise remain. 4× MSAA multiplies the cost.
3. **No GPU timing**: frame rate was measured through the window title. We need
   GPU timestamps and a repeatable benchmark scene before optimizing blind.
4. **CPU-side animation allocations**: `Model::animated_positions` allocates a
   fresh vector for each visible unit every frame. At current unit counts this
   is small, but it will matter for 10 heroes plus large waves.
5. **HUD geometry**: the HUD is rebuilt and uploaded every frame. Static
   panels and glyph runs could be cached; dynamic numbers still update.
6. **Repeated model draws**: the static mesh is merged, not instanced. Future
   scenery at larger draw distances will benefit from instancing and batching.
7. **No streaming/LOD**: the whole 5×5 block is resident. Chunk culling hides
   off-screen chunks, but there is no distance LOD or async load/unload.
8. **Startup cost**: decoding models, building chunk meshes and baking portraits
   is synchronous. Move to background jobs when the lobby and map streaming
   arrive.

## 4. Priority road map

### P0 — Instrumentation and performance gates (started)

- ✅ `--benchmark <seconds>` with avg/p50/p95/p99 frame logging.
- ✅ `--present-mode fifo|immediate|mailbox` and `--msaa 1|4` controls.
- ✅ CPU phase timing: sim, mesh build, HUD, interface and submit.
- ✅ `--window-size WIDTHxHEIGHT` and `--render-scale auto|0.5..1.0` for a
  repeatable 1080p and native-resolution comparison.
- [ ] Enable `wgpu` timestamp queries where supported.
- [ ] Add golden performance smoke tests for chunk counts and draw counts.
- [ ] Make a 1080p capture the default benchmark scene before further
  renderer changes; `--window-size 1920x1080` is the current gate.

Acceptance: one command produces reproducible CPU/GPU frame budgets on the dev
machine, and regressions are visible in CI or a checked-in report.

### P1 — Shadow map cost (largely done)

- ✅ Tree trunk/canopy and building AABB shadow proxies cut the static caster
  mesh from 413,513 to 38,028 triangles.
- ✅ Benchmarked shadow-on vs shadow-off is now within noise at the test view.
- [ ] Cache static shadows by camera tile to eliminate even the proxy pass when
  the camera is stationary.
- [ ] Consider cascaded shadow maps for a larger camera-relative range.
- [ ] Keep dynamic casters at every-other-frame updates; interpolate where lag
  is visible at 1080p.

Acceptance: shadows-on remains within 1–1.5 ms of shadows-off at 1080p.

### P2 — Terrain fragment shader (started)

- ✅ Removed the redundant water-layer fetch underneath the water mesh.
- ✅ Stopped green ground from falling into the cobblestone/stone classifier.
- [ ] Pre-bake classification weights/top-two layers per terrain vertex instead
  of sampling four ground layers in the fragment shader.
- [ ] Move macro dirt patches and road masks into a low-resolution control
  texture.
- [ ] Profile 1080p with `--msaa 1 --shadows off`.
- Keep nearest filtering and the OSRS look; use mips, not expensive PBR.

Acceptance: terrain pass drops below 5 ms at 1080p and stays visually
indistinguishable in A/B captures.

### P3 — Geometry, LOD and streaming

- ✅ Chunk size retuned to 24 tiles after benchmarking 16/20/24/28/32; water,
  terrain and model draw counts dropped by roughly half without a visual
  regression at the default camera.

- Add distance LODs for trees, ruins and ground decor (low-poly proxy meshes).
- Stream chunks around the camera instead of keeping the 5×5 block resident.
- Add terrain skirts at block/chunk boundaries to hide seams.
- Instance repeated scenery models and batch static draws.
- Add a GPU-driven or indirect path only after CPU draw submission shows up in
  the frame budget.

Acceptance: draw distance 2× the current horizon with stable 60 fps and no
visible chunk popping at the default camera.

### P4 — Animation and simulation

- ✅ Cull dynamic unit animation and mesh work past 9,000 units from the
  camera focus.
- ✅ Cache per-model face palette RGB and use an sRGB byte LUT; mesh build
  fell from ~10.4 ms to ~3.5–4.0 ms in the 1080p benchmark.
- [ ] Reuse per-unit vertex position buffers (`animated_positions` should
  write into a caller-owned Vec).
- [ ] Move unit updates to a fixed 30 Hz deterministic sim tick; interpolate
  render transforms between ticks.
- [ ] Convert the unit array to stable slots/IDs with deferred removal; avoid
  per-frame cloning and temporary Vecs in targeting/pathing.
- [ ] Cache pathfinding per lane/segment; A* only on target changes.
- [ ] Add a headless soak benchmark with thousands of ticks and a state hash.

Acceptance: 10 heroes + 100 minions + projectiles update under 2 ms/frame CPU
and produce identical hashes across runs.

### P5 — Post and quality settings

- Implement SSAO on a depth/normal prepass; keep a cheap fallback.
- Add a quality preset that selects MSAA 1/4, shadow resolution and bloom.
- Improve bloom to a proper mip chain and add an HDR exposure control.
- Add alpha-tested foliage shadow casters if the visual win is worth the cost.

Acceptance: SSAO and MSAA settings produce predictable frame budgets and can be
toggled without restarting a match.

### P6 — Engine/content pipeline

- Cache-native terrain layer blending (per-tile texture IDs, not the current
  six fixed layers).
- Async startup: cache decode, texture upload and chunk splitting on worker
  threads.
- Hot-reload champions/items/maps; validate references with `rift-tools`.
- Expand profiling to memory: static meshes, texture array, chunk overhead.

## 5. Profiling recipes

```sh
# Release, uncapped, current defaults
cargo run --release -p rift-client -- --present-mode immediate

# Fixed-duration frame-time stats, then exit
cargo run --release -p rift-client -- --present-mode immediate --benchmark 8

# Shadow cost
cargo run --release -p rift-client -- --present-mode immediate --shadows off

# MSAA cost
cargo run --release -p rift-client -- --present-mode immediate --msaa 1

# Bounded chunk draw counts
RUST_LOG=rift_client::renderer=info cargo run --release -p rift-client

# Simulation stress
cargo run --release -p rift-client -- --timescale 20 --shadows off --present-mode immediate
```

## 6. Design notes

- Keep scenery OSRS-flat-shaded and nearest-filtered. Do not introduce PBR to
  win a frame-time graph.
- Prefer chunk-level culling and LOD over shrinking the visible world.
- Every renderer optimization should have a visual A/B capture and a measured
  budget before it is considered done.
- The 1-tile = 128-unit contract and cache-native assets remain hard rules.
