struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) slot_id: f32,
}

@vertex
fn vs_main(model: VertexInput, @builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 0.0, 1.0);
    out.tex_coords = model.tex_coords;
    
    // Calculate which slot this vertex belongs to (4 vertices per slot)
    out.slot_id = f32(vertex_index / 4u);
    
    return out;
}

struct SlotUniform {
    selected_slot: u32,
}

struct SlotInventoryData {
    slot_data_1: vec4<u32>, // slots 0-3
    slot_data_2: vec4<u32>, // slots 4-7  
    slot_data_3: vec4<u32>, // slots 8-9 (z and w unused)
}

@group(0) @binding(0)
var<uniform> slot_uniform: SlotUniform;

@group(0) @binding(1)
var<uniform> inventory_data: SlotInventoryData;

@group(1) @binding(0)
var texture_atlas: texture_2d<f32>;

@group(1) @binding(1)
var atlas_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let slot_id = u32(in.slot_id);
    let selected_slot = slot_uniform.selected_slot;
    
    // Border thickness
    let border_thickness = 0.05;
    
    // Check if we're on the border
    let is_border = in.tex_coords.x < border_thickness || 
                   in.tex_coords.x > (1.0 - border_thickness) ||
                   in.tex_coords.y < border_thickness || 
                   in.tex_coords.y > (1.0 - border_thickness);
    
    if (is_border) {
        // Border color - bright white for selected slot, gray for others
        if (slot_id == selected_slot) {
            return vec4<f32>(1.0, 1.0, 1.0, 0.9); // Bright white border for selected
        } else {
            return vec4<f32>(0.4, 0.4, 0.4, 0.7); // Gray border for unselected
        }
    } else {
        // Interior - check if slot has a block
        var texture_id: u32 = 0u;
        if (slot_id < 4u) {
            texture_id = inventory_data.slot_data_1[slot_id];
        } else if (slot_id < 8u) {
            texture_id = inventory_data.slot_data_2[slot_id - 4u];
        } else {
            texture_id = inventory_data.slot_data_3[slot_id - 8u];
        }
        
        if (texture_id > 0u) {
            // Calculate texture coordinates in the atlas
            // Texture atlas is 4x4, so we have 16 textures total (0-15)
            let atlas_size = 4.0;
            let texture_x = f32(texture_id % 4u);
            let texture_y = f32(texture_id / 4u);
            
            // Map slot UV to texture UV within the atlas
            let inner_uv = (in.tex_coords - border_thickness) / (1.0 - 2.0 * border_thickness);
            let atlas_uv = vec2<f32>(
                (texture_x + inner_uv.x) / atlas_size,
                (texture_y + inner_uv.y) / atlas_size
            );
            
            // Sample the texture
            let texture_color = textureSample(texture_atlas, atlas_sampler, atlas_uv);
            return vec4<f32>(texture_color.rgb, 0.9);
        } else {
            // Empty slot - show background
            if (slot_id == selected_slot) {
                return vec4<f32>(0.3, 0.3, 0.3, 0.8); // Dark gray interior for selected
            } else {
                return vec4<f32>(0.1, 0.1, 0.1, 0.6); // Very dark interior for unselected
            }
        }
    }
}