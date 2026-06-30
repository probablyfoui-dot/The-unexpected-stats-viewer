struct Uniforms {
    mvp: mat4x4<f32>,
    light_direction: vec3<f32>,
    ambient: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var skin_texture: texture_2d<f32>;

@group(0) @binding(2)
var skin_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) normal: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.mvp * vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    out.normal = in.normal;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(skin_texture, skin_sampler, in.uv);
    if color.a < 0.1 {
        discard;
    }

    let normal = normalize(in.normal);
    let light = normalize(uniforms.light_direction);
    let diffuse = max(dot(normal, light), 0.0);
    let brightness = uniforms.ambient + (1.0 - uniforms.ambient) * diffuse;

    return vec4<f32>(color.rgb * brightness, color.a);
}
