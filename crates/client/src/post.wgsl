struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var scene_texture: texture_2d<f32>;
@group(0) @binding(1) var bloom_texture: texture_2d<f32>;
@group(0) @binding(2) var scene_sampler: sampler;

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
    let scene = textureSample(scene_texture, scene_sampler, uv).rgb;
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
    color *= 0.94;
    color *= vec3<f32>(1.03, 1.0, 0.99);

    // Desaturate the whole frame. Terrain is the main offender; grading it
    // here keeps props and effects consistent with it.
    let l0 = luma(color);
    color = mix(vec3<f32>(l0), color, 0.88);

    // Split tone: shadow blue-grey, highlight warm cream.
    let l1 = luma(color);
    let split = mix(
        vec3<f32>(0.92, 0.97, 1.03),
        vec3<f32>(1.06, 1.02, 0.92),
        smoothstep(0.10, 0.62, l1)
    );
    color *= split;

    // A touch of warm bounce in the darkest values so silhouettes keep the
    // olive cast of the reference instead of crushing to pure black.
    color += vec3<f32>(0.018, 0.017, 0.011) * (1.0 - smoothstep(0.0, 0.34, l1));

    // Filmic contrast around a low pivot deepens the forest floor.
    color = clamp((color - 0.42) * 1.06 + 0.42, vec3<f32>(0.0), vec3<f32>(1.0));

    // Heavy painterly vignette: the corners fade into the same fog colour as
    // the horizon, pulling the eye to the centre of the fight.
    let centred = uv - vec2<f32>(0.5);
    let vignette = 1.0 - smoothstep(0.28, 1.00, length(centred * vec2<f32>(1.0, 1.22)));
    color *= mix(0.72, 1.0, vignette);

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
