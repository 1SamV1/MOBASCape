// The interface pass: quads over one atlas, drawn after the scene.
//
// The atlas is read by whole pixel, never blended between neighbours, so an
// image drawn at a whole multiple of its size keeps hard edges.

@group(0) @binding(0) var atlas: texture_2d<f32>;

struct VsIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) colour: vec4<f32>,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) colour: vec4<f32>,
};

@vertex
fn vs(input: VsIn) -> VsOut {
    var output: VsOut;
    output.clip = vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;
    output.colour = input.colour;
    return output;
}

@fragment
fn fs(input: VsOut) -> @location(0) vec4<f32> {
    let texel = textureLoad(atlas, vec2<i32>(floor(input.uv)), 0);
    return texel * input.colour;
}
