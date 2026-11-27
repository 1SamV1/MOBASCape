// Screen-space ambient occlusion (crates/client/src/ssao.rs).
//
// Pass 0 (`fs_depth`, half resolution): read the scene depth once per
// half-resolution pixel and store it as linear view distance. The occlusion
// test then reads this small single-sample texture, which is far kinder to
// the cache than scattered reads into the full-size (multisampled) buffer.
//
// Pass 1 (`fs_ao`, half resolution): rebuild the view-space position and
// normal of each pixel from the scene depth and test 12 points in the
// hemisphere above it. A point that the depth buffer says is buried behind
// nearer geometry is occluded. The hemisphere turns with a 4x4 interleaved
// pattern, so every 4x4 block of pixels together covers 16 rotations.
//
// Pass 2 (`fs_blur`, half resolution): average each 4x4 block, weighting
// neighbours by how close their depth is, which removes the pattern without
// bleeding shade across silhouettes. It writes the final multiplier the post
// pass applies to the lit scene: mix(1, ao, strength).
//
// DEPTH_TYPE is replaced when the shader is built: texture_depth_2d, or
// texture_depth_multisampled_2d under MSAA (sample 0 is read).

struct SsaoParams {
    proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    // x: radius (world units), y: strength, z: depth bias, w: contrast power
    params: vec4<f32>,
    // xy: scene depth size in pixels, zw: AO target size in pixels
    size: vec4<f32>,
};

@group(0) @binding(0) var<uniform> p: SsaoParams;
@group(0) @binding(1) var depth_tex: DEPTH_TYPE;
// The previous pass's output: linear depth for `fs_ao`, AO for `fs_blur`.
@group(0) @binding(2) var ao_tex: texture_2d<f32>;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let q = positions[vertex_index];
    var output: VsOut;
    output.clip = vec4<f32>(q, 1.0, 1.0);
    output.uv = q * vec2<f32>(0.5, -0.5) + 0.5;
    return output;
}

fn load_depth(c: vec2<i32>) -> f32 {
    let size = vec2<i32>(p.size.xy);
    return textureLoad(depth_tex, clamp(c, vec2<i32>(0), size - 1), 0);
}

@fragment
fn fs_depth(input: VsOut) -> @location(0) vec4<f32> {
    let c = vec2<i32>(floor(input.uv * p.size.xy));
    let depth = load_depth(c);
    if (depth >= 1.0) {
        return vec4<f32>(1e6);
    }
    let v = p.inv_proj * vec4<f32>(0.0, 0.0, depth, 1.0);
    return vec4<f32>(-v.z / v.w);
}

// Linear view distance at a half-resolution pixel (1e6 = sky).
fn linear_at(c: vec2<i32>) -> f32 {
    let size = vec2<i32>(p.size.zw);
    return textureLoad(ao_tex, clamp(c, vec2<i32>(0), size - 1), 0).r;
}

// View-space position from a screen uv (0..1, y down) and linear distance.
// For a right-handed perspective, x_ndc = proj00 * x / -z.
fn view_from(uv: vec2<f32>, dist: f32) -> vec3<f32> {
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    return vec3<f32>(ndc / vec2<f32>(p.proj[0][0], p.proj[1][1]) * dist, -dist);
}

fn view_at(c: vec2<i32>) -> vec3<f32> {
    let uv = (vec2<f32>(c) + 0.5) / p.size.zw;
    return view_from(uv, linear_at(c));
}

// 4x4 ordered pattern, 0..15.
fn bayer4(c: vec2<i32>) -> f32 {
    let x = u32(c.x) & 3u;
    let y = u32(c.y) & 3u;
    var m = array<u32, 16>(0u, 8u, 2u, 10u, 12u, 4u, 14u, 6u, 3u, 11u, 1u, 9u, 15u, 7u, 13u, 5u);
    return f32(m[y * 4u + x]);
}

const SAMPLES: u32 = 12u;

@fragment
fn fs_ao(input: VsOut) -> @location(0) vec4<f32> {
    let half_c = vec2<i32>(input.clip.xy);
    let c = half_c;
    if (linear_at(c) >= 1e5) {
        return vec4<f32>(1.0, 1e6, 0.0, 1.0);
    }
    let P = view_at(c);

    // Normal from the neighbours on the flatter side of each axis, so a
    // pixel on a silhouette doesn't take its slope from the far background.
    let pr = view_at(c + vec2<i32>(1, 0));
    let pl = view_at(c - vec2<i32>(1, 0));
    let pd = view_at(c + vec2<i32>(0, 1));
    let pu = view_at(c - vec2<i32>(0, 1));
    let dx = select(pr - P, P - pl, abs(pl.z - P.z) < abs(pr.z - P.z));
    let dy = select(pd - P, P - pu, abs(pu.z - P.z) < abs(pd.z - P.z));
    var N = normalize(cross(dy, dx));
    if (dot(N, -P) < 0.0) {
        N = -N;
    }

    // Turn the kernel about the normal by the pattern angle.
    let angle = (bayer4(half_c) + 0.5) / 16.0 * 6.2831853;
    let seed = vec3<f32>(cos(angle), sin(angle), 0.0);
    var T = seed - N * dot(seed, N);
    if (dot(T, T) < 1e-4) {
        T = vec3<f32>(0.0, 0.0, 1.0) - N * N.z;
    }
    T = normalize(T);
    let B = cross(N, T);

    let radius = p.params.x;
    let bias = p.params.z;
    var occlusion = 0.0;
    for (var i = 0u; i < SAMPLES; i = i + 1u) {
        // A spiral over the hemisphere, denser near the centre so close
        // contact shades more than distant cover.
        let fi = f32(i);
        let t = (fi + 0.5) / f32(SAMPLES);
        let a = fi * 2.3999632;
        let r = sqrt(t);
        let h = sqrt(max(1.0 - t, 0.0));
        let scale = mix(0.15, 1.0, t * t);
        let k = vec3<f32>(cos(a) * r, sin(a) * r, h) * scale;
        let S = P + (T * k.x + B * k.y + N * k.z) * radius;

        let ndc = vec2<f32>(p.proj[0][0], p.proj[1][1]) * S.xy / -S.z;
        let suv = ndc * vec2<f32>(0.5, -0.5) + 0.5;
        if (any(suv < vec2<f32>(0.0)) || any(suv > vec2<f32>(1.0))) {
            continue;
        }
        let scene_z = -linear_at(vec2<i32>(suv * p.size.zw));
        // View space looks down -z: a surface in front of the sample point
        // has the larger z. Fade occluders far in front (a tree canopy over
        // open ground is not contact).
        let range = smoothstep(0.0, 1.0, radius / max(abs(P.z - scene_z), 1e-3));
        occlusion += select(0.0, 1.0, scene_z >= S.z + bias) * range;
    }
    let ao = pow(clamp(1.0 - occlusion / f32(SAMPLES), 0.0, 1.0), p.params.w);
    return vec4<f32>(ao, -P.z, 0.0, 1.0);
}

@fragment
fn fs_blur(input: VsOut) -> @location(0) vec4<f32> {
    let c = vec2<i32>(input.clip.xy);
    let size = vec2<i32>(p.size.zw);
    let centre = textureLoad(ao_tex, c, 0).rg;
    if (centre.g >= 1e5) {
        return vec4<f32>(1.0);
    }
    var sum = 0.0;
    var weight = 0.0;
    for (var y = -2; y < 2; y = y + 1) {
        for (var x = -2; x < 2; x = x + 1) {
            let s = textureLoad(ao_tex, clamp(c + vec2<i32>(x, y), vec2<i32>(0), size - 1), 0).rg;
            let w = max(0.0, 1.0 - abs(s.g - centre.g) / (centre.g * 0.04 + 8.0));
            sum += s.r * w;
            weight += w;
        }
    }
    let ao = select(centre.r, sum / weight, weight > 1e-3);
    return vec4<f32>(mix(1.0, ao, p.params.y));
}
