struct LightUniform {
    direction: vec3<f32>,
    color: vec3<f32>,
    intensity: f32,
    light_space_matrix: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> light: LightUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
}

@vertex
fn vs_main(model: VertexInput) -> @builtin(position) vec4<f32> {
    return light.light_space_matrix * vec4<f32>(model.position, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0);
}