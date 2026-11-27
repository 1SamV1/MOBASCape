struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var scene_texture: texture_2d<f32>;
@group(0) @binding(1) var bloom_texture: texture_2d<f32>;
@group(0) @binding(2) var scene_sampler: sampler;
// Ambient occlusion multiplier, half resolution (crates/client/src/ssao.rs).
@group(0) @binding(3) var ao_texture: texture_2d<f32>;

@vertex
fn vs(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let p = positions[vertex_index];
    var output: VsOut;
    output.clip = vec4<f32>(p, 1.0, 1.0);
    output.uv = p * vec2<f32>(0.5, -0.5) + 0.5;
    return output;
}

fn tonemap(c: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c2 = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((c * (a * c + b)) / (c * (c2 * c + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn composite(uv: vec2<f32>) -> vec3<f32> {
    let ao = textureSample(ao_texture, scene_sampler, uv).r;
    let scene = textureSample(scene_texture, scene_sampler, uv).rgb * ao;
    let bloom = textureSample(bloom_texture, scene_sampler, uv).rgb;
    // Bloom is a glint on metal and magic, not a neon halo around every
    // sword. The reference client has almost no visible bloom.
    return tonemap(scene + bloom * 0.24);
}

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

// Console-style FXAA on the tonemapped composite.
fn fxaa(uv: vec2<f32>, texel: vec2<f32>) -> vec3<f32> {
    let rgbM = composite(uv);
    let lumaM = luma(rgbM);
    let lumaNW = luma(composite(uv + vec2<f32>(-1.0, -1.0) * texel));
    let lumaNE = luma(composite(uv + vec2<f32>(1.0, -1.0) * texel));
    let lumaSW = luma(composite(uv + vec2<f32>(-1.0, 1.0) * texel));
    let lumaSE = luma(composite(uv + vec2<f32>(1.0, 1.0) * texel));

    let lumaMin = min(lumaM, min(min(lumaNW, lumaNE), min(lumaSW, lumaSE)));
    let lumaMax = max(lumaM, max(max(lumaNW, lumaNE), max(lumaSW, lumaSE)));
    let range = lumaMax - lumaMin;
    if (range < max(0.0312, lumaMax * 0.125)) {
        return rgbM;
    }

    var dir = vec2<f32>(
        -((lumaNW + lumaNE) - (lumaSW + lumaSE)),
        (lumaNW + lumaSW) - (lumaNE + lumaSE),
    );
    let dirReduce = max((lumaNW + lumaNE + lumaSW + lumaSE) * 0.03125, 0.0078125);
    let rcpDirMin = 1.0 / (min(abs(dir.x), abs(dir.y)) + dirReduce);
    dir = clamp(dir * rcpDirMin, vec2<f32>(-8.0), vec2<f32>(8.0)) * texel;

    let rgbA = 0.5 * (
        composite(uv + dir * (1.0 / 3.0 - 0.5))
        + composite(uv + dir * (2.0 / 3.0 - 0.5))
    );
    let rgbB = rgbA * 0.5 + 0.25 * (
        composite(uv - dir)
        + composite(uv + dir)
    );
    let lumaB = luma(rgbB);
    if (lumaB < lumaMin || lumaB > lumaMax) {
        return rgbA;
    }
    return rgbB;
}

fn grade(uv: vec2<f32>, base: vec3<f32>, dimensions: vec2<f32>) -> vec3<f32> {
    var color = base;

    // ---- Reference grade -------------------------------------------------
    // The clip is graded like a foggy late-afternoon painting: low saturation,
    // warm highlights, cool lifted shadows and a heavy edge falloff. The old
    // pass only tinted a few channels and left the highlighter greens intact.
    // Neutral white balance: a warm multiply here turned every green olive
    // (2026-11-24 study: reference grass 100-115 degrees, ours 73).
    color *= 0.98;

    // Saturation for the whole frame, so props and effects stay
    // consistent with the terrain.
    // 2026-11-24: the reference sits near 0.6 saturation on grass; with
    // the old channel-clipping contrast gone the frame needs a lift, not a
    // cut.
    let l0 = luma(color);
    color = max(mix(vec3<f32>(l0), color, 1.12), vec3<f32>(0.0));

    // Split tone: shadow blue-grey, highlight warm cream.
    let l1 = luma(color);
    let split = mix(
        vec3<f32>(0.94, 0.98, 1.03),
        vec3<f32>(1.03, 1.01, 0.97),
        smoothstep(0.10, 0.62, l1)
    );
    color *= split;

    // A touch of warm bounce in the darkest values so silhouettes keep the
    // olive cast of the reference instead of crushing to pure black.
    color += vec3<f32>(0.018, 0.017, 0.011) * (1.0 - smoothstep(0.0, 0.34, l1));

    // Contrast deepens the forest floor and lifts the lit lanes.
    // The reference spreads values wide (10th to 90th percentile 0.10 to
    // 0.45 in value); ours sat at 0.19 to 0.37 before this.
    // Applied to luma only, so the stretch keeps each colour's hue and
    // chroma ratios instead of clipping a channel (which pushed every
    // green to full saturation).
    // The stretch runs on the square root of luma (close to display
    // values; the frame here is linear), pivoting at mid grass.
    let lc = max(luma(color), 1e-4);
    let perceived = clamp((sqrt(lc) - 0.30) * 1.30 + 0.30, 0.0, 1.0);
    color = clamp(color * (perceived * perceived / lc), vec3<f32>(0.0), vec3<f32>(1.0));

    // Heavy painterly vignette: the corners fade into the same fog colour as
    // the horizon, pulling the eye to the centre of the fight.
    let centred = uv - vec2<f32>(0.5);
    let vignette = 1.0 - smoothstep(0.28, 1.00, length(centred * vec2<f32>(1.0, 1.22)));
    color *= mix(0.86, 1.0, vignette);

    let dither = fract(sin(dot(uv * dimensions, vec2<f32>(12.9898, 78.233))) * 43758.5453);
    color += (dither - 0.5) / 255.0;
    return clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs(input: VsOut) -> @location(0) vec4<f32> {
    let dimensions = vec2<f32>(textureDimensions(scene_texture));
    let texel = 1.0 / dimensions;
    let color = fxaa(input.uv, texel);
    return vec4<f32>(grade(input.uv, color, dimensions), 1.0);
}

/// The 4x MSAA path has already resolved its edges; skip the FXAA taps.
@fragment
fn fs_no_fxaa(input: VsOut) -> @location(0) vec4<f32> {
    let dimensions = vec2<f32>(textureDimensions(scene_texture));
    let color = composite(input.uv);
    return vec4<f32>(grade(input.uv, color, dimensions), 1.0);
}
