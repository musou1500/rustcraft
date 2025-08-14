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
        // Create a simple 4x4 texture atlas with Minecraft-like block textures
        // Each texture is 16x16 pixels for a total of 64x64 atlas
        let atlas_size = 64u32;
        let tile_size = 16u32;

        // Generate procedural textures for each block type
        let mut atlas_data = vec![0u8; (atlas_size * atlas_size * 4) as usize]; // RGBA

        // Fill the atlas with different textures
        for tile_y in 0..4 {
            for tile_x in 0..4 {
                let texture_id = tile_y * 4 + tile_x;
                generate_texture(
                    &mut atlas_data,
                    atlas_size,
                    tile_x * tile_size,
                    tile_y * tile_size,
                    tile_size,
                    texture_id,
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

// Generate procedural textures for different block types
fn generate_texture(
    atlas_data: &mut [u8],
    atlas_width: u32,
    start_x: u32,
    start_y: u32,
    size: u32,
    texture_id: u32,
) {
    for y in 0..size {
        for x in 0..size {
            let atlas_x = start_x + x;
            let atlas_y = start_y + y;
            let index = ((atlas_y * atlas_width + atlas_x) * 4) as usize;

            let (r, g, b) = match texture_id {
                0 => generate_stone_texture(x, y, size),      // Stone
                1 => generate_dirt_texture(x, y, size),       // Dirt
                2 => generate_grass_top_texture(x, y, size),  // Grass Top
                3 => generate_grass_side_texture(x, y, size), // Grass Side
                4 => generate_sand_texture(x, y, size),       // Sand
                5 => generate_water_texture(x, y, size),      // Water
                6 => generate_wood_top_texture(x, y, size),   // Wood Top
                7 => generate_wood_side_texture(x, y, size),  // Wood Side
                8 => generate_leaves_texture(x, y, size),     // Leaves
                9 => generate_coal_texture(x, y, size),       // Coal
                10 => generate_iron_texture(x, y, size),      // Iron
                11 => generate_gold_texture(x, y, size),      // Gold
                12 => generate_snow_texture(x, y, size),      // Snow
                _ => (128, 128, 128),                         // Default gray
            };

            if index + 3 < atlas_data.len() {
                atlas_data[index] = r;
                atlas_data[index + 1] = g;
                atlas_data[index + 2] = b;
                atlas_data[index + 3] = 255; // Alpha
            }
        }
    }
}

// Texture generation functions for different block types
fn generate_stone_texture(x: u32, y: u32, _size: u32) -> (u8, u8, u8) {
    let noise = ((x * 7 + y * 13) % 31) as f32 / 31.0;
    let base = 120;
    let variation = (noise * 40.0) as u8;
    (base + variation, base + variation, base + variation + 10)
}

fn generate_dirt_texture(x: u32, y: u32, _size: u32) -> (u8, u8, u8) {
    let noise = ((x * 11 + y * 17) % 23) as f32 / 23.0;
    let r = (100.0 + noise * 50.0) as u8;
    let g = (65.0 + noise * 35.0) as u8;
    let b = (35.0 + noise * 20.0) as u8;
    (r, g, b)
}

fn generate_grass_top_texture(x: u32, y: u32, _size: u32) -> (u8, u8, u8) {
    let noise = ((x * 5 + y * 7) % 19) as f32 / 19.0;
    let r = (30.0 + noise * 40.0) as u8;
    let g = (120.0 + noise * 60.0) as u8;
    let b = (30.0 + noise * 40.0) as u8;
    (r, g, b)
}

fn generate_grass_side_texture(x: u32, y: u32, size: u32) -> (u8, u8, u8) {
    if y < size / 4 {
        // Top portion - grass
        generate_grass_top_texture(x, y, size)
    } else {
        // Bottom portion - dirt
        generate_dirt_texture(x, y, size)
    }
}

fn generate_sand_texture(x: u32, y: u32, _size: u32) -> (u8, u8, u8) {
    let noise = ((x * 13 + y * 19) % 29) as f32 / 29.0;
    let r = (220.0 + noise * 35.0) as u8;
    let g = (195.0 + noise * 30.0) as u8;
    let b = (140.0 + noise * 25.0) as u8;
    (r, g, b)
}

fn generate_water_texture(x: u32, y: u32, _size: u32) -> (u8, u8, u8) {
    let noise = ((x * 3 + y * 5) % 13) as f32 / 13.0;
    let r = (30.0 + noise * 30.0) as u8;
    let g = (100.0 + noise * 40.0) as u8;
    let b = (200.0 + noise * 55.0) as u8;
    (r, g, b)
}

fn generate_wood_top_texture(x: u32, y: u32, size: u32) -> (u8, u8, u8) {
    let center_x = size as f32 / 2.0;
    let center_y = size as f32 / 2.0;
    let dist = ((x as f32 - center_x).powi(2) + (y as f32 - center_y).powi(2)).sqrt();
    let ring = (dist * 2.0) as u32 % 3;
    let base_r = 140;
    let base_g = 85;
    let base_b = 50;
    match ring {
        0 => (base_r, base_g, base_b),
        1 => (base_r - 20, base_g - 15, base_b - 10),
        _ => (base_r + 15, base_g + 10, base_b + 5),
    }
}

fn generate_wood_side_texture(x: u32, y: u32, _size: u32) -> (u8, u8, u8) {
    let stripe = (x / 2) % 3;
    let base_r = 120;
    let base_g = 75;
    let base_b = 45;
    let noise = ((x * 7 + y * 11) % 17) as f32 / 17.0 * 20.0;
    match stripe {
        0 => (
            (base_r as f32 + noise) as u8,
            (base_g as f32 + noise * 0.7) as u8,
            (base_b as f32 + noise * 0.5) as u8,
        ),
        1 => ((base_r - 15) as u8, (base_g - 10) as u8, (base_b - 8) as u8),
        _ => ((base_r + 10) as u8, (base_g + 8) as u8, (base_b + 5) as u8),
    }
}

fn generate_leaves_texture(x: u32, y: u32, _size: u32) -> (u8, u8, u8) {
    let noise = ((x * 9 + y * 15) % 21) as f32 / 21.0;
    let r = (25.0 + noise * 35.0) as u8;
    let g = (100.0 + noise * 70.0) as u8;
    let b = (25.0 + noise * 35.0) as u8;
    (r, g, b)
}

fn generate_coal_texture(x: u32, y: u32, _size: u32) -> (u8, u8, u8) {
    let noise = ((x * 17 + y * 23) % 31) as f32 / 31.0;
    let brightness = (20.0 + noise * 40.0) as u8;
    (brightness, brightness, brightness.saturating_add(10))
}

fn generate_iron_texture(x: u32, y: u32, _size: u32) -> (u8, u8, u8) {
    let noise = ((x * 6 + y * 8) % 15) as f32 / 15.0;
    let r = (160.0 + noise * 50.0) as u8;
    let g = (160.0 + noise * 50.0) as u8;
    let b = (170.0 + noise * 55.0) as u8;
    (r, g, b)
}

fn generate_gold_texture(x: u32, y: u32, _size: u32) -> (u8, u8, u8) {
    let noise = ((x * 4 + y * 6) % 11) as f32 / 11.0;
    let r = (240.0 + noise * 15.0) as u8;
    let g = (200.0 + noise * 35.0) as u8;
    let b = (20.0 + noise * 30.0) as u8;
    (r, g, b)
}

fn generate_snow_texture(x: u32, y: u32, _size: u32) -> (u8, u8, u8) {
    let noise = ((x * 2 + y * 3) % 7) as f32 / 7.0;
    let brightness = (235.0 + noise * 20.0) as u8;
    (brightness, brightness, brightness.saturating_add(5))
}
