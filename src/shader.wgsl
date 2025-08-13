struct CameraUniform {
    view_proj: mat4x4<f32>,
}

struct LightUniform {
    direction: vec3<f32>,
    color: vec3<f32>,
    intensity: f32,
    light_space_matrix: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> light: LightUniform;

@group(2) @binding(0)
var shadow_texture: texture_depth_2d;
@group(2) @binding(1)
var shadow_sampler: sampler_comparison;

@group(3) @binding(0)
var texture_atlas: texture_2d<f32>;
@group(3) @binding(1)
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
    @location(2) light_space_position: vec4<f32>,
    @location(3) normal: vec3<f32>,
    @location(4) texture_id: u32,
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
    out.light_space_position = light.light_space_matrix * vec4<f32>(model.position, 1.0);
    return out;
}

fn shadow_calculation(light_space_pos: vec4<f32>) -> f32 {
    // Perform perspective divide
    var proj_coords = light_space_pos.xyz / light_space_pos.w;
    
    // Transform to [0,1] range
    proj_coords = proj_coords * 0.5 + 0.5;
    
    // If outside shadow map, assume not in shadow
    if (proj_coords.x < 0.0 || proj_coords.x > 1.0 || 
        proj_coords.y < 0.0 || proj_coords.y > 1.0 || 
        proj_coords.z > 1.0) {
        return 1.0;
    }
    
    // Get closest depth value from light's perspective
    let current_depth = proj_coords.z;
    
    // Check if current fragment is in shadow with reduced bias
    let bias = 0.002;
    let shadow = textureSampleCompare(shadow_texture, shadow_sampler, proj_coords.xy, current_depth - bias);
    
    // Return a value between 0.3 and 1.0 to avoid pure black shadows
    return mix(0.3, 1.0, shadow);
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
    
    // Calculate shadow
    let shadow = shadow_calculation(in.light_space_position);
    
    // Combine with minimum lighting to avoid pure black
    let final_lighting = max(lighting * shadow, 0.15);
    
    // Apply lighting to the texture color
    let final_color = texture_color * final_lighting;
    
    return vec4<f32>(final_color, 1.0);
}