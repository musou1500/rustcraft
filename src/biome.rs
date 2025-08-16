use crate::blocks::BlockType;
use noise::{NoiseFn, Perlin};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Explicit biome types with unique characteristics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Biome {
    Plains,
    Desert,
    Mountain,
    Tundra,
    Forest,
    Swamp,
}

/// Configuration for biome-specific terrain generation
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

impl Biome {
    /// Get configuration parameters for this biome
    pub fn get_config(&self) -> BiomeConfig {
        match self {
            Biome::Mountain => BiomeConfig {
                base_height: 8,
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

/// Manages biome configurations with live reloading from file
pub struct BiomeManager {
    configs: HashMap<Biome, BiomeConfig>,
}

impl BiomeManager {
    /// Create a new BiomeManager with default configs
    pub fn new() -> Self {
        Self {
            configs: Self::load_default_configs(),
        }
    }

    /// Load biome configurations from biome.toml file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let configs: HashMap<Biome, BiomeConfig> = toml::from_str(&content)?;

        // Ensure all biomes are present
        for biome in [
            Biome::Plains,
            Biome::Desert,
            Biome::Mountain,
            Biome::Tundra,
            Biome::Forest,
            Biome::Swamp,
        ] {
            if !configs.contains_key(&biome) {
                return Err(format!("Missing configuration for biome: {:?}", biome).into());
            }
        }

        Ok(Self { configs })
    }

    /// Reload configurations from file
    pub fn reload_from_file<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let new_configs: HashMap<Biome, BiomeConfig> = toml::from_str(&content)?;

        // Ensure all biomes are present
        for biome in [
            Biome::Plains,
            Biome::Desert,
            Biome::Mountain,
            Biome::Tundra,
            Biome::Forest,
            Biome::Swamp,
        ] {
            if !new_configs.contains_key(&biome) {
                return Err(format!("Missing configuration for biome: {:?}", biome).into());
            }
        }

        self.configs = new_configs;
        println!("Biome configurations reloaded successfully!");
        Ok(())
    }

    /// Get configuration for a specific biome
    pub fn get_config(&self, biome: Biome) -> &BiomeConfig {
        self.configs.get(&biome).unwrap_or_else(|| {
            // Fallback to default if somehow missing
            &self.configs[&Biome::Plains]
        })
    }

    /// Create default configurations (fallback)
    fn load_default_configs() -> HashMap<Biome, BiomeConfig> {
        let mut configs = HashMap::new();

        for biome in [
            Biome::Plains,
            Biome::Desert,
            Biome::Mountain,
            Biome::Tundra,
            Biome::Forest,
            Biome::Swamp,
        ] {
            configs.insert(biome, biome.get_config());
        }

        configs
    }

    /// Save current configurations to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let toml_content = toml::to_string_pretty(&self.configs)?;
        fs::write(path, toml_content)?;
        Ok(())
    }
}
