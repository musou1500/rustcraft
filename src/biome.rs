use crate::blocks::BlockType;
use noise::{NoiseFn, Perlin};
use std::collections::HashMap;

/// Explicit biome types with unique characteristics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Biome {
    Plains,
    Desert,
    Mountain,
    Tundra,
    Forest,
    Swamp,
}

/// Configuration for biome-specific terrain generation
#[derive(Debug, Clone)]
pub struct BiomeConfig {
    // Terrain shape parameters
    /// Base elevation level for terrain generation (in blocks above sea level)
    pub base_height: usize,
    /// Base frequency for noise generation - higher values create more detailed terrain
    pub frequency: f64,
    /// Base amplitude for terrain variation - higher values create more dramatic height changes
    pub amplitude: f64,

    // Block palette
    /// Primary block type for the topmost layer of terrain
    pub surface_block: BlockType,
    /// Secondary block type for layers beneath the surface (typically 1-3 blocks deep)
    pub subsurface_block: BlockType,
    /// Base block type used for deeper underground layers and mountain cores
    pub stone_block: BlockType,

    // Environmental factors
    /// Temperature value (-1.0 to 1.0) affecting block selection and biome transitions
    pub temperature: f64,
    /// Humidity value (-1.0 to 1.0) affecting vegetation and water-related features
    pub humidity: f64,

    // Structure spawn rates
    /// Probability per chunk position for tree generation (0.0 = never, higher = more frequent)
    pub tree_density: f64,
    /// Probability per chunk for house structure placement (0.0 = never, higher = more frequent)
    pub house_chance: f64,
}

/// Selects biomes based on environmental factors
pub struct BiomeSelector {
    temperature_noise: Perlin,
    humidity_noise: Perlin,
}

impl BiomeSelector {
    pub fn new(seed: u32) -> Self {
        Self {
            temperature_noise: Perlin::new(seed.wrapping_add(2000)),
            humidity_noise: Perlin::new(seed.wrapping_add(3000)),
        }
    }

    /// Select biome based on world position using temperature and humidity
    pub fn select_biome(&self, world_x: i32, world_z: i32) -> Biome {
        let temp = self
            .temperature_noise
            .get([world_x as f64 * 0.003, world_z as f64 * 0.003]);
        let humidity = self
            .humidity_noise
            .get([world_x as f64 * 0.004, world_z as f64 * 0.004]);

        // 2D biome grid based on temperature and humidity
        match (temp, humidity) {
            // Very cold regions
            (t, h) if t < -0.4 && h < 0.0 => Biome::Tundra,
            (t, _) if t < -0.2 => Biome::Mountain,

            // Hot and dry regions
            (t, h) if t > 0.3 && h < -0.2 => Biome::Desert,

            // Wet regions
            (_, h) if h > 0.4 => Biome::Swamp,

            // Temperate regions
            (t, _) if t > 0.1 => Biome::Forest,

            // Default temperate plains
            _ => Biome::Plains,
        }
    }

    /// Get blended biome configuration using distance-based blending
    pub fn get_blended_config(&self, world_x: i32, world_z: i32) -> BiomeConfig {
        // Sample biomes at nearby positions for distance-based blending
        let sample_radius = 8.0; // blocks
        let sample_points = [
            (0.0, 0.0), // center
            (sample_radius, 0.0),
            (-sample_radius, 0.0),
            (0.0, sample_radius),
            (0.0, -sample_radius),
            (sample_radius * 0.7, sample_radius * 0.7),
            (-sample_radius * 0.7, sample_radius * 0.7),
            (sample_radius * 0.7, -sample_radius * 0.7),
            (-sample_radius * 0.7, -sample_radius * 0.7),
        ];

        let mut biome_weights = HashMap::new();

        // Sample biomes and calculate distance-based weights
        for (dx, dz) in sample_points.iter() {
            let sample_x = world_x as f64 + dx;
            let sample_z = world_z as f64 + dz;
            let biome = self.select_biome(sample_x as i32, sample_z as i32);

            // Distance from center position
            let distance = (dx * dx + dz * dz).sqrt();
            // Use inverse distance weighting (closer = more influence)
            let weight = 1.0 / (1.0 + distance / sample_radius);

            *biome_weights.entry(biome).or_insert(0.0) += weight;
        }

        // Normalize weights
        let total_weight: f64 = biome_weights.values().sum();
        if total_weight == 0.0 {
            return Biome::Plains.get_config();
        }

        // Blend configurations based on weights
        let mut blended_config = BiomeConfig {
            base_height: 0,
            frequency: 0.0,
            amplitude: 0.0,
            surface_block: BlockType::Grass,
            subsurface_block: BlockType::Dirt,
            stone_block: BlockType::Stone,
            temperature: 0.0,
            humidity: 0.0,
            tree_density: 0.0,
            house_chance: 0.0,
        };

        let mut base_height_sum = 0.0;
        let mut dominant_biome = Biome::Plains;
        let mut max_weight = 0.0;

        for (biome, weight) in &biome_weights {
            let config = biome.get_config();
            let normalized_weight = weight / total_weight;

            base_height_sum += config.base_height as f64 * normalized_weight;
            blended_config.frequency += config.frequency * normalized_weight;
            blended_config.amplitude += config.amplitude * normalized_weight;
            blended_config.temperature += config.temperature * normalized_weight;
            blended_config.humidity += config.humidity * normalized_weight;
            blended_config.tree_density += config.tree_density * normalized_weight;
            blended_config.house_chance += config.house_chance * normalized_weight;

            // Track dominant biome for block types
            if *weight > max_weight {
                max_weight = *weight;
                dominant_biome = *biome;
            }
        }

        blended_config.base_height = base_height_sum.round() as usize;

        // Use dominant biome's block types
        let dominant_config = dominant_biome.get_config();
        blended_config.surface_block = dominant_config.surface_block;
        blended_config.subsurface_block = dominant_config.subsurface_block;

        blended_config
    }
}

impl Biome {
    /// Get configuration parameters for this biome
    pub fn get_config(&self) -> BiomeConfig {
        match self {
            Biome::Mountain => BiomeConfig {
                base_height: 18,
                frequency: 0.02, // Medium detail for mountain ridges
                amplitude: 6.0,  // Higher amplitude for dramatic mountain peaks
                surface_block: BlockType::Stone,
                subsurface_block: BlockType::Stone,
                stone_block: BlockType::Stone,
                temperature: -0.5,
                humidity: 0.0,
                tree_density: 0.005, // Sparse trees
                house_chance: 0.001, // Rare settlements
            },

            Biome::Desert => BiomeConfig {
                base_height: 5,
                frequency: 0.015, // Low detail for smooth terrain with subtle dunes
                amplitude: 1.5,   // Low amplitude for gentle dunes
                surface_block: BlockType::Sand,
                subsurface_block: BlockType::Sand,
                stone_block: BlockType::Stone,
                temperature: 0.8,
                humidity: -0.8,
                tree_density: 0.0001, // Almost no trees
                house_chance: 0.002,  // Occasional oasis settlements
            },

            Biome::Plains => BiomeConfig {
                base_height: 5,
                frequency: 0.018, // Standard detail level for rolling terrain
                amplitude: 2.5,   // Moderate amplitude for gentle rolling hills
                surface_block: BlockType::Grass,
                subsurface_block: BlockType::Dirt,
                stone_block: BlockType::Stone,
                temperature: 0.2,
                humidity: 0.0,
                tree_density: 0.015, // Moderate tree coverage
                house_chance: 0.008, // Common settlements
            },

            Biome::Forest => BiomeConfig {
                base_height: 5,
                frequency: 0.022, // Slightly higher detail for varied forest terrain
                amplitude: 3.0,   // Moderate amplitude for forest hills
                surface_block: BlockType::Grass,
                subsurface_block: BlockType::Dirt,
                stone_block: BlockType::Stone,
                temperature: 0.3,
                humidity: 0.2,
                tree_density: 0.08,  // Dense forest
                house_chance: 0.003, // Rare clearings
            },

            Biome::Tundra => BiomeConfig {
                base_height: 5,
                frequency: 0.012, // Low detail for flat tundra terrain
                amplitude: 1.0,   // Very low amplitude for flat tundra
                surface_block: BlockType::Snow,
                subsurface_block: BlockType::Dirt,
                stone_block: BlockType::Stone,
                temperature: -0.7,
                humidity: -0.2,
                tree_density: 0.002,  // Very sparse trees
                house_chance: 0.0005, // Extremely rare settlements
            },

            Biome::Swamp => BiomeConfig {
                base_height: 3,
                frequency: 0.01, // Very low detail for very flat swampland
                amplitude: 0.8,  // Minimal amplitude for swamp flatness
                surface_block: BlockType::Grass,
                subsurface_block: BlockType::Dirt,
                stone_block: BlockType::Stone,
                temperature: 0.1,
                humidity: 0.8,
                tree_density: 0.04,  // Moderate tree coverage
                house_chance: 0.001, // Rare stilted settlements
            },
        }
    }

    /// Get the name of this biome for debugging
    pub fn name(&self) -> &'static str {
        match self {
            Biome::Plains => "Plains",
            Biome::Desert => "Desert",
            Biome::Mountain => "Mountain",
            Biome::Tundra => "Tundra",
            Biome::Forest => "Forest",
            Biome::Swamp => "Swamp",
        }
    }
}
