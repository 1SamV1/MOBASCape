# Engine road map

Last updated: 2026-11-21, after per-pass GPU timings and the point-light
radius gate that pushed native 1080p past 60 fps.

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
  grade + dither post, HUD after post. `--render-scale auto` targets 1.4M
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
culling recovered most of it. These early 720p numbers predate the GPU timing
harness; sections 2b and 2c hold the current 1080p budgets. They remain a
dev-laptop measurement, not a certified benchmark; use the harness described
below before treating them as budgets.

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
| 1280×720 | 1280×720 (1.00×) | **9.30 ms** | 10.44 ms | 106.8 |
| 1920×1008 | 1578×828 (0.82×) | **13.22 ms** | 14.77 ms | 75.2 |

The same 1920×1008 run before this pass measured p50 61.5 ms. Native
1920×1008 with `--msaa 1 --render-scale 1.0` remains around p50 17.2 ms; the
auto scene scale is what produces the 60 fps margin on this iGPU.

What moved the number:

- **Terrain albedo**: removed the per-fragment multi-octave macro noise,
  meadow/hollow/thicket/patch/noise warp and speckle stack. The cache grass
  sprites, biome vertex tint and classification remain. Fog banks and grass
  normal perturbation were removed for the same reason. The sun shadow lookup
  is now two taps instead of four.
- **Animated mesh culling**: units outside the camera view radius plus a
  2,000-unit margin (5,000–14,000 units depending on distance and pitch) skip
  animation, mesh and blob-shadow construction. At the default camera this
  leaves roughly 14–18 units instead of 157.
- **Mesh CPU**: face palette RGB is cached per model instance instead of
  converted from HSL every frame, per-unit animation pose buffers are reused,
  and sRGB byte colours use a 256-entry lookup instead of `powf` per channel.
  Mesh build fell from ~10.4 ms to ~3.5–4.1 ms.
- **Resolution**: `--render-scale auto` targets 1.4M scene pixels, so a
  1920×1008 window renders the scene at 1578×828 and upscales through the
  linear post sampler. The HUD and post chain stay native.
- **Post**: with 4× MSAA the resolved scene is already antialiased, so the
  post pass skips its FXAA taps on that path. `--msaa 1` keeps FXAA.
- **Profiling**: `--benchmark` now prints `sim`, `mesh`, `hud`, `ui` and
  `submit` phase averages, plus the mesh shadow/build split and unit count.
  `--gpu-timing` (automatic under `--benchmark`) adds per-pass GPU time from
  timestamp queries: `benchmark gpu: frame=… shadow=… scene=… bloom=… post=…
  hud=… interface=…`.
- **UI cost**: solid theme panels were being tiled as 1×1 images. They now
  draw as one rectangle, so the live 1080p HUD dropped from 162,830 quads to
  2,327 and from 46.7 ms to ~0.9 ms of build time.

Timestamp queries put the 1920×1008 auto-scale frame at **10.76 ms GPU**:
scene **9.11 ms**, bloom **0.73 ms**, post **0.79 ms**, shadow **0.04 ms**,
HUD/interface **0.09 ms**. The old HUD's GPU pass is only **0.06 ms**; its
remaining cost is the 0.6 ms CPU build. The scene pass is still the only
large GPU cost, but native-resolution 1080p now clears 60 (details below).

### 2c. 2026-11-21 — Point-light radius gate

The lighting debug view showed dynamic point lights costing ~1.9 ms even
though the only active light (the fountain) covered a small part of the
screen. Fragments outside a light's radius still ran `normalize` and `dot`
before multiplying by an attenuation that was already zero.
`dynamic_lighting` now tests squared distance against the radius, skips the
rest for outside fragments, and computes `sqrt` only for covered fragments.
Visuals are identical because the old attenuation clamped to zero at exactly
the same radius.

| 1920×1008, MSAA 4, shadows, immediate | Before | After |
|---|---:|---:|
| Auto scale (0.82×) p50 | 12.61 ms | **11.11 ms** |
| Auto scale GPU frame | 12.00 ms | **10.76 ms** |
| Auto scale scene pass | 10.33 ms | **9.11 ms** |
| Native scale (1.00×) p50 | 16.23 ms | **14.35 ms** |
| Native scale p95 | 17.17 ms | **15.47 ms** |
| Native scale GPU frame | 15.90 ms | **13.86 ms** |
| Native scale scene pass | 14.19 ms | **12.14 ms** |

Native 1920×1008 now holds p50 14.35 ms / **69.7 fps** with p95 15.47 ms, so
the 60 fps target no longer depends on the auto scene scale. Tried and not
kept: a direct one-light fast path (no measurable change) and gradient-based
grass mip selection (no measurable change; reverted).

## 3. Current hot spots

1. **Scene fill**: the scene pass is ~9.1 ms at auto scale and ~12.1 ms
   native. The normal-only view costs ~4.0 ms, so 4× MSAA raster plus terrain
   and scenery fragments dominate; the shadow map is effectively free at
   ~0.04 ms.
2. **Terrain fragment cost**: the fast path samples grass, dirt and stone;
   sand reuses dirt. Albedo alone adds ~2.1 ms over the normal view, before
   lighting and fog.
3. **Post and bloom**: the full-screen chain is ~1.5 ms at native resolution
   and is the next fixed cost after the scene.
4. **HUD geometry**: the old HUD is rebuilt and uploaded every frame. The
   live HUD now draws flat panels as one rectangle instead of tiling a 1×1
   image, cutting its 1080p build from 46.7 ms to ~0.9 ms.
5. **Repeated model draws**: the static mesh is merged, not instanced. Future
   scenery at larger draw distances will benefit from instancing and batching.
6. **No streaming/LOD**: the whole 5×5 block is resident. Frustum and coarse
   distance culling hide off-screen chunks, but there is no distance LOD or
   async load/unload.
7. **Startup cost**: decoding models, building chunk meshes and baking portraits
   is synchronous. Move to background jobs when the lobby and map streaming
   arrive.

## 4. Priority road map

### P0 — Instrumentation and performance gates (started)

- ✅ `--benchmark <seconds>` with avg/p50/p95/p99 frame logging.
- ✅ `--present-mode fifo|immediate|mailbox` and `--msaa 1|4` controls.
- ✅ CPU phase timing: sim, mesh build, HUD, interface and submit.
- ✅ `--window-size WIDTHxHEIGHT` and `--render-scale auto|0.5..1.0` for a
  repeatable 1080p and native-resolution comparison.
- ✅ Enable `wgpu` timestamp queries where supported: `--gpu-timing` writes
  per-pass begin/end timestamps, reads them back asynchronously and reports
  averages; adapters without `TIMESTAMP_QUERY` fall back cleanly.
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

- ✅ Coarse distance culling: models/water 16k, blob shadows 14k and static
  shadow casters 9k by default (`--draw-distance`). Terrain draws to the
  horizon.
- [ ] Add distance LODs for trees, ruins and ground decor (low-poly proxy meshes).
- Stream chunks around the camera instead of keeping the 5×5 block resident.
- Add terrain skirts at block/chunk boundaries to hide seams.
- Instance repeated scenery models and batch static draws.
- Add a GPU-driven or indirect path only after CPU draw submission shows up in
  the frame budget.

Acceptance: draw distance 2× the current horizon with stable 60 fps and no
visible chunk popping at the default camera.

### P4 — Animation and simulation

- ✅ Cull dynamic unit animation and mesh work outside the camera view radius
  plus a 2,000-unit margin.
- ✅ Cache per-model face palette RGB and use an sRGB byte LUT; mesh build
  fell from ~10.4 ms to ~3.5–4.0 ms in the 1080p benchmark.
- ✅ Reuse per-unit vertex position and frame scratch buffers
  (`animated_positions_into`).
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

# Per-pass GPU budget (also automatic with --benchmark); drop the benchmark
# to print live GPU averages while playing
cargo run --release -p rift-client -- --present-mode immediate --benchmark 8 --gpu-timing
cargo run --release -p rift-client -- --present-mode immediate --gpu-timing

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
