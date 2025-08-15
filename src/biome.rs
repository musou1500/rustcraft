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

/// Represents blended biome weights for smooth transitions
#[derive(Debug, Clone)]
pub struct BiomeWeights {
    pub weights: HashMap<Biome, f64>,
}

impl BiomeWeights {
    pub fn new() -> Self {
        Self {
            weights: HashMap::new(),
        }
    }

    pub fn add_weight(&mut self, biome: Biome, weight: f64) {
        self.weights.insert(biome, weight);
    }

    /// Get the dominant biome (highest weight)
    pub fn get_dominant_biome(&self) -> Biome {
        self.weights
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(biome, _)| *biome)
            .unwrap_or(Biome::Plains)
    }

    /// Get blended biome configuration by averaging configs weighted by their influence
    pub fn get_blended_config(&self) -> BiomeConfig {
        if self.weights.is_empty() {
            return Biome::Plains.get_config();
        }

        let total_weight: f64 = self.weights.values().sum();
        if total_weight == 0.0 {
            return Biome::Plains.get_config();
        }

        let mut blended_config = BiomeConfig {
            base_height: 0,
            frequency: 0.0,
            amplitude: 0.0,
            surface_block: BlockType::Grass, // Will be determined by dominant biome
            subsurface_block: BlockType::Dirt, // Will be determined by dominant biome
            stone_block: BlockType::Stone,
            temperature: 0.0,
            humidity: 0.0,
            tree_density: 0.0,
            house_chance: 0.0,
        };

        // Calculate weighted averages for numerical properties
        let mut base_height_sum = 0.0;
        for (biome, weight) in &self.weights {
            let config = biome.get_config();
            let normalized_weight = weight / total_weight;

            base_height_sum += config.base_height as f64 * normalized_weight;
            blended_config.frequency += config.frequency * normalized_weight;
            blended_config.amplitude += config.amplitude * normalized_weight;
            blended_config.temperature += config.temperature * normalized_weight;
            blended_config.humidity += config.humidity * normalized_weight;
            blended_config.tree_density += config.tree_density * normalized_weight;
            blended_config.house_chance += config.house_chance * normalized_weight;
        }

        blended_config.base_height = base_height_sum.round() as usize;

        // Use dominant biome's block types (for now)
        let dominant_config = self.get_dominant_biome().get_config();
        blended_config.surface_block = dominant_config.surface_block;
        blended_config.subsurface_block = dominant_config.subsurface_block;

        blended_config
    }
}

/// Helper function for smooth interpolation
fn smooth_step(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
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

    /// Get biome weights for smooth transitions around a position
    pub fn get_biome_weights(&self, world_x: i32, world_z: i32) -> BiomeWeights {
        let temp = self
            .temperature_noise
            .get([world_x as f64 * 0.003, world_z as f64 * 0.003]);
        let humidity = self
            .humidity_noise
            .get([world_x as f64 * 0.004, world_z as f64 * 0.004]);

        let mut weights = BiomeWeights::new();

        // Define transition width for smooth boundaries
        let transition_width = 0.1;

        // Calculate weights for each biome based on distance from thresholds

        // Tundra: Very cold and not humid
        let tundra_temp_weight = if temp < -0.4 + transition_width {
            1.0 - smooth_step(-0.4 - transition_width, -0.4 + transition_width, temp)
        } else {
            0.0
        };
        let tundra_humidity_weight = if humidity < 0.0 + transition_width {
            1.0 - smooth_step(0.0 - transition_width, 0.0 + transition_width, humidity)
        } else {
            0.0
        };
        let tundra_weight = tundra_temp_weight * tundra_humidity_weight;
        if tundra_weight > 0.01 {
            weights.add_weight(Biome::Tundra, tundra_weight);
        }

        // Mountain: Cold
        let mountain_weight = if temp < -0.2 + transition_width {
            1.0 - smooth_step(-0.2 - transition_width, -0.2 + transition_width, temp)
        } else {
            0.0
        };
        if mountain_weight > 0.01 {
            weights.add_weight(Biome::Mountain, mountain_weight);
        }

        // Desert: Hot and dry
        let desert_temp_weight = if temp > 0.3 - transition_width {
            smooth_step(0.3 - transition_width, 0.3 + transition_width, temp)
        } else {
            0.0
        };
        let desert_humidity_weight = if humidity < -0.2 + transition_width {
            1.0 - smooth_step(-0.2 - transition_width, -0.2 + transition_width, humidity)
        } else {
            0.0
        };
        let desert_weight = desert_temp_weight * desert_humidity_weight;
        if desert_weight > 0.01 {
            weights.add_weight(Biome::Desert, desert_weight);
        }

        // Swamp: Very humid
        let swamp_weight = if humidity > 0.4 - transition_width {
            smooth_step(0.4 - transition_width, 0.4 + transition_width, humidity)
        } else {
            0.0
        };
        if swamp_weight > 0.01 {
            weights.add_weight(Biome::Swamp, swamp_weight);
        }

        // Forest: Temperate warm
        let forest_weight = if temp > 0.1 - transition_width && temp < 0.3 + transition_width {
            smooth_step(0.1 - transition_width, 0.1 + transition_width, temp)
                * (1.0 - smooth_step(0.3 - transition_width, 0.3 + transition_width, temp))
        } else {
            0.0
        };
        if forest_weight > 0.01 {
            weights.add_weight(Biome::Forest, forest_weight);
        }

        // Plains: Default for areas not covered by other biomes
        let other_weights_sum: f64 = weights.weights.values().sum();
        let plains_weight = (1.0 - other_weights_sum).max(0.0);
        if plains_weight > 0.01 {
            weights.add_weight(Biome::Plains, plains_weight);
        }

        // Ensure we have at least one biome
        if weights.weights.is_empty() {
            weights.add_weight(Biome::Plains, 1.0);
        }

        weights
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
