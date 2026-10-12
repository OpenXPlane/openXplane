@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;
struct Output { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vs(@builtin(vertex_index) index: u32) -> Output {
    let positions = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    var out: Output;
    out.position = vec4(positions[index], 0.0, 1.0);
    out.uv = vec2((positions[index].x + 1.0) * 0.5, (1.0 - positions[index].y) * 0.5);
    return out;
}
@fragment fn fs(input: Output) -> @location(0) vec4<f32> { return textureSample(image, image_sampler, input.uv); }
