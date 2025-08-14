use crate::blocks::{generation, BlockType};
use crate::chunk::{ChunkBlocks, ChunkPos, CHUNK_SIZE, TERRAIN_MAX_HEIGHT, WORLD_HEIGHT};
use noise::{NoiseFn, Perlin};

const BASE_HEIGHT: usize = 8; // Minimum terrain height

/// Pure terrain generation with noise functions
pub struct Terrain {
    height_noise: Perlin,
    biome_noise: Perlin,
    ore_noise: Perlin,
    texture_noise: Perlin,
}

impl Terrain {
    pub fn new(seed: u32) -> Self {
        let height_noise = Perlin::new(seed);
        let biome_noise = Perlin::new(seed.wrapping_add(1337));
        let ore_noise = Perlin::new(seed.wrapping_add(9999));
        let texture_noise = Perlin::new(seed.wrapping_add(5555));

        Self {
            height_noise,
            biome_noise,
            ore_noise,
            texture_noise,
        }
    }

    /// Generate terrain blocks for a chunk (no structures)
    pub fn generate_terrain_blocks(
        &self,
        chunk_pos: ChunkPos,
        height_values: &[Vec<usize>],
        biome_values: &[Vec<f64>],
    ) -> ChunkBlocks {
        let mut chunk_blocks = [[[BlockType::Air; WORLD_HEIGHT]; CHUNK_SIZE]; CHUNK_SIZE];

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
                    if y < height.min(TERRAIN_MAX_HEIGHT) {
                        // Only generate natural terrain within the natural height
                        let ore_noise_val = self.ore_noise.get([
                            world_x as f64 * 0.2,
                            y as f64 * 0.3,
                            world_z as f64 * 0.2,
                        ]);
                        let base_block = generation::get_block_for_height(
                            height,
                            TERRAIN_MAX_HEIGHT,
                            y,
                            ore_noise_val,
                        );
                        let block_type =
                            generation::get_biome_block(base_block, biome_noise_val, height, y);
                        chunk_blocks[x][z][y] = block_type;
                    } else {
                        // Above natural terrain height - default to air
                        chunk_blocks[x][z][y] = BlockType::Air;
                    }
                }
            }
        }

        chunk_blocks
    }

    /// Calculate terrain height at any world position
    pub fn calculate_height_at(&self, world_x: i32, world_z: i32) -> usize {
        let world_x = world_x as f32;
        let world_z = world_z as f32;

        // Height noise computation (centralized terrain calculation logic)
        let scale1 = 0.02;
        let scale2 = 0.05;
        let scale3 = 0.1;

        let height_noise1 = self
            .height_noise
            .get([world_x as f64 * scale1, world_z as f64 * scale1]);
        let height_noise2 = self
            .height_noise
            .get([world_x as f64 * scale2, world_z as f64 * scale2]);
        let height_noise3 = self
            .height_noise
            .get([world_x as f64 * scale3, world_z as f64 * scale3]);

        let combined_noise = height_noise1 * 0.6 + height_noise2 * 0.3 + height_noise3 * 0.1;
        let height_variation = (TERRAIN_MAX_HEIGHT - BASE_HEIGHT) as f64;
        BASE_HEIGHT + ((combined_noise + 1.0) * 0.5 * height_variation) as usize
    }

    /// Calculate biome value at any world position
    pub fn calculate_biome_at(&self, world_x: i32, world_z: i32) -> f64 {
        let world_x = world_x as f32;
        let world_z = world_z as f32;

        // Biome noise computation (centralized biome calculation logic)
        self.biome_noise
            .get([world_x as f64 * 0.01, world_z as f64 * 0.01])
    }
}
