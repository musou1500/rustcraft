use crate::blocks::BlockType;
use crate::chunk::{Chunk, ChunkData, ChunkGenerator, ChunkPos, ChunkBlocks, CHUNK_SIZE, WORLD_HEIGHT};
use crate::terrain::Terrain;
use crate::voxel::{create_cube_indices_selective, create_cube_vertices_selective};
use cgmath::Point3;
use std::collections::HashMap;

const RENDER_DISTANCE: i32 = 4;

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

pub struct World {
    chunks: HashMap<ChunkPos, Chunk>,
    terrain: Terrain,
    chunk_generator: ChunkGenerator,
    pub progress: TerrainProgress,
    // Cache the actual block data for each chunk - this is the single source of truth
    chunk_blocks: HashMap<ChunkPos, ChunkBlocks>,
}

impl World {
    pub fn new() -> Self {
        let terrain = Terrain::new(42);
        let chunk_generator = ChunkGenerator::new(7777);
        let chunks = HashMap::new();

        Self {
            chunks,
            terrain,
            chunk_generator,
            progress: TerrainProgress::new(),
            chunk_blocks: HashMap::new(),
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

            // Generate chunks in parallel
            use rayon::prelude::*;
            let chunk_data_results: Vec<(ChunkPos, ChunkData, ChunkBlocks)> = chunks_to_generate
                .into_par_iter()
                .map(|chunk_pos| {
                    let (chunk_data, block_array) =
                        self.chunk_generator.generate_chunk(chunk_pos, &self.terrain);
                    (chunk_pos, chunk_data, block_array)
                })
                .collect();

            // Create GPU buffers on main thread and insert chunks
            for (chunk_pos, chunk_data, block_array) in chunk_data_results {
                let chunk = Chunk::from_data(chunk_data, device);
                self.chunks.insert(chunk_pos, chunk);
                self.chunk_blocks.insert(chunk_pos, block_array);
                self.progress.completed_chunks += 1;
            }

            self.progress.is_generating = false;
        }

        // Remove distant chunks
        let chunks_to_remove: Vec<ChunkPos> = self
            .chunks
            .keys()
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
        let chunk_pos = ChunkPos {
            x: chunk_x,
            z: chunk_z,
        };

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
        let chunk_pos = ChunkPos {
            x: chunk_x,
            z: chunk_z,
        };

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

    /// Remove a block at the given world position and update the mesh
    /// Returns the type of block that was removed, or None if no block was removed
    pub fn remove_block(
        &mut self,
        world_x: i32,
        world_y: i32,
        world_z: i32,
        device: &wgpu::Device,
    ) -> Option<BlockType> {
        // Check if block exists before trying to remove it
        if !self.is_block_solid(world_x, world_y, world_z) {
            return None;
        }

        // Get the block type before removing it
        let block_type = self.get_block_type(world_x, world_y, world_z);

        println!(
            "Removing block at world position: ({}, {}, {})",
            world_x, world_y, world_z
        );

        // Convert world coordinates to chunk coordinates
        let chunk_x = world_x.div_euclid(CHUNK_SIZE as i32);
        let chunk_z = world_z.div_euclid(CHUNK_SIZE as i32);
        let chunk_pos = ChunkPos {
            x: chunk_x,
            z: chunk_z,
        };

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
            let neighbor_pos = ChunkPos {
                x: chunk_x - 1,
                z: chunk_z,
            };
            self.update_chunk_mesh(neighbor_pos, device);
        }
        if local_x == CHUNK_SIZE as i32 - 1 {
            let neighbor_pos = ChunkPos {
                x: chunk_x + 1,
                z: chunk_z,
            };
            self.update_chunk_mesh(neighbor_pos, device);
        }
        if local_z == 0 {
            let neighbor_pos = ChunkPos {
                x: chunk_x,
                z: chunk_z - 1,
            };
            self.update_chunk_mesh(neighbor_pos, device);
        }
        if local_z == CHUNK_SIZE as i32 - 1 {
            let neighbor_pos = ChunkPos {
                x: chunk_x,
                z: chunk_z + 1,
            };
            self.update_chunk_mesh(neighbor_pos, device);
        }

        block_type
    }

    /// Add a block at the given world position and regenerate the affected chunk
    /// Returns true if the block was successfully added
    pub fn add_block(
        &mut self,
        world_x: i32,
        world_y: i32,
        world_z: i32,
        block_type: BlockType,
        device: &wgpu::Device,
    ) -> bool {
        // Check if Y is within valid range
        if world_y < 0 || world_y >= WORLD_HEIGHT as i32 {
            return false;
        }

        // Check if there's already a block at this position
        if self.is_block_solid(world_x, world_y, world_z) {
            return false;
        }

        println!(
            "Adding {:?} block at world position: ({}, {}, {})",
            block_type, world_x, world_y, world_z
        );

        // Convert world coordinates to chunk coordinates
        let chunk_x = world_x.div_euclid(CHUNK_SIZE as i32);
        let chunk_z = world_z.div_euclid(CHUNK_SIZE as i32);
        let chunk_pos = ChunkPos {
            x: chunk_x,
            z: chunk_z,
        };

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

        // Update neighboring chunks at boundaries
        self.update_boundary_chunks(chunk_x, chunk_z, local_x, local_z, device);

        true
    }

    fn update_boundary_chunks(&mut self, chunk_x: i32, chunk_z: i32, local_x: i32, local_z: i32, device: &wgpu::Device) {
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
    }

    /// Update chunk mesh from existing block data (no terrain regeneration)
    fn update_chunk_mesh(&mut self, chunk_pos: ChunkPos, device: &wgpu::Device) {
        // Get the existing chunk block data
        if let Some(chunk_blocks) = self.chunk_blocks.get(&chunk_pos) {
            // Generate mesh from current block data
            let mesh_data = self.generate_mesh_from_blocks(chunk_pos, chunk_blocks);
            let new_chunk = Chunk::from_data(mesh_data, device);
            self.chunks.insert(chunk_pos, new_chunk);
        }
    }

    /// Generate mesh from existing block data
    fn generate_mesh_from_blocks(
        &self,
        chunk_pos: ChunkPos,
        chunk_blocks: &ChunkBlocks,
    ) -> ChunkData {
        let mut vertices = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        let registry = crate::blocks::get_block_registry();

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
                        (0, 0, 1),  // Front (+Z)
                        (0, 0, -1), // Back (-Z)
                        (-1, 0, 0), // Left (-X)
                        (1, 0, 0),  // Right (+X)
                        (0, 1, 0),  // Top (+Y)
                        (0, -1, 0), // Bottom (-Y)
                    ];

                    for (i, &(dx, dy, dz)) in directions.iter().enumerate() {
                        let adj_x = x as i32 + dx;
                        let adj_y = y as i32 + dy;
                        let adj_z = z as i32 + dz;

                        let should_render_face = if adj_x < 0
                            || adj_x >= CHUNK_SIZE as i32
                            || adj_z < 0
                            || adj_z >= CHUNK_SIZE as i32
                            || adj_y < 0
                            || adj_y >= WORLD_HEIGHT as i32
                        {
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
                            let adj_block =
                                chunk_blocks[adj_x as usize][adj_z as usize][adj_y as usize];
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
                            world_x,
                            y as f32,
                            world_z,
                            &textures,
                            &faces_to_render,
                        );
                        vertices.extend(cube_vertices);

                        let cube_indices =
                            create_cube_indices_selective(&faces_to_render, vertex_offset);
                        indices.extend(cube_indices);
                    }
                }
            }
        }

        ChunkData { vertices, indices }
    }

    /// Get all currently loaded chunk positions for debug rendering
    pub fn get_loaded_chunk_positions(&self) -> Vec<ChunkPos> {
        self.chunks.keys().copied().collect()
    }
}