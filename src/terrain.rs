use wgpu::util::DeviceExt;
use noise::{NoiseFn, Perlin};
use crate::voxel::{Vertex, create_cube_vertices_selective, create_cube_indices_selective};
use crate::blocks::{BlockType, get_block_registry, generation};
use crate::structures::{StructureGenerator, PlacedStructure};
use std::collections::HashMap;
use cgmath::Point3;
use rayon::prelude::*;

const CHUNK_SIZE: usize = 16;
const WORLD_HEIGHT: usize = 24; // Reduced height for gentler terrain
const BASE_HEIGHT: usize = 8;   // Minimum terrain height
const RENDER_DISTANCE: i32 = 4;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

/// Raw chunk data that can be generated concurrently
pub struct ChunkData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

pub struct Chunk {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
}

pub struct TerrainProgress {
    pub total_chunks: usize,
    pub completed_chunks: usize,
    pub is_generating: bool,
}

impl TerrainProgress {
    pub fn new() -> Self {
        Self {
            total_chunks: 0,
            completed_chunks: 0,
            is_generating: false,
        }
    }
    
    pub fn get_progress(&self) -> f32 {
        if self.total_chunks == 0 {
            0.0
        } else {
            self.completed_chunks as f32 / self.total_chunks as f32
        }
    }
}

pub struct Terrain {
    chunks: HashMap<ChunkPos, Chunk>,
    height_noise: Perlin,
    biome_noise: Perlin,
    ore_noise: Perlin,
    texture_noise: Perlin,
    pub progress: TerrainProgress,
    // Structure generator for trees and houses
    structure_generator: StructureGenerator,
    // Cache the actual block data for each chunk - this is the single source of truth
    chunk_blocks: HashMap<ChunkPos, [[[BlockType; WORLD_HEIGHT]; CHUNK_SIZE]; CHUNK_SIZE]>,
}

impl Terrain {
    pub fn new(_device: &wgpu::Device) -> Self {
        let height_noise = Perlin::new(42);
        let biome_noise = Perlin::new(1337);
        let ore_noise = Perlin::new(9999);
        let texture_noise = Perlin::new(5555);
        let chunks = HashMap::new();
        let structure_generator = StructureGenerator::new(7777);
        
        Self {
            chunks,
            height_noise,
            biome_noise,
            ore_noise,
            texture_noise,
            progress: TerrainProgress::new(),
            structure_generator,
            chunk_blocks: HashMap::new(),
        }
    }

    fn generate_chunk_data(&self, chunk_pos: ChunkPos, structures: &[PlacedStructure]) -> (ChunkData, [[[BlockType; WORLD_HEIGHT]; CHUNK_SIZE]; CHUNK_SIZE]) {
        let mut vertices = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        let registry = get_block_registry();

        // Pre-generate block data for the entire chunk to enable face culling
        let mut chunk_blocks = [[[BlockType::Air; WORLD_HEIGHT]; CHUNK_SIZE]; CHUNK_SIZE];
        
        // Pre-compute noise values for the entire chunk in batches
        let mut height_values = vec![vec![0usize; CHUNK_SIZE]; CHUNK_SIZE];
        let mut biome_values = vec![vec![0.0f64; CHUNK_SIZE]; CHUNK_SIZE];
        
        // Compute height and biome noise in parallel
        let noise_data: Vec<(usize, usize, usize, f64)> = (0..CHUNK_SIZE)
            .into_par_iter()
            .flat_map(|x| {
                (0..CHUNK_SIZE).into_par_iter().map(move |z| {
                    let world_x = (chunk_pos.x * CHUNK_SIZE as i32 + x as i32) as f32;
                    let world_z = (chunk_pos.z * CHUNK_SIZE as i32 + z as i32) as f32;
                    
                    // Height noise computation
                    let scale1 = 0.02;
                    let scale2 = 0.05;
                    let scale3 = 0.1;
                    
                    let height_noise1 = self.height_noise.get([world_x as f64 * scale1, world_z as f64 * scale1]);
                    let height_noise2 = self.height_noise.get([world_x as f64 * scale2, world_z as f64 * scale2]);
                    let height_noise3 = self.height_noise.get([world_x as f64 * scale3, world_z as f64 * scale3]);
                    
                    let combined_noise = height_noise1 * 0.6 + height_noise2 * 0.3 + height_noise3 * 0.1;
                    let height_variation = (WORLD_HEIGHT - BASE_HEIGHT) as f64;
                    let height = BASE_HEIGHT + ((combined_noise + 1.0) * 0.5 * height_variation) as usize;
                    
                    // Biome noise computation
                    let biome_noise_val = self.biome_noise.get([world_x as f64 * 0.01, world_z as f64 * 0.01]);
                    
                    (x, z, height, biome_noise_val)
                })
            })
            .collect();
            
        // Store the computed values
        for (x, z, height, biome_noise_val) in noise_data {
            height_values[x][z] = height;
            biome_values[x][z] = biome_noise_val;
        }
        
        // Generate block types using pre-computed noise
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let height = height_values[x][z];
                let biome_noise_val = biome_values[x][z];
                let world_x = (chunk_pos.x * CHUNK_SIZE as i32 + x as i32) as f32;
                let world_z = (chunk_pos.z * CHUNK_SIZE as i32 + z as i32) as f32;
                
                // Process all Y levels in the chunk, not just natural terrain height
                for y in 0..WORLD_HEIGHT {
                    // Generate terrain blocks (removed/placed blocks will be handled after generation)
                    if y < height.min(WORLD_HEIGHT) {
                        // Only generate natural terrain within the natural height
                        let ore_noise_val = self.ore_noise.get([world_x as f64 * 0.2, y as f64 * 0.3, world_z as f64 * 0.2]);
                        let base_block = generation::get_block_for_height(height, WORLD_HEIGHT, y, ore_noise_val);
                        let block_type = generation::get_biome_block(base_block, biome_noise_val, height, y);
                        chunk_blocks[x][z][y] = block_type;
                    } else {
                        // Above natural terrain height - default to air
                        chunk_blocks[x][z][y] = BlockType::Air;
                    }
                }
            }
        }
        
        // Place structure blocks into the chunk
        for structure in structures {
            for block in &structure.blocks {
                let block_x = structure.world_x + block.relative_pos.0;
                let block_y = structure.world_y + block.relative_pos.1;
                let block_z = structure.world_z + block.relative_pos.2;
                
                // Check if this block is within the current chunk
                let local_x = block_x - (chunk_pos.x * CHUNK_SIZE as i32);
                let local_z = block_z - (chunk_pos.z * CHUNK_SIZE as i32);
                
                if local_x >= 0 && local_x < CHUNK_SIZE as i32 &&
                   local_z >= 0 && local_z < CHUNK_SIZE as i32 &&
                   block_y >= 0 && block_y < WORLD_HEIGHT as i32 {
                    // Place structure blocks
                    chunk_blocks[local_x as usize][local_z as usize][block_y as usize] = block.block_type;
                }
            }
        }

        // Generate vertices with face culling
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for y in 0..WORLD_HEIGHT {
                    let block_type = chunk_blocks[x][z][y];
                    
                    // Skip air blocks
                    if block_type == BlockType::Air {
                        continue;
                    }
                    
                    let world_x = (chunk_pos.x * CHUNK_SIZE as i32 + x as i32) as f32;
                    let world_z = (chunk_pos.z * CHUNK_SIZE as i32 + z as i32) as f32;
                    
                    // Check each face for culling
                    let mut faces_to_render = Vec::new();
                    
                    // Check each direction for adjacent blocks
                    let directions = [
                        (0, 0, 1),   // Front (+Z)
                        (0, 0, -1),  // Back (-Z)
                        (-1, 0, 0),  // Left (-X)
                        (1, 0, 0),   // Right (+X)
                        (0, 1, 0),   // Top (+Y)
                        (0, -1, 0),  // Bottom (-Y)
                    ];
                    
                    for (i, &(dx, dy, dz)) in directions.iter().enumerate() {
                        let adj_x = x as i32 + dx;
                        let adj_y = y as i32 + dy;
                        let adj_z = z as i32 + dz;
                        
                        let should_render_face = if adj_x < 0 || adj_x >= CHUNK_SIZE as i32 ||
                                                   adj_z < 0 || adj_z >= CHUNK_SIZE as i32 ||
                                                   adj_y < 0 || adj_y >= WORLD_HEIGHT as i32 {
                            // Face is at chunk boundary, check if there's a block in the neighboring position
                            if adj_y < 0 || adj_y >= WORLD_HEIGHT as i32 {
                                // Out of world bounds vertically, always render
                                true
                            } else {
                                // Check the actual world position for a block
                                let world_adj_x = world_x as i32 + dx;
                                let world_adj_z = world_z as i32 + dz;
                                let world_adj_y = y as i32 + dy;
                                !self.is_block_solid(world_adj_x, world_adj_y, world_adj_z)
                            }
                        } else {
                            // Check if adjacent block is air (render face) or solid (cull face)
                            let adj_block = chunk_blocks[adj_x as usize][adj_z as usize][adj_y as usize];
                            adj_block == BlockType::Air
                        };
                        
                        if should_render_face {
                            faces_to_render.push(i);
                        }
                    }
                    
                    // Only generate vertices for visible faces
                    if !faces_to_render.is_empty() {
                        let textures = registry.get_textures(block_type);

                        let vertex_offset = vertices.len() as u32;
                        let cube_vertices = create_cube_vertices_selective(
                            world_x, y as f32, world_z, 
                            &textures,
                            &faces_to_render
                        );
                        vertices.extend(cube_vertices);

                        let cube_indices = create_cube_indices_selective(&faces_to_render, vertex_offset);
                        indices.extend(cube_indices);
                    }
                }
            }
        }

        (ChunkData {
            vertices,
            indices,
        }, chunk_blocks)
    }

    fn create_chunk_from_data(&self, chunk_data: ChunkData, device: &wgpu::Device) -> Chunk {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk Vertex Buffer"),
            contents: bytemuck::cast_slice(&chunk_data.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk Index Buffer"),
            contents: bytemuck::cast_slice(&chunk_data.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Chunk {
            vertex_buffer,
            index_buffer,
            num_indices: chunk_data.indices.len() as u32,
        }
    }

    pub fn update(&mut self, camera_pos: Point3<f32>, device: &wgpu::Device) {
        let camera_chunk_x = (camera_pos.x / CHUNK_SIZE as f32).floor() as i32;
        let camera_chunk_z = (camera_pos.z / CHUNK_SIZE as f32).floor() as i32;

        // Collect all chunk positions that need generation
        let mut chunks_to_generate = Vec::new();
        for dx in -RENDER_DISTANCE..=RENDER_DISTANCE {
            for dz in -RENDER_DISTANCE..=RENDER_DISTANCE {
                let chunk_pos = ChunkPos {
                    x: camera_chunk_x + dx,
                    z: camera_chunk_z + dz,
                };

                if !self.chunks.contains_key(&chunk_pos) {
                    chunks_to_generate.push(chunk_pos);
                }
            }
        }

        // Generate chunk data in parallel
        if !chunks_to_generate.is_empty() {
            self.progress.is_generating = true;
            self.progress.total_chunks = chunks_to_generate.len();
            self.progress.completed_chunks = 0;
            
            // Generate chunks in parallel (structures will be generated inside)
            let structure_generator = &self.structure_generator;
            let height_noise = &self.height_noise;
            let biome_noise = &self.biome_noise;
            
            let chunk_data_results: Vec<(ChunkPos, ChunkData, [[[BlockType; WORLD_HEIGHT]; CHUNK_SIZE]; CHUNK_SIZE])> = chunks_to_generate
                .into_par_iter()
                .map(|chunk_pos| {
                    // Generate height and biome maps for structure generation
                    let mut height_values = [[0usize; CHUNK_SIZE]; CHUNK_SIZE];
                    let mut biome_values = [[0.0f64; CHUNK_SIZE]; CHUNK_SIZE];
                    
                    for x in 0..CHUNK_SIZE {
                        for z in 0..CHUNK_SIZE {
                            let world_x = (chunk_pos.x * CHUNK_SIZE as i32 + x as i32) as f32;
                            let world_z = (chunk_pos.z * CHUNK_SIZE as i32 + z as i32) as f32;
                            
                            let scale1 = 0.02;
                            let scale2 = 0.05;
                            let scale3 = 0.1;
                            
                            let height_noise1 = height_noise.get([world_x as f64 * scale1, world_z as f64 * scale1]);
                            let height_noise2 = height_noise.get([world_x as f64 * scale2, world_z as f64 * scale2]);
                            let height_noise3 = height_noise.get([world_x as f64 * scale3, world_z as f64 * scale3]);
                            
                            let combined_noise = height_noise1 * 0.6 + height_noise2 * 0.3 + height_noise3 * 0.1;
                            let height_variation = (WORLD_HEIGHT - BASE_HEIGHT) as f64;
                            let height = BASE_HEIGHT + ((combined_noise + 1.0) * 0.5 * height_variation) as usize;
                            
                            let biome_noise_val = biome_noise.get([world_x as f64 * 0.01, world_z as f64 * 0.01]);
                            
                            height_values[x][z] = height;
                            biome_values[x][z] = biome_noise_val;
                        }
                    }
                    
                    let structures = structure_generator.generate_structures_for_chunk(
                        chunk_pos.x,
                        chunk_pos.z,
                        &height_values,
                        &biome_values,
                    );
                    
                    let (chunk_data, block_array) = self.generate_chunk_data(chunk_pos, &structures);
                    (chunk_pos, chunk_data, block_array)
                })
                .collect();

            // Create GPU buffers on main thread and insert chunks
            for (chunk_pos, chunk_data, block_array) in chunk_data_results {
                let chunk = self.create_chunk_from_data(chunk_data, device);
                self.chunks.insert(chunk_pos, chunk);
                self.chunk_blocks.insert(chunk_pos, block_array);
                self.progress.completed_chunks += 1;
                
                // Remove delay for production
                // std::thread::sleep(std::time::Duration::from_millis(50));
            }
            
            self.progress.is_generating = false;
        }

        // Remove distant chunks
        let chunks_to_remove: Vec<ChunkPos> = self.chunks.keys()
            .filter(|&pos| {
                let dx = pos.x - camera_chunk_x;
                let dz = pos.z - camera_chunk_z;
                dx.abs() > RENDER_DISTANCE || dz.abs() > RENDER_DISTANCE
            })
            .copied()
            .collect();

        for chunk_pos in chunks_to_remove {
            self.chunks.remove(&chunk_pos);
            self.chunk_blocks.remove(&chunk_pos);
        }
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        for chunk in self.chunks.values() {
            render_pass.set_vertex_buffer(0, chunk.vertex_buffer.slice(..));
            render_pass.set_index_buffer(chunk.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..chunk.num_indices, 0, 0..1);
        }
    }
    
    /// Check if there's a solid block at the given world position
    pub fn is_block_solid(&self, world_x: i32, world_y: i32, world_z: i32) -> bool {
        // Check if Y is within valid range
        if world_y < 0 || world_y >= WORLD_HEIGHT as i32 {
            return false;
        }
        
        // Convert world coordinates to chunk coordinates
        let chunk_x = world_x.div_euclid(CHUNK_SIZE as i32);
        let chunk_z = world_z.div_euclid(CHUNK_SIZE as i32);
        let chunk_pos = ChunkPos { x: chunk_x, z: chunk_z };
        
        // Check if chunk blocks exist
        if let Some(chunk_blocks) = self.chunk_blocks.get(&chunk_pos) {
            // Convert world coordinates to block coordinates within chunk
            let block_x = world_x.rem_euclid(CHUNK_SIZE as i32) as usize;
            let block_z = world_z.rem_euclid(CHUNK_SIZE as i32) as usize;
            let block_y = world_y as usize;
            
            // Use cached block data - this is the single source of truth
            chunk_blocks[block_x][block_z][block_y] != BlockType::Air
        } else {
            false // Chunk not loaded
        }
    }
    
    /// Get the block type at the given world position
    pub fn get_block_type(&self, world_x: i32, world_y: i32, world_z: i32) -> Option<BlockType> {
        // Check if Y is within valid range
        if world_y < 0 || world_y >= WORLD_HEIGHT as i32 {
            return None;
        }
        
        // Convert world coordinates to chunk coordinates
        let chunk_x = world_x.div_euclid(CHUNK_SIZE as i32);
        let chunk_z = world_z.div_euclid(CHUNK_SIZE as i32);
        let chunk_pos = ChunkPos { x: chunk_x, z: chunk_z };
        
        // Check if chunk blocks exist
        if let Some(chunk_blocks) = self.chunk_blocks.get(&chunk_pos) {
            // Convert world coordinates to block coordinates within chunk
            let block_x = world_x.rem_euclid(CHUNK_SIZE as i32) as usize;
            let block_z = world_z.rem_euclid(CHUNK_SIZE as i32) as usize;
            let block_y = world_y as usize;
            
            // Use cached block data - this is the single source of truth
            Some(chunk_blocks[block_x][block_z][block_y])
        } else {
            None // Chunk not loaded
        }
    }
    
    /// Update chunk mesh from existing block data (no terrain regeneration)
    fn update_chunk_mesh(&mut self, chunk_pos: ChunkPos, device: &wgpu::Device) {
        // Get the existing chunk block data
        if let Some(chunk_blocks) = self.chunk_blocks.get(&chunk_pos) {
            // Generate mesh from current block data
            let mesh_data = self.generate_mesh_from_blocks(chunk_pos, chunk_blocks);
            let new_chunk = self.create_chunk_from_data(mesh_data, device);
            self.chunks.insert(chunk_pos, new_chunk);
        }
    }
    
    /// Generate mesh from existing block data (extracted from generate_chunk_data)
    fn generate_mesh_from_blocks(&self, chunk_pos: ChunkPos, chunk_blocks: &[[[BlockType; WORLD_HEIGHT]; CHUNK_SIZE]; CHUNK_SIZE]) -> ChunkData {
        let mut vertices = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        let registry = get_block_registry();
        
        // Generate vertices with face culling (same logic as before)
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for y in 0..WORLD_HEIGHT {
                    let block_type = chunk_blocks[x][z][y];
                    
                    // Skip air blocks
                    if block_type == BlockType::Air {
                        continue;
                    }
                    
                    let world_x = (chunk_pos.x * CHUNK_SIZE as i32 + x as i32) as f32;
                    let world_z = (chunk_pos.z * CHUNK_SIZE as i32 + z as i32) as f32;
                    
                    // Check each face for culling
                    let mut faces_to_render = Vec::new();
                    
                    // Check each direction for adjacent blocks
                    let directions = [
                        (0, 0, 1),   // Front (+Z)
                        (0, 0, -1),  // Back (-Z)
                        (-1, 0, 0),  // Left (-X)
                        (1, 0, 0),   // Right (+X)
                        (0, 1, 0),   // Top (+Y)
                        (0, -1, 0),  // Bottom (-Y)
                    ];
                    
                    for (i, &(dx, dy, dz)) in directions.iter().enumerate() {
                        let adj_x = x as i32 + dx;
                        let adj_y = y as i32 + dy;
                        let adj_z = z as i32 + dz;
                        
                        let should_render_face = if adj_x < 0 || adj_x >= CHUNK_SIZE as i32 ||
                                                   adj_z < 0 || adj_z >= CHUNK_SIZE as i32 ||
                                                   adj_y < 0 || adj_y >= WORLD_HEIGHT as i32 {
                            // Face is at chunk boundary, check if there's a block in the neighboring position
                            if adj_y < 0 || adj_y >= WORLD_HEIGHT as i32 {
                                // Out of world bounds vertically, always render
                                true
                            } else {
                                // Check the actual world position for a block
                                let world_adj_x = world_x as i32 + dx;
                                let world_adj_z = world_z as i32 + dz;
                                let world_adj_y = y as i32 + dy;
                                !self.is_block_solid(world_adj_x, world_adj_y, world_adj_z)
                            }
                        } else {
                            // Check if adjacent block is air (render face) or solid (cull face)
                            let adj_block = chunk_blocks[adj_x as usize][adj_z as usize][adj_y as usize];
                            adj_block == BlockType::Air
                        };
                        
                        if should_render_face {
                            faces_to_render.push(i);
                        }
                    }
                    
                    // Only generate vertices for visible faces
                    if !faces_to_render.is_empty() {
                        let textures = registry.get_textures(block_type);

                        let vertex_offset = vertices.len() as u32;
                        let cube_vertices = create_cube_vertices_selective(
                            world_x, y as f32, world_z, 
                            &textures,
                            &faces_to_render
                        );
                        vertices.extend(cube_vertices);

                        let cube_indices = create_cube_indices_selective(&faces_to_render, vertex_offset);
                        indices.extend(cube_indices);
                    }
                }
            }
        }
        
        ChunkData {
            vertices,
            indices,
        }
    }
    
    
    /// Remove a block at the given world position and update the mesh
    /// Returns the type of block that was removed, or None if no block was removed
    pub fn remove_block(&mut self, world_x: i32, world_y: i32, world_z: i32, device: &wgpu::Device) -> Option<BlockType> {
        // Check if block exists before trying to remove it
        if !self.is_block_solid(world_x, world_y, world_z) {
            return None;
        }
        
        // Get the block type before removing it
        let block_type = self.get_block_type(world_x, world_y, world_z);
        
        println!("Removing block at world position: ({}, {}, {})", world_x, world_y, world_z);
        
        // Convert world coordinates to chunk coordinates
        let chunk_x = world_x.div_euclid(CHUNK_SIZE as i32);
        let chunk_z = world_z.div_euclid(CHUNK_SIZE as i32);
        let chunk_pos = ChunkPos { x: chunk_x, z: chunk_z };
        
        // Get chunk-relative coordinates
        let block_x = world_x.rem_euclid(CHUNK_SIZE as i32) as usize;
        let block_z = world_z.rem_euclid(CHUNK_SIZE as i32) as usize;
        let block_y = world_y as usize;
        
        // Update the block directly in chunk_blocks
        if let Some(chunk_blocks) = self.chunk_blocks.get_mut(&chunk_pos) {
            chunk_blocks[block_x][block_z][block_y] = BlockType::Air;
            
            // Update mesh for this chunk (much faster than full regeneration)
            self.update_chunk_mesh(chunk_pos, device);
        }
        
        // Check if block is at chunk boundary and regenerate neighboring chunks if needed
        let local_x = world_x.rem_euclid(CHUNK_SIZE as i32);
        let local_z = world_z.rem_euclid(CHUNK_SIZE as i32);
        
        // Check each direction for chunk boundaries
        if local_x == 0 {
            let neighbor_pos = ChunkPos { x: chunk_x - 1, z: chunk_z };
            self.update_chunk_mesh(neighbor_pos, device);
        }
        if local_x == CHUNK_SIZE as i32 - 1 {
            let neighbor_pos = ChunkPos { x: chunk_x + 1, z: chunk_z };
            self.update_chunk_mesh(neighbor_pos, device);
        }
        if local_z == 0 {
            let neighbor_pos = ChunkPos { x: chunk_x, z: chunk_z - 1 };
            self.update_chunk_mesh(neighbor_pos, device);
        }
        if local_z == CHUNK_SIZE as i32 - 1 {
            let neighbor_pos = ChunkPos { x: chunk_x, z: chunk_z + 1 };
            self.update_chunk_mesh(neighbor_pos, device);
        }
        
        block_type
    }
    
    /// Add a block at the given world position and regenerate the affected chunk
    /// Returns true if the block was successfully added
    pub fn add_block(&mut self, world_x: i32, world_y: i32, world_z: i32, block_type: BlockType, device: &wgpu::Device) -> bool {
        // Check if Y is within valid range
        if world_y < 0 || world_y >= WORLD_HEIGHT as i32 {
            return false;
        }
        
        // Check if there's already a block at this position
        if self.is_block_solid(world_x, world_y, world_z) {
            return false;
        }
        
        println!("Adding {:?} block at world position: ({}, {}, {})", block_type, world_x, world_y, world_z);
        
        // Convert world coordinates to chunk coordinates
        let chunk_x = world_x.div_euclid(CHUNK_SIZE as i32);
        let chunk_z = world_z.div_euclid(CHUNK_SIZE as i32);
        let chunk_pos = ChunkPos { x: chunk_x, z: chunk_z };
        
        // Get chunk-relative coordinates
        let block_x = world_x.rem_euclid(CHUNK_SIZE as i32) as usize;
        let block_z = world_z.rem_euclid(CHUNK_SIZE as i32) as usize;
        let block_y = world_y as usize;
        
        // Update the block directly in chunk_blocks
        if let Some(chunk_blocks) = self.chunk_blocks.get_mut(&chunk_pos) {
            chunk_blocks[block_x][block_z][block_y] = block_type;
            
            // Update mesh for this chunk (much faster than full regeneration)
            self.update_chunk_mesh(chunk_pos, device);
        } else {
            return false; // Chunk not loaded
        }
        
        // Check if block is at chunk boundary and regenerate neighboring chunks if needed
        let local_x = world_x.rem_euclid(CHUNK_SIZE as i32);
        let local_z = world_z.rem_euclid(CHUNK_SIZE as i32);
        
        // Check each direction for chunk boundaries
        if local_x == 0 {
            // Block is at -X boundary, regenerate chunk to the left
            let neighbor_pos = ChunkPos { x: chunk_x - 1, z: chunk_z };
            self.update_chunk_mesh(neighbor_pos, device);
        }
        if local_x == CHUNK_SIZE as i32 - 1 {
            // Block is at +X boundary, regenerate chunk to the right
            let neighbor_pos = ChunkPos { x: chunk_x + 1, z: chunk_z };
            self.update_chunk_mesh(neighbor_pos, device);
        }
        if local_z == 0 {
            // Block is at -Z boundary, regenerate chunk behind
            let neighbor_pos = ChunkPos { x: chunk_x, z: chunk_z - 1 };
            self.update_chunk_mesh(neighbor_pos, device);
        }
        if local_z == CHUNK_SIZE as i32 - 1 {
            // Block is at +Z boundary, regenerate chunk in front
            let neighbor_pos = ChunkPos { x: chunk_x, z: chunk_z + 1 };
            self.update_chunk_mesh(neighbor_pos, device);
        }
        
        // Check corners (block at corner of chunk affects 3 neighboring chunks)
        if local_x == 0 && local_z == 0 {
            let neighbor_pos = ChunkPos { x: chunk_x - 1, z: chunk_z - 1 };
            self.update_chunk_mesh(neighbor_pos, device);
        }
        if local_x == 0 && local_z == CHUNK_SIZE as i32 - 1 {
            let neighbor_pos = ChunkPos { x: chunk_x - 1, z: chunk_z + 1 };
            self.update_chunk_mesh(neighbor_pos, device);
        }
        if local_x == CHUNK_SIZE as i32 - 1 && local_z == 0 {
            let neighbor_pos = ChunkPos { x: chunk_x + 1, z: chunk_z - 1 };
            self.update_chunk_mesh(neighbor_pos, device);
        }
        if local_x == CHUNK_SIZE as i32 - 1 && local_z == CHUNK_SIZE as i32 - 1 {
            let neighbor_pos = ChunkPos { x: chunk_x + 1, z: chunk_z + 1 };
            self.update_chunk_mesh(neighbor_pos, device);
        }
        
        true
    }
}