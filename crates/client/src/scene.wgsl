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
    let sun = scene.sun_color.rgb * 0.72 * mix(0.52, 1.0, shadow);
    // Camera facing fill keeps the fronts of models readable even when the
    // sun is behind them, which a top down MOBA camera hits constantly.
    let unit_fill = select(0.24, 0.40, input.material == 3u);
    let fill = max(dot(N, V), 0.0) * unit_fill;
    let ambient = select(1.64, 1.74, input.material == 3u);
    var lit = albedo(input) * (hemi * ambient + sun * ndotl + fill) * input.ao * 1.02;

    lit += input.color.rgb * dynamic_lighting(input);
    return lit;
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

    let fog = fog_factor(input);
    color = mix(color, scene.fog_color.rgb, fog * 0.82);
    let alpha = clamp(input.color.a * (0.52 + fresnel * 0.48), 0.0, 0.92);
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
        // Stable world-space planar UVs. Two grass scales break the 4-tile
        // repeat: the 512-unit macro layer carries the field, and the
        // 192-unit detail layer restores close-up grass grain. Both use
        // derivatives so the mip chain is finally used at distance.
        let slope = clamp(input.uv.x, 0.0, 1.0);
        // 384 units per repeat is a compromise between the old 512-unit
        // macro tile (too soft) and a second detail sample (too expensive).
        let uv = input.world.xz / 384.0;
        let variant = min(input.texture_id, 5u);
        let grass_macro = textureSampleGrad(
            grass_textures,
            terrain_sampler,
            uv,
            variant,
            dpdx(uv),
            dpdy(uv)
        ).rgb;
        // Reuse the sampled grass luma for patch variation instead of four
        // extra hash evaluations per fragment.
        let field_tone = clamp(
            dot(grass_macro, vec3<f32>(0.9, 1.1, 0.7)) * 1.8,
            0.0,
            1.0
        );
        let grass_raw = grass_macro * (0.88 + field_tone * 0.24);
        let grass_luma = dot(grass_raw, vec3<f32>(0.299, 0.587, 0.114));
        let grass = mix(vec3<f32>(grass_luma), grass_raw, 1.18) * vec3<f32>(0.98, 1.10, 1.02);

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
        let cliff = smoothstep(0.30, 0.62, slope);
        tex = mix(tex, stone * (0.82 + 0.18 * c), cliff * 0.88);

        let steep = smoothstep(0.10, 0.38, slope);
        tex *= 1.0 - steep * 0.16;

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
        let tint = 0.86 + c * 1.05;
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
    // Fog of war: a consistent lateral circle around the hero. This is a
    // shader term rather than a mesh, so it never fights with terrain z
    // levels and the boundary is a smooth radius instead of a tile edge.
    let los_distance = length(input.world.xz - scene.vision.xz);
    let los = 1.0 - smoothstep(scene.vision.w * 0.72, scene.vision.w * 1.45, los_distance);
    lit = mix(lit * 0.55, lit, los);
    let fog = fog_factor(input);
    let sun_amount = max(
        dot(normalize(input.world - scene.camera_pos.xyz), normalize(scene.sun_dir.xyz)),
        0.0
    );
    let fog_tint = mix(scene.fog_color.rgb, scene.sun_color.rgb, sun_amount * 0.15);
    lit = mix(lit, fog_tint, fog);
    return vec4<f32>(lit, 1.0);
}
