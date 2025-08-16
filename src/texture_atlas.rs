use crate::texture_parser;

pub struct TextureAtlas {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
}

impl TextureAtlas {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        // Create a 4x4 texture atlas with loaded block textures
        // Each texture is 16x16 pixels for a total of 64x64 atlas
        let atlas_size = 64u32;
        let tile_size = 16u32;

        // Load textures from .texture files
        let loaded_textures = texture_parser::load_all_textures().unwrap_or_else(|e| {
            eprintln!("Failed to load textures: {}", e);
            std::collections::HashMap::new()
        });

        // Create the atlas data
        let mut atlas_data = vec![0u8; (atlas_size * atlas_size * 4) as usize]; // RGBA

        // Fill the atlas with loaded textures
        for tile_y in 0..4 {
            for tile_x in 0..4 {
                let texture_id = tile_y * 4 + tile_x;
                copy_texture_to_atlas(
                    &mut atlas_data,
                    atlas_size,
                    tile_x * tile_size,
                    tile_y * tile_size,
                    tile_size,
                    texture_id,
                    &loaded_textures,
                );
            }
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: atlas_size,
                height: atlas_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("Texture Atlas"),
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(atlas_size * 4),
                rows_per_image: Some(atlas_size),
            },
            wgpu::Extent3d {
                width: atlas_size,
                height: atlas_size,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest, // Pixel-perfect for Minecraft style
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("Texture Atlas Bind Group"),
        });

        Self {
            texture,
            view,
            sampler,
            bind_group,
        }
    }
}

// Copy loaded textures to atlas positions
fn copy_texture_to_atlas(
    atlas_data: &mut [u8],
    atlas_width: u32,
    start_x: u32,
    start_y: u32,
    size: u32,
    texture_id: u32,
    loaded_textures: &std::collections::HashMap<String, texture_parser::ParsedTexture>,
) {
    // Map texture IDs to texture file names
    let texture_name = match texture_id {
        0 => "stone",        // Stone
        1 => "dirt",         // Dirt
        2 => "grass_top",    // Grass Top
        3 => "grass_side",   // Grass Side
        4 => "sand",         // Sand
        5 => "water",        // Water
        6 => "wood_top",     // Wood Top
        7 => "wood_side",    // Wood Side
        8 => "leaves",       // Leaves
        9 => "snow",         // Snow
        10 => "bedrock",     // Bedrock
        11 => "planks",      // Planks
        12 => "cobblestone", // Cobblestone
        13 => "glass",       // Glass
        _ => "stone",        // Default to stone
    };

    // Get the loaded texture or use a fallback
    if let Some(texture) = loaded_textures.get(texture_name) {
        // Copy texture data to atlas
        for y in 0..size {
            for x in 0..size {
                let atlas_x = start_x + x;
                let atlas_y = start_y + y;
                let atlas_index = ((atlas_y * atlas_width + atlas_x) * 4) as usize;

                if x < texture.width && y < texture.height {
                    let texture_index = ((y * texture.width + x) * 4) as usize;

                    if atlas_index + 3 < atlas_data.len()
                        && texture_index + 3 < texture.pixels.len()
                    {
                        atlas_data[atlas_index] = texture.pixels[texture_index]; // R
                        atlas_data[atlas_index + 1] = texture.pixels[texture_index + 1]; // G
                        atlas_data[atlas_index + 2] = texture.pixels[texture_index + 2]; // B
                        atlas_data[atlas_index + 3] = texture.pixels[texture_index + 3];
                        // A
                    }
                }
            }
        }
    } else {
        // Fallback: generate a simple colored pattern if texture not found
        let (r, g, b, a) = match texture_id {
            0 => (128, 128, 128, 255), // Stone - gray
            1 => (139, 90, 43, 255),   // Dirt - brown
            _ => (255, 0, 255, 255),   // Magenta for missing textures
        };

        for y in 0..size {
            for x in 0..size {
                let atlas_x = start_x + x;
                let atlas_y = start_y + y;
                let atlas_index = ((atlas_y * atlas_width + atlas_x) * 4) as usize;

                if atlas_index + 3 < atlas_data.len() {
                    atlas_data[atlas_index] = r;
                    atlas_data[atlas_index + 1] = g;
                    atlas_data[atlas_index + 2] = b;
                    atlas_data[atlas_index + 3] = a; // Use proper alpha value
                }
            }
        }

        eprintln!(
            "Warning: Texture '{}' not found, using fallback color",
            texture_name
        );
    }
}
