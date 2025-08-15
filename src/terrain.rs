use crate::biome::{Biome, BiomeSelector, BiomeWeights};
use crate::blocks::BlockType;
use crate::chunk::{ChunkBlocks, ChunkPos, CHUNK_SIZE, TERRAIN_MAX_HEIGHT, WORLD_HEIGHT};
use noise::{NoiseFn, Perlin};

const BASE_HEIGHT: usize = 8; // Minimum terrain height

/// Terrain generation with biome-aware shaping and block selection
pub struct Terrain {
    height_noise: Perlin,
    ore_noise: Perlin,
    texture_noise: Perlin,
    biome_selector: BiomeSelector,
}

impl Terrain {
    pub fn new(seed: u32) -> Self {
        let height_noise = Perlin::new(seed);
        let ore_noise = Perlin::new(seed.wrapping_add(9999));
        let texture_noise = Perlin::new(seed.wrapping_add(5555));
        let biome_selector = BiomeSelector::new(seed);

        Self {
            height_noise,
            ore_noise,
            texture_noise,
            biome_selector,
        }
    }

    /// Generate terrain blocks for a chunk with biome-aware block selection
    pub fn generate_terrain_blocks(
        &self,
        chunk_pos: ChunkPos,
        height_values: &[Vec<usize>],
        biome_map: &[Vec<Biome>],
    ) -> ChunkBlocks {
        let mut chunk_blocks = [[[BlockType::Air; WORLD_HEIGHT]; CHUNK_SIZE]; CHUNK_SIZE];

        // Generate block types using pre-computed biome data
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let height = height_values[x][z];
                let biome = biome_map[x][z];
                let world_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
                let world_z = chunk_pos.z * CHUNK_SIZE as i32 + z as i32;

                // Process all Y levels in the chunk, not just natural terrain height
                for y in 0..WORLD_HEIGHT {
                    // Generate terrain blocks (removed/placed blocks will be handled after generation)
                    if y < height.min(TERRAIN_MAX_HEIGHT) {
                        // Use new biome-aware block selection
                        chunk_blocks[x][z][y] =
                            self.get_block_for_position(world_x, y, world_z, height, biome);
                    } else {
                        // Above natural terrain height - default to air
                        chunk_blocks[x][z][y] = BlockType::Air;
                    }
                }
            }
        }

        chunk_blocks
    }

    /// Calculate terrain height at any world position using octave-based noise
    pub fn height_at(&self, world_x: i32, world_z: i32) -> usize {
        let biome_weights = self.biome_selector.get_biome_weights(world_x, world_z);
        let config = biome_weights.get_blended_config();

        let world_x = world_x as f64;
        let world_z = world_z as f64;

        // Octave-based noise generation parameters
        let octaves = 3;
        let persistence = 0.5; // Amplitude decay factor
        let lacunarity = 2.0; // Frequency multiplier

        // Generate octave-based noise
        let mut noise_value = 0.0;
        let mut amplitude = config.amplitude;
        let mut frequency = config.frequency;

        for _ in 0..octaves {
            noise_value += self
                .height_noise
                .get([world_x * frequency, world_z * frequency])
                * amplitude;
            amplitude *= persistence;
            frequency *= lacunarity;
        }

        TERRAIN_MAX_HEIGHT.min(noise_value.floor() as usize + config.base_height)
    }

    /// Select biome at any world position
    pub fn select_biome_at(&self, world_x: i32, world_z: i32) -> Biome {
        self.biome_selector.select_biome(world_x, world_z)
    }

    /// Get biome weights for smooth transitions at any world position
    pub fn get_biome_weights_at(&self, world_x: i32, world_z: i32) -> BiomeWeights {
        self.biome_selector.get_biome_weights(world_x, world_z)
    }

    /// Get block type for a specific position using biome configuration
    pub fn get_block_for_position(
        &self,
        world_x: i32,
        y: usize,
        world_z: i32,
        height: usize,
        biome: Biome,
    ) -> BlockType {
        let config = biome.get_config();
        let surface_level = height.saturating_sub(1);

        // Altitude-based overrides (snow on mountain peaks)
        if biome == Biome::Mountain && y > 30 && y >= surface_level {
            return BlockType::Snow;
        }

        // Biome-specific layering using config
        if y >= surface_level {
            config.surface_block
        } else if y >= height.saturating_sub(4) {
            config.subsurface_block
        } else {
            // Ore generation in stone layer
            let ore_noise =
                self.ore_noise
                    .get([world_x as f64 * 0.2, y as f64 * 0.3, world_z as f64 * 0.2]);

            if ore_noise > 0.95 {
                BlockType::Gold
            } else if ore_noise > 0.9 {
                BlockType::Iron
            } else if ore_noise > 0.8 {
                BlockType::Coal
            } else {
                config.stone_block
            }
        }
    }
}
