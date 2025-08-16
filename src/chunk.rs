use crate::biome::Biome;
use crate::biome::BiomeManager;
use crate::blocks::{get_block_registry, BlockType};
use crate::structures::{PlacedStructure, StructureGenerator};
use crate::terrain::Terrain;
use crate::voxel::{create_cube_indices_selective, create_cube_vertices_selective, Vertex};

pub const CHUNK_SIZE: usize = 16;
pub const WORLD_HEIGHT: usize = 255; // Maximum world height for building
pub const TERRAIN_MAX_HEIGHT: usize = 64; // Maximum natural terrain height

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
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

pub type ChunkBlocks = [[[BlockType; WORLD_HEIGHT]; CHUNK_SIZE]; CHUNK_SIZE];

/// Orchestrates chunk generation by combining terrain and structures
pub struct ChunkGenerator {
    structure_generator: StructureGenerator,
}

impl ChunkGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            structure_generator: StructureGenerator::new(seed),
        }
    }

    /// Generate a complete chunk with terrain and structures
    pub fn generate_chunk(
        &self,
        chunk_pos: ChunkPos,
        terrain: &Terrain,
        biome_manager: &BiomeManager,
    ) -> (ChunkData, ChunkBlocks) {
        // Generate height and biome maps for structure generation
        let mut height_values = [[0usize; CHUNK_SIZE]; CHUNK_SIZE];
        let mut biome_map = [[Biome::Plains; CHUNK_SIZE]; CHUNK_SIZE];

        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let world_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
                let world_z = chunk_pos.z * CHUNK_SIZE as i32 + z as i32;

                let height = terrain.height_at(world_x, world_z, biome_manager);
                let biome = terrain.biome_at(world_x, world_z);

                height_values[x][z] = height;
                biome_map[x][z] = biome;
            }
        }

        // Generate structures for this chunk
        let structures = self.structure_generator.generate_structures_for_chunk(
            chunk_pos.x,
            chunk_pos.z,
            &height_values,
            &biome_map,
            terrain,
            biome_manager,
        );

        // Generate chunk data with terrain and structures combined
        self.generate_chunk_data(chunk_pos, &structures, terrain, biome_manager)
    }

    fn generate_chunk_data(
        &self,
        chunk_pos: ChunkPos,
        structures: &[PlacedStructure],
        terrain: &Terrain,
        biome_manager: &BiomeManager,
    ) -> (ChunkData, ChunkBlocks) {
        let mut vertices = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        let registry = get_block_registry();

        // Pre-generate block data for the entire chunk to enable face culling
        let mut chunk_blocks;

        // Pre-compute noise values for the entire chunk in batches
        let mut height_values = vec![vec![0usize; CHUNK_SIZE]; CHUNK_SIZE];
        let mut biome_map = vec![vec![Biome::Plains; CHUNK_SIZE]; CHUNK_SIZE];

        // Compute height and biome data sequentially
        let mut terrain_data = Vec::new();
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let world_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
                let world_z = chunk_pos.z * CHUNK_SIZE as i32 + z as i32;

                let height = terrain.height_at(world_x, world_z, biome_manager);
                let biome = terrain.biome_at(world_x, world_z);

                terrain_data.push((x, z, height, biome));
            }
        }

        // Store the computed values
        for (x, z, height, biome) in terrain_data {
            height_values[x][z] = height;
            biome_map[x][z] = biome;
        }

        // Generate terrain blocks using pre-computed biome data
        chunk_blocks =
            terrain.generate_terrain_blocks(chunk_pos, &height_values, &biome_map, biome_manager);

        // Place structure blocks into the chunk
        for structure in structures {
            for block in &structure.blocks {
                let block_x = structure.world_x + block.relative_pos.0;
                let block_y = structure.world_y + block.relative_pos.1;
                let block_z = structure.world_z + block.relative_pos.2;

                // Check if this block is within the current chunk
                let local_x = block_x - (chunk_pos.x * CHUNK_SIZE as i32);
                let local_z = block_z - (chunk_pos.z * CHUNK_SIZE as i32);

                if local_x >= 0
                    && local_x < CHUNK_SIZE as i32
                    && local_z >= 0
                    && local_z < CHUNK_SIZE as i32
                    && block_y >= 0
                    && block_y < WORLD_HEIGHT as i32
                {
                    // Place structure blocks
                    chunk_blocks[local_x as usize][local_z as usize][block_y as usize] =
                        block.block_type;
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
                                // For chunk boundaries, we'll assume render face (can be optimized later)
                                true
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

        (ChunkData { vertices, indices }, chunk_blocks)
    }
}

impl Chunk {
    pub fn from_data(chunk_data: ChunkData, device: &wgpu::Device) -> Self {
        use wgpu::util::DeviceExt;

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

        Self {
            vertex_buffer,
            index_buffer,
            num_indices: chunk_data.indices.len() as u32,
        }
    }
}
