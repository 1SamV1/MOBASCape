struct Light {
    position: vec4<f32>,
    color: vec4<f32>,
    params: vec4<f32>,
};

struct SceneGlobals {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    ambient_sky: vec4<f32>,
    ambient_ground: vec4<f32>,
    fog_color: vec4<f32>,
    fog_params: vec4<f32>,
    sky_top: vec4<f32>,
    sky_horizon: vec4<f32>,
    sun_screen: vec4<f32>,
    light_view_proj: mat4x4<f32>,
    shadow_params: vec4<f32>,
    tuning: vec4<f32>,
    vision: vec4<f32>,
    vision_grid: vec4<f32>,
    light_params: vec4<f32>,
    fog_hidden: vec4<f32>,
    grass_params: vec4<f32>,
    lights: array<Light, 16>,
    light_count: vec4<u32>,
};

@group(0) @binding(0) var<uniform> scene: SceneGlobals;
@group(0) @binding(1) var shadow_map: texture_depth_2d;
@group(0) @binding(2) var shadow_sampler: sampler_comparison;
@group(0) @binding(3) var terrain_textures: texture_2d_array<f32>;
@group(0) @binding(4) var terrain_sampler: sampler;
@group(0) @binding(5) var model_textures: texture_2d_array<f32>;
@group(0) @binding(6) var model_sampler: sampler;
@group(0) @binding(7) var grass_textures: texture_2d_array<f32>;
// The team's line-of-sight grid, one texel per 128-unit tile (1 = seen).
@group(0) @binding(8) var vision_texture: texture_2d<f32>;
@group(0) @binding(9) var vision_sampler: sampler;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) ao: f32,
    @location(5) material: u32,
    @location(6) texture_id: u32,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) world: vec3<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) ao: f32,
    @location(5) @interpolate(flat) material: u32,
    @location(6) @interpolate(flat) texture_id: u32,
};

@vertex
fn vs(input: VsIn) -> VsOut {
    var output: VsOut;
    output.clip = scene.view_proj * vec4<f32>(input.position, 1.0);
    output.color = input.color;
    output.world = input.position;
    output.normal = input.normal;
    output.uv = input.uv;
    output.ao = input.ao;
    output.material = input.material;
    output.texture_id = input.texture_id;
    return output;
}

fn dynamic_lighting(input: VsOut) -> vec3<f32> {
    var sum = vec3<f32>(0.0);
    let N = normalize(input.normal);
    let count = min(scene.light_count.x, 16u);
    for (var i = 0u; i < count; i = i + 1u) {
        let light = scene.lights[i];
        let radius = max(light.params.x, 1.0);
        let to_light = light.position.xyz - input.world;
        // Most fragments are outside a point light's small radius (the
        // fountain is the usual one). Test squared distance first so those
        // fragments never pay for the normalize and dot.
        let dist2 = dot(to_light, to_light);
        if (dist2 >= radius * radius) {
            continue;
        }
        let dist = sqrt(dist2);
        let atten = 1.0 - dist / radius;
        let lndotl = max(dot(N, normalize(to_light)), 0.0);
        sum += light.color.rgb * light.color.a * atten * atten * lndotl;
    }
    return sum;
}

fn shadow_factor(input: VsOut) -> f32 {
    if (scene.shadow_params.w < 0.5) {
        return 1.0;
    }
    // Push the lookup along the surface normal. The static tree/building
    // shadow proxies are deliberately coarse boxes; without a normal offset
    // they self-shadow canopy tops with visible rectangular acne.
    let shadow_world = input.world + normalize(input.normal) * 8.0;
    let coord = scene.light_view_proj * vec4<f32>(shadow_world, 1.0);
    let proj = coord.xyz / coord.w;
    let uv = vec2<f32>(proj.x * 0.5 + 0.5, -proj.y * 0.5 + 0.5);
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0))
        || proj.z < 0.0 || proj.z > 1.0) {
        return 1.0;
    }
    let texel = scene.shadow_params.x;
    let bias = scene.shadow_params.y;
    let depth = proj.z - bias;
    // Wide four-tap PCF. The static tree casters are coarse boxes, and a
    // single bilinear tap gave their canopy shadows a hard, stencilled edge
    // that read as a hole in the ground rather than an out-of-focus shadow.
    let o = texel * 1.8;
    let sum = textureSampleCompare(shadow_map, shadow_sampler, uv + vec2<f32>(-o, -o), depth)
        + textureSampleCompare(shadow_map, shadow_sampler, uv + vec2<f32>(o, o), depth);
    return sum * 0.5;
}

fn lighting(input: VsOut) -> vec3<f32> {
    let V = normalize(scene.camera_pos.xyz - input.world);
    var N = normalize(input.normal);
    // OSRS models are single sided and a few ship with inverted normals.
    // Flipping the normal towards the camera keeps the face the player can
    // actually see lit instead of black.
    if (dot(N, V) < 0.0) {
        N = -N;
    }
    let L = normalize(scene.sun_dir.xyz);

    var ndotl = max(dot(N, L), 0.0);
    if (scene.tuning.y > 0.0) {
        ndotl = floor(ndotl * scene.tuning.y + 0.5) / scene.tuning.y;
    }

    let hemi = mix(
        scene.ambient_ground.rgb,
        scene.ambient_sky.rgb,
        N.y * 0.5 + 0.5
    );

    let shadow = select(
        shadow_factor(input),
        1.0,
        input.material == 7u || input.material == 8u
    );
    // Keep a fifth of the sunlight inside shadow so the shadowed ground
    // retains its albedo instead of collapsing to a black polygon.
    // Phase 1 retune: a stronger key light and deeper shadows over a lower
    // ambient, so trees and walls ground themselves like the reference.
    let sun = scene.sun_color.rgb * scene.light_params.z * mix(scene.light_params.w, 1.0, shadow);
    // Camera facing fill keeps the fronts of models readable even when the
    // sun is behind them, which a top down MOBA camera hits constantly.
    let unit_fill = select(0.24, 0.40, input.material == 3u);
    let fill = max(dot(N, V), 0.0) * unit_fill;
    let ambient = select(scene.light_params.x, scene.light_params.y, input.material == 3u);
    var lit = albedo(input) * (hemi * ambient + sun * ndotl + fill) * input.ao * 1.02;

    lit += input.color.rgb * dynamic_lighting(input);
    return lit;
}

// How much of the team sees this ground point (0..1), softened over about
// one tile with a five-tap filtered lookup so edges fade instead of
// stepping per tile.
fn vision_seen(xz: vec2<f32>) -> f32 {
    let uv = (xz - scene.vision_grid.xy) * scene.vision_grid.z;
    let t = scene.vision_grid.z * 96.0;
    var sum = textureSampleLevel(vision_texture, vision_sampler, uv, 0.0).r * 2.0;
    sum += textureSampleLevel(vision_texture, vision_sampler, uv + vec2<f32>(t, 0.0), 0.0).r;
    sum += textureSampleLevel(vision_texture, vision_sampler, uv - vec2<f32>(t, 0.0), 0.0).r;
    sum += textureSampleLevel(vision_texture, vision_sampler, uv + vec2<f32>(0.0, t), 0.0).r;
    sum += textureSampleLevel(vision_texture, vision_sampler, uv - vec2<f32>(0.0, t), 0.0).r;
    return smoothstep(0.15, 0.85, sum / 6.0);
}

fn fog_factor(input: VsOut) -> f32 {
    let dist = length(input.world - scene.camera_pos.xyz);
    let base = 1.0 - exp(-scene.fog_params.x * dist);
    // Distance and height haze. The old two-octave mist banks cost several
    // noise fetches on every fragment for a difference only visible in
    // motion; keep the depth cue and spend the budget on frame rate.
    let height = exp(
        -max(input.world.y - scene.fog_params.z, 0.0) * scene.fog_params.y
    );
    return clamp(base * 1.02 + height * scene.fog_params.w, 0.0, 0.90);
}

fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn noise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash21(i), hash21(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash21(i + vec2<f32>(0.0, 1.0)), hash21(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y
    );
}

fn is_leaf_texture(id: u32) -> bool {
    return id == 8u || id == 28u || id == 30u || id == 41u || id == 60u;
}

fn sample_model_texture(input: VsOut) -> vec4<f32> {
    // Textured faces carry a per-face planar UV generated on the CPU. This
    // follows the OSRS texture-plane idea and avoids the smeared triplanar
    // look on large cone/roof triangles.
    let sampled = textureSampleGrad(
        model_textures, model_sampler, input.uv, input.texture_id, dpdx(input.uv), dpdy(input.uv)
    );
    // Cache foliage palettes are very dark in linear light. Lift leaves enough
    // to read as a canopy but keep them olive rather than fluorescent: the
    // reference client's trees sit close to the ground palette.
    if (is_leaf_texture(input.texture_id)) {
        let leaf = vec3<f32>(
            sampled.r * 0.88 + sampled.g * 0.02,
            sampled.g * 0.96,
            sampled.b * 0.66 + sampled.g * 0.03
        );
        let leaf_luma = dot(leaf, vec3<f32>(0.299, 0.587, 0.114));
        let muted = mix(vec3<f32>(leaf_luma), leaf, 0.82);
        return vec4<f32>(
            pow(max(muted, vec3<f32>(0.0)), vec3<f32>(0.86)) * 1.34,
            sampled.a
        );
    }
    return vec4<f32>(
        pow(max(sampled.rgb, vec3<f32>(0.0)), vec3<f32>(0.88)) * 1.18,
        sampled.a
    );
}

fn water_surface(input: VsOut) -> vec4<f32> {
    let time = scene.tuning.z;
    let flow = vec2<f32>(time * 0.035, time * 0.018);
    let uv_a = input.world.xz / 220.0 + flow;
    let uv_b = input.world.zx / 170.0 - flow * 0.8;
    let water_a = textureSample(terrain_textures, terrain_sampler, uv_a, 4).rgb;
    let water_b = textureSample(terrain_textures, terrain_sampler, uv_b, 4).rgb;
    let water_tex = mix(water_a, water_b, 0.45);

    // Small analytic ripples keep the surface alive without another normal
    // map. The amplitude is deliberately low so the river still reads as a
    // flat OSRS water plane from the MOBA camera.
    let ripple = vec3<f32>(
        sin(input.world.x * 0.045 + time * 1.1) * 0.075
            + sin(input.world.z * 0.12 - time * 0.9) * 0.035,
        1.0,
        cos(input.world.z * 0.055 - time * 0.8) * 0.06
            + sin(input.world.x * 0.09 + time * 1.3) * 0.03
    );
    let N = normalize(ripple);
    let V = normalize(scene.camera_pos.xyz - input.world);
    let L = normalize(scene.sun_dir.xyz);
    let fresnel = pow(1.0 - max(dot(N, V), 0.0), 3.0);
    let sun = pow(max(dot(reflect(-L, N), V), 0.0), 96.0);
    let sky = mix(scene.sky_horizon.rgb, scene.sky_top.rgb, 0.30);
    let deep = mix(vec3<f32>(0.028, 0.105, 0.155), water_tex * 0.68, 0.62);
    var color = mix(deep, sky, fresnel * 0.80);
    color += scene.sun_color.rgb * sun * 0.85;

    // A shallow rim at the mask edge keeps the water from ending in a flat
    // dark cut. The river mesh stores the water mask in uv.x.
    let shore = 1.0 - smoothstep(0.06, 0.42, input.uv.x);
    color = mix(color, vec3<f32>(0.42, 0.62, 0.58), shore * 0.20);

    let fog = fog_factor(input);
    color = mix(color, scene.fog_color.rgb, fog * 0.82);
    let alpha = clamp(
        input.color.a * (0.52 + fresnel * 0.48) + shore * 0.10,
        0.0,
        0.95
    );
    return vec4<f32>(color, alpha);
}

/// Procedural ashlar masonry for the castle boxes.
///
/// The castle is assembled from plain boxes with no authored UVs, so there is
/// no cache sprite that could be projected on them correctly. Generate the
/// courses in world space instead: staggered blocks on the vertical faces, big
/// flagstones on the walkable tops, and use the stone sprite only as surface
/// grain. Fitted blocks read as a wall at every distance; a projected sprite
/// alone washed out into the near-white vertex colour.
fn castle_stone(input: VsOut) -> vec3<f32> {
    let n = abs(normalize(input.normal));
    let total = max(n.x + n.y + n.z, 0.0001);
    let w = n / total;

    // Grain from the cache stone sprite, projected on all three axes. The
    // sprite averages 0.13 in linear light, so centre it before use or the
    // whole wall turns into a flat mid grey.
    let scale = 420.0;
    let sx = textureSample(terrain_textures, terrain_sampler, input.world.zy / scale, 2).rgb;
    let sy = textureSample(terrain_textures, terrain_sampler, input.world.xz / scale, 2).rgb;
    let sz = textureSample(terrain_textures, terrain_sampler, input.world.xy / scale, 2).rgb;
    let grain = sx * w.x + sy * w.y + sz * w.z;
    let relief = clamp(0.92 + (grain - vec3<f32>(0.13)) * 1.5, vec3<f32>(0.60), vec3<f32>(1.24));

    // Vertical faces: horizontal courses, staggered joints row to row. Project
    // along a tangent derived from the normal so the rotated castle wings keep
    // level beds instead of shearing with the world axes.
    let tangent = vec3<f32>(-n.z, 0.0, n.x);
    let horiz = dot(input.world.xz, tangent.xz) / max(length(tangent.xz), 0.18);
    let course = 92.0;
    let block = 196.0;
    let row = floor(input.world.y / course);
    let bx = (horiz + fract(row * 0.5) * block) / block;
    let fx = fract(bx);
    let fy = fract(input.world.y / course);
    let face_edge = min(
        min(fx, 1.0 - fx) * block,
        min(fy, 1.0 - fy) * course
    );
    // 0 in the mortar, 1 in the middle of a block: a wide bevel reads as
    // recessed joints even when one course is only a few pixels tall.
    let face_bevel = smoothstep(3.0, 30.0, face_edge);
    let face_tone = 0.90 + noise2(vec2<f32>(floor(bx) * 4.1, row * 2.7)) * 0.20;

    // Tops: larger flagstone paving, one joint every 190 units.
    let paving = 190.0;
    let tz = input.world.z / paving + floor(input.world.x / paving) * 0.5;
    let top_edge = min(
        min(fract(input.world.x / paving), 1.0 - fract(input.world.x / paving)) * paving,
        min(fract(tz), 1.0 - fract(tz)) * paving
    );
    let top_bevel = smoothstep(3.0, 26.0, top_edge);
    let top_tone = 0.94 + noise2(input.world.xz / 300.0) * 0.12;

    let on_top = smoothstep(0.46, 0.86, n.y);
    let bevel = mix(face_bevel, top_bevel, on_top);
    let tone = mix(face_tone, top_tone, on_top);

    // Weathered patches and a darker footing where damp collects.
    let grime = noise2(input.world.xz / 620.0) * 0.55
        + noise2(input.world.yx / 210.0) * 0.45;
    var stone = relief * tone * (0.94 + grime * 0.12);
    stone *= mix(0.58, 1.04, bevel);
    let footing = 1.0 - smoothstep(0.0, 240.0, input.world.y);
    stone *= 1.0 - footing * 0.16;
    return input.color.rgb * stone;
}

fn albedo(input: VsOut) -> vec3<f32> {
    var c = input.color.rgb;
    if (input.material == 8u) {
        return castle_stone(input);
    }
    if (input.material == 5u || input.material == 7u) {
        let tex = sample_model_texture(input);
        return tex.rgb * input.color.rgb;
    }
    if (input.material == 4u) {
        // Stable world-space planar UVs. The 384-unit grass repeat is a
        // compromise between the old 512-unit macro tile and an expensive
        // second detail sample; derivatives keep the mip chain useful at
        // distance.
        let slope = clamp(input.uv.x, 0.0, 1.0);
        // 384 units per repeat is a compromise between the old 512-unit
        // macro tile (too soft) and a second detail sample (too expensive).
        let uv = input.world.xz / 384.0;
        let packed = input.texture_id;
        let variant = min(packed & 0xFFu, 5u);
        let overlay_texture = (packed >> 8u) & 0xFFu;
        let overlay_weight = f32((packed >> 16u) & 0xFFu) / 255.0;
        // The grass array mirror-tiles each 128px sprite, so a single
        // axis-aligned sample shows a seam grid every ~96 units. Two samples
        // at unrelated rotations and scales cancel each other's seams.
        let rot_a = mat2x2<f32>(0.921, -0.391, 0.391, 0.921);
        let rot_b = mat2x2<f32>(0.755, 0.656, -0.656, 0.755);
        let uv_a = (rot_a * input.world.xz) / 430.0;
        let uv_b = (rot_b * input.world.xz) / 1030.0 + vec2<f32>(0.37, 0.71);
        let grass_a4 = textureSampleGrad(
            grass_textures, terrain_sampler, uv_a, variant, dpdx(uv_a), dpdy(uv_a)
        );
        let grass_b4 = textureSampleGrad(
            grass_textures, terrain_sampler, uv_b, variant, dpdx(uv_b), dpdy(uv_b)
        );
        // A coarse mip far below the sprite detail stands in for the mean
        // colour; sampled at a 6400-unit repeat its baked alpha noise gives
        // the slow lush/dry field patches.
        let macro_uv = input.world.xz / 6400.0 + vec2<f32>(0.19, 0.53);
        let grass_mean4 = textureSampleLevel(grass_textures, terrain_sampler, macro_uv, variant, 5.0);
        let grass_a = grass_a4.rgb;
        let grass_b = grass_b4.rgb;
        let grass_mean = grass_mean4.rgb;
        // Alpha carries tileable noise baked at load (see grass_textures.rs);
        // the two rotated samples give two unrelated octaves of mottle.
        let macro_n = grass_mean4.a;
        let mottle = grass_b4.a * 0.6 + grass_a4.a * 0.4;
        let grass_detail = mix(grass_a, grass_b, 0.45);
        let grass_raw = mix(grass_mean, grass_detail, 0.7);
        // Push the palette towards the reference's deep, saturated OSRS
        // green and away from lime/olive; slow patches of lusher and drier
        // grass give fields scale without a visible tile.
        let grass_luma = dot(grass_raw, vec3<f32>(0.299, 0.587, 0.114));
        // Retuned 2026-11-23 against the reference study (ROADMAPPLAN.md
        // phase 1): the old dry tint pushed fields to highlighter lime.
        let lush = vec3<f32>(0.50, 0.84, 0.50);
        let dry = vec3<f32>(0.70, 0.82, 0.50);
        let field_tint = mix(lush, dry, smoothstep(0.45, 0.85, macro_n));
        // Saturation, base brightness and mottle come from data/lighting.json.
        let grass = mix(vec3<f32>(grass_luma), grass_raw, scene.grass_params.x)
            * field_tint
            * (scene.grass_params.y + scene.grass_params.z * mottle);
        // The dark biome grass sprites are nearly black in linear light and
        // read as flat fog/water in low jungle. Lift the linear values while
        // preserving the sprite's texture and hue.

        // Project the soil and rock sprites onto the face closest to the
        // camera. A flat xz projection stretches one texel over four world
        // units and turns every hillside into a smooth brown blob.
        let n = normalize(input.normal);
        let ground_uv = select(
            select(input.world.xy, input.world.zy, n.x >= n.z),
            input.world.xz,
            n.y >= max(n.x, n.z)
        ) / 128.0;
        let dirt = textureSample(terrain_textures, terrain_sampler, ground_uv, 1).rgb;
        let stone = textureSample(terrain_textures, terrain_sampler, ground_uv, 2).rgb;
        let sand = dirt;
        // The river mesh covers these tiles, so a flat deep tint is enough.
        let water = vec3<f32>(0.018, 0.060, 0.105);

        // Classify in display space: linear values compress saturation and
        // make grass look like stone.
        let d = pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));
        let green = max(d.g - d.r, 0.0) * 2.2 + max(d.g - d.b, 0.0);
        let brown = max(d.r - d.g, 0.0) * 2.0 + max(d.g - d.b, 0.0) * 0.15;
        let blue = max(d.b - d.g, 0.0) * 2.0 + max(d.b - d.r, 0.0);
        let sat = max(max(d.r, d.g), d.b) - min(min(d.r, d.g), d.b);
        let green_dominance = smoothstep(0.015, 0.11, green);
        let gray = (1.0 - smoothstep(0.05, 0.25, sat))
            * (1.0 - smoothstep(0.12, 0.30, d.r - d.b))
            * (1.0 - green_dominance)
            * smoothstep(0.045, 0.20, slope);
        let sandy = max(d.r - d.b, 0.0) * max(d.g - d.b, 0.0) * 4.0;
        let total = max(green + brown + blue + gray + sandy, 0.0001);
        // Gentle hills should read as grass, not mud. Only genuinely steep
        // faces become rock; weak brown tints are damped on open ground,
        // while a strongly brown patch still reads as bare soil.
        let rise = smoothstep(0.05, 0.28, slope);
        let bare_patch = smoothstep(0.14, 0.36, max(d.r - d.g, 0.0));
        let grass_weight = (green / total) * (1.0 + rise * 1.2);
        let dirt_weight = (brown / total)
            * mix(0.42, 1.0, bare_patch)
            * (1.0 - rise * 0.80);
        let stone_weight = gray / total;
        let sand_weight = sandy / total;
        let water_weight = blue / total;
        let weight_total = max(
            grass_weight + dirt_weight + stone_weight + sand_weight + water_weight,
            0.0001
        );
        var tex = (grass * grass_weight
            + dirt * dirt_weight
            + stone * stone_weight
            + sand * sand_weight
            + water * water_weight) / weight_total;
        // Terraces make slopes bimodal: fields are near flat and cliff
        // risers are steep. Rock belongs only on the risers, with a cool
        // grey-brown grade and horizontal strata so faces read as stone.
        let cliff = smoothstep(0.13, 0.32, slope);
        let strata = 0.86 + 0.14 * sin(input.world.y * 0.29 + (input.world.x + input.world.z) * 0.0031);
        let rock = stone * vec3<f32>(0.92, 0.90, 0.86) * strata * (0.80 + 0.2 * c);
        tex = mix(tex, rock, cliff);

        // Cache overlay art wins where the source tile carries a textured
        // overlay. This restores real OSRS paths, dirt and biome ground that
        // the old colour classifier could only approximate.
        if (overlay_weight > 0.001 && overlay_texture > 0u) {
            let overlay = textureSample(
                model_textures,
                model_sampler,
                input.world.xz / 128.0,
                overlay_texture
            ).rgb;
            // Match the overlay's brightness to the classified ground. Some
            // OSRS overlays are very dark marsh/grass art; replacing the
            // whole tile made low ground read as flat fog/water.
            let base_luma = max(dot(tex, vec3<f32>(0.299, 0.587, 0.114)), 0.03);
            let overlay_luma = max(dot(overlay, vec3<f32>(0.299, 0.587, 0.114)), 0.03);
            let corrected = overlay
                * clamp(base_luma / overlay_luma, 0.65, 1.35)
                * (0.86 + c * 0.28);
            tex = mix(tex, corrected, overlay_weight);
        }

        let steep = smoothstep(0.08, 0.30, slope);
        tex *= 1.0 - steep * 0.12;

        // Lanes are baked into the terrain; the road sprites are blended in
        // world space so there is no ribbon hovering over the ground.
        let water_mask = clamp(input.uv.y, 0.0, 1.0);
        let lane = clamp(input.color.a, 0.0, 1.0)
            * (1.0 - smoothstep(0.08, 0.30, slope))
            * (1.0 - smoothstep(0.02, 0.35, water_mask));
        if (lane > 0.002) {
            let road_uv = input.world.xz / 240.0;
            let soil = textureSample(
                model_textures, model_sampler, road_uv, 32
            ).rgb;
            let dark_soil = textureSample(
                model_textures, model_sampler, road_uv * 0.63, 11
            ).rgb;
            let road = mix(soil, dark_soil, 0.38);
            tex = mix(tex, road * (0.90 + c * 0.35), lane * 0.96);
        }

        // The sprite carries the palette; the vertex colour only nudges it.
        // Cliff faces skip the nudge: the underlay colour is grass green
        // and would moss over the stone.
        let tint = mix(0.86 + c * 1.05, vec3<f32>(1.0), cliff);
        c = tex * tint;
    }
    return c;
}

@fragment
fn fs(input: VsOut) -> @location(0) vec4<f32> {
    let flags = u32(scene.tuning.w);
    let debug_view = flags & 15u;

    if (debug_view == 1u) {
        return vec4<f32>(albedo(input), 1.0);
    }
    if (debug_view == 2u) {
        return vec4<f32>(normalize(input.normal) * 0.5 + 0.5, 1.0);
    }
    if (debug_view == 3u) {
        return vec4<f32>(lighting(input), 1.0);
    }
    if (debug_view == 4u) {
        return vec4<f32>(vec3<f32>(fog_factor(input)), 1.0);
    }
    if (debug_view == 5u) {
        return vec4<f32>(dynamic_lighting(input), 1.0);
    }
    if (debug_view == 6u) {
        return vec4<f32>(vec3<f32>(shadow_factor(input)), 1.0);
    }
    if (debug_view == 7u) {
        return vec4<f32>(vec3<f32>(input.ao), 1.0);
    }

    // River water is a blended surface with its own fresnel/ripple shading.
    if (input.material == 6u) {
        return water_surface(input);
    }

    // Unlit geometry: fog-of-war, world-space bars, rings, and markers.
    if (input.material == 1u) {
        return input.color;
    }

    // Classic baseline for A/B comparison.
    if ((flags & 256u) != 0u) {
        let dist = length(input.world - scene.camera_pos.xyz);
        let fog = clamp((dist - 9000.0) / 28000.0, 0.0, 1.0);
        let sky = vec3<f32>(0.296, 0.478, 0.638);
        return vec4<f32>(mix(input.color.rgb, sky, fog * 0.82), 1.0);
    }

    var lit = lighting(input);
    // Fog of war. In a match it is the team's line-of-sight grid: everything
    // an ally (hero, minion, tower) sees is lit, the rest of the map falls
    // to a dim, greyed version of itself. Without a grid (menus, demos) it
    // is the old circle around the hero.
    var seen = 1.0;
    if (scene.vision_grid.w > 0.5) {
        seen = vision_seen(input.world.xz);
    } else {
        let los_distance = length(input.world.xz - scene.vision.xz);
        seen = 1.0 - smoothstep(scene.vision.w * 0.72, scene.vision.w * 1.45, los_distance);
    }
    let hidden_grey = vec3<f32>(dot(lit, vec3<f32>(0.299, 0.587, 0.114)));
    let hidden = mix(hidden_grey, lit, scene.fog_hidden.w) * scene.fog_hidden.rgb;
    lit = mix(hidden, lit, seen);
    let fog = fog_factor(input);
    let sun_amount = max(
        dot(normalize(input.world - scene.camera_pos.xyz), normalize(scene.sun_dir.xyz)),
        0.0
    );
    let fog_tint = mix(scene.fog_color.rgb, scene.sun_color.rgb, sun_amount * 0.15);
    lit = mix(lit, fog_tint, fog);
    return vec4<f32>(lit, 1.0);
}

// Additive combat effects (see `fx.rs`). Colour alpha carries intensity and
// the material picks the shape; the pipeline adds the result to the scene
// with depth testing but no depth writes, so glows stack into bloom.
@fragment
fn fs_fx(input: VsOut) -> @location(0) vec4<f32> {
    let intensity = input.color.a;
    var rgb = input.color.rgb;
    var shape = 1.0;
    let m = input.material;
    if (m == 20u) {
        // Soft sprite with a hot white core.
        let r2 = dot(input.uv, input.uv);
        if (r2 >= 1.0) {
            discard;
        }
        let edge = 1.0 - r2;
        let glow = exp(-r2 * 3.2) * edge;
        let core = exp(-r2 * 22.0);
        rgb = rgb * glow + vec3<f32>(core * 0.40);
        shape = 1.0;
    } else if (m == 21u) {
        // Ribbon: soft across the strip, a brighter spine down the middle.
        let v = abs(input.uv.y);
        let soft = pow(max(1.0 - v, 0.0), 1.6);
        let spine = pow(max(1.0 - v, 0.0), 8.0);
        rgb = rgb * soft + vec3<f32>(spine * 0.35);
    } else if (m == 23u) {
        let v = abs(input.uv.y);
        shape = pow(max(1.0 - v, 0.0), 1.8);
    } else if (m == 24u) {
        // Ground indicator: faint fill, bright rim, a slow inward shimmer.
        let d = clamp(input.uv.y, 0.0, 1.0);
        let rim = smoothstep(0.84, 0.97, d) * (1.0 - smoothstep(0.985, 1.0, d) * 0.6);
        let shimmer = 0.5 + 0.5 * sin(d * 18.0 - scene.tuning.z * 4.0);
        shape = 0.10 + 0.05 * shimmer + 0.85 * rim;
    }
    // Distance haze dims far effects the same way it hazes the scene.
    let haze = 1.0 - fog_factor(input) * 0.85;
    return vec4<f32>(rgb * shape * intensity * haze, 0.0);
}
