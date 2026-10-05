struct Camera { vp: mat4x4<f32> }
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var color_texture: texture_2d<f32>;
@group(1) @binding(1) var color_sampler: sampler;
struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
}
@vertex fn vs(@location(0) position: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) uv: vec2<f32>) -> Output {
    var out: Output;
    out.position = camera.vp * vec4<f32>(position, 1.0);
    out.normal = normal;
    out.uv = uv;
    return out;
}
@fragment fn fs(input: Output, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let texel = textureSample(color_texture, color_sampler, input.uv);
    if texel.a < 0.02 { discard; }
    let normal = normalize(input.normal) * select(-1.0, 1.0, front);
    let light = normalize(vec3<f32>(-0.4, 0.9, -0.5));
    let diffuse = 0.42 + 0.58 * max(dot(normal, light), 0.0);
    return vec4<f32>(texel.rgb * diffuse, texel.a);
}
