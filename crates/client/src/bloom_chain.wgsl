// Mip-chain bloom (crates/client/src/bloom_chain.rs), after Jimenez's
// "Next Generation Post Processing in Call of Duty: Advanced Warfare"
// (SIGGRAPH 2014).
//
// `fs_down` halves the image with a 13-tap filter (five overlapping 2x2
// boxes). On the first step (`params.w` = 1) each box is weighted by
// 1 / (1 + luma) — the Karis average — so one lone bright pixel can't
// sparkle and flicker as it crosses texels.
//
// `fs_up` doubles the image back with a 3x3 tent filter and is drawn with
// additive blending onto the level above, so each level adds a wider,
// softer ring of glow. `params.z` scales what it adds.

struct ChainParams {
    // xy: source texel size, z: weight (up), w: 1 = Karis average (down)
    params: vec4<f32>,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> chain: ChainParams;

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

fn tap(uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(source, source_sampler, uv, 0.0).rgb;
}

fn karis(c: vec3<f32>) -> f32 {
    return 1.0 / (1.0 + dot(c, vec3<f32>(0.2126, 0.7152, 0.0722)));
}

@fragment
fn fs_down(input: VsOut) -> @location(0) vec4<f32> {
    let t = chain.params.xy;
    let uv = input.uv;
    let a = tap(uv + t * vec2<f32>(-2.0, -2.0));
    let b = tap(uv + t * vec2<f32>(0.0, -2.0));
    let c = tap(uv + t * vec2<f32>(2.0, -2.0));
    let d = tap(uv + t * vec2<f32>(-2.0, 0.0));
    let e = tap(uv);
    let f = tap(uv + t * vec2<f32>(2.0, 0.0));
    let g = tap(uv + t * vec2<f32>(-2.0, 2.0));
    let h = tap(uv + t * vec2<f32>(0.0, 2.0));
    let i = tap(uv + t * vec2<f32>(2.0, 2.0));
    let j = tap(uv + t * vec2<f32>(-1.0, -1.0));
    let k = tap(uv + t * vec2<f32>(1.0, -1.0));
    let l = tap(uv + t * vec2<f32>(-1.0, 1.0));
    let m = tap(uv + t * vec2<f32>(1.0, 1.0));

    // Five boxes: the centre one counts half, the four corners an eighth.
    let centre = (j + k + l + m) * 0.25;
    let tl = (a + b + d + e) * 0.25;
    let tr = (b + c + e + f) * 0.25;
    let bl = (d + e + g + h) * 0.25;
    let br = (e + f + h + i) * 0.25;
    if (chain.params.w > 0.5) {
        let wc = 0.5 * karis(centre);
        let w1 = 0.125 * karis(tl);
        let w2 = 0.125 * karis(tr);
        let w3 = 0.125 * karis(bl);
        let w4 = 0.125 * karis(br);
        let sum = centre * wc + tl * w1 + tr * w2 + bl * w3 + br * w4;
        return vec4<f32>(sum / max(wc + w1 + w2 + w3 + w4, 1e-4), 1.0);
    }
    return vec4<f32>(centre * 0.5 + (tl + tr + bl + br) * 0.125, 1.0);
}

@fragment
fn fs_up(input: VsOut) -> @location(0) vec4<f32> {
    let t = chain.params.xy;
    let uv = input.uv;
    var sum = tap(uv) * 4.0;
    sum += (tap(uv + vec2<f32>(-t.x, 0.0)) + tap(uv + vec2<f32>(t.x, 0.0))
        + tap(uv + vec2<f32>(0.0, -t.y)) + tap(uv + vec2<f32>(0.0, t.y))) * 2.0;
    sum += tap(uv + vec2<f32>(-t.x, -t.y)) + tap(uv + vec2<f32>(t.x, -t.y))
        + tap(uv + vec2<f32>(-t.x, t.y)) + tap(uv + vec2<f32>(t.x, t.y));
    return vec4<f32>(sum / 16.0 * chain.params.z, 1.0);
}
