struct CameraUniform {
    view_proj: mat4x4<f32>,
}

struct LightUniform {
    direction: vec3<f32>,
    color: vec3<f32>,
    intensity: f32,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> light: LightUniform;

@group(2) @binding(0)
var texture_atlas: texture_2d<f32>;
@group(2) @binding(1)
var texture_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) texture_id: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) texture_id: u32,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.world_position = model.position;
    out.normal = model.normal;
    out.texture_id = model.texture_id;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    return out;
}


// Calculate texture coordinates within the atlas
fn get_atlas_coords(tex_coords: vec2<f32>, texture_id: u32) -> vec2<f32> {
    let atlas_size = 4u; // 4x4 texture atlas
    let tile_size = 1.0 / f32(atlas_size);
    
    let tile_x = f32(texture_id % atlas_size);
    let tile_y = f32(texture_id / atlas_size);
    
    // Map texture coordinates to the correct tile in the atlas
    let atlas_x = (tile_x + tex_coords.x) * tile_size;
    let atlas_y = (tile_y + tex_coords.y) * tile_size;
    
    return vec2<f32>(atlas_x, atlas_y);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample from texture atlas
    let atlas_coords = get_atlas_coords(in.tex_coords, in.texture_id);
    let texture_color = textureSample(texture_atlas, texture_sampler, atlas_coords).rgb;
    
    // Use the actual surface normal from the vertex
    let normal = normalize(in.normal);
    let light_dir = normalize(-light.direction);
    
    // Calculate diffuse lighting with good ambient
    let diffuse_strength = max(dot(normal, light_dir), 0.0);
    let ambient = 0.3; // Ambient lighting
    let lighting = ambient + (1.0 - ambient) * diffuse_strength;
    
    // Apply lighting to the texture color
    let final_color = texture_color * lighting;
    
    return vec4<f32>(final_color, 1.0);
}