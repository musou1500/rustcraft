use crate::biome::{Biome, BiomeManager, BiomeSelector};
use crate::blocks::BlockType;
use crate::chunk::{ChunkBlocks, ChunkPos, CHUNK_SIZE, TERRAIN_MAX_HEIGHT, WORLD_HEIGHT};
use noise::{NoiseFn, Perlin};

/// Terrain generation with biome-aware shaping and block selection
pub struct Terrain {
    height_noise: Perlin,
    biome_selector: BiomeSelector,
}

impl Terrain {
    pub fn new(seed: u32) -> Self {
        let height_noise = Perlin::new(seed);
        let biome_selector = BiomeSelector::new(seed);

        Self {
            height_noise,
            biome_selector,
        }
    }

    /// Generate terrain blocks for a chunk with biome-aware block selection
    pub fn generate_terrain_blocks(
        &self,
        chunk_pos: ChunkPos,
        height_values: &[Vec<usize>],
        biome_map: &[Vec<Biome>],
        biome_manager: &BiomeManager,
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
                        chunk_blocks[x][z][y] = self.get_block_for_position(
                            world_x,
                            y,
                            world_z,
                            height,
                            biome,
                            biome_manager,
                        );
                    }
                }
            }
        }

        chunk_blocks
    }

    /// Calculate terrain height at any world position using IWD-blended heights from nearby biomes
    pub fn height_at(&self, world_x: i32, world_z: i32, biome_manager: &BiomeManager) -> usize {
        let current_biome = self.biome_selector.select_biome(world_x, world_z);
        let current_height =
            self.calculate_height_for_biome(world_x, world_z, current_biome, biome_manager);

        // Find nearby biome boundaries
        let biome_boundaries = self.find_biome_boundaries(world_x, world_z);

        // If no boundaries found, return current biome height
        if biome_boundaries.is_empty() {
            return current_height;
        }

        // Calculate heights at boundaries and apply IWD blending
        let mut height_sum = 0.0; // Current position has distance ~0
        let mut weight_sum = 0.0;

        for (boundary_x, boundary_y, boundary_biome) in biome_boundaries {
            let boundary_height = self.calculate_height_for_biome(
                boundary_x,
                boundary_y,
                boundary_biome,
                biome_manager,
            );
            let distance =
                (((world_x - boundary_x).pow(2) + (world_z - boundary_y).pow(2)) as f64).sqrt();
            let weight = 1.0 / distance;
            height_sum += (boundary_height as f64 - current_height as f64) * weight;
            weight_sum += weight;
        }

        let blended_height = (height_sum / weight_sum).round() as usize;
        TERRAIN_MAX_HEIGHT
            .min(current_height + blended_height)
            .max(1)
    }

    /// Calculate height for a specific biome using octave-based noise
    fn calculate_height_for_biome(
        &self,
        world_x: i32,
        world_z: i32,
        biome: Biome,
        biome_manager: &BiomeManager,
    ) -> usize {
        let config = biome_manager.get_config(biome);
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

        noise_value.floor() as usize + config.base_height
    }

    /// Find nearby biome boundaries by searching in cardinal directions
    fn find_biome_boundaries(&self, world_x: i32, world_z: i32) -> Vec<(i32, i32, Biome)> {
        let max_distance = 32; // Maximum distance to search for boundaries
        let start_biome = self.biome_selector.select_biome(world_x, world_z);

        let search_directions = [
            (1, 0),  // +X axis
            (-1, 0), // -X axis
            (0, 1),  // +Z axis
            (0, -1), // -Z axis
        ];
        let mut boundaries = vec![];

        for (dx, dz) in &search_directions {
            for distance in 1..=max_distance {
                let neighbor_biome = self
                    .biome_selector
                    .select_biome(world_x + dx * distance, world_z + dz * distance);
                if neighbor_biome != start_biome {
                    boundaries.push((
                        world_x + dx * distance,
                        world_z + dz * distance,
                        neighbor_biome,
                    ));
                }
            }
        }
        boundaries
    }

    /// Select biome at any world position
    pub fn biome_at(&self, world_x: i32, world_z: i32) -> Biome {
        self.biome_selector.select_biome(world_x, world_z)
    }

    /// Get block type for a specific position using biome configuration
    pub fn get_block_for_position(
        &self,
        _world_x: i32,
        y: usize,
        _world_z: i32,
        height: usize,
        biome: Biome,
        biome_manager: &BiomeManager,
    ) -> BlockType {
        let config = biome_manager.get_config(biome);
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
            config.stone_block
        }
    }
}
