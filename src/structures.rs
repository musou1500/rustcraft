use crate::biome::Biome;
use crate::blocks::BlockType;
use crate::chunk::CHUNK_SIZE;
use noise::{NoiseFn, Perlin};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Represents a block placement in a structure
#[derive(Debug, Clone)]
pub struct BlockPlacement {
    pub relative_pos: (i32, i32, i32), // Position relative to structure origin
    pub block_type: BlockType,
}

/// Trait for all structures that can be generated in the world
pub trait Structure {
    /// Generate the blocks that make up this structure
    fn generate(&self, rng: &mut StdRng) -> Vec<BlockPlacement>;

    /// Get the bounding box size of this structure
    fn get_bounds(&self) -> (i32, i32, i32);

    /// Check if this structure can be placed at the given height
    fn can_place_at_height(&self, height: i32) -> bool;
}

/// Tree structure with varied shapes
pub struct TreeStructure {
    pub tree_type: TreeType,
}

#[derive(Debug, Clone, Copy)]
pub enum TreeType {
    Oak,
    Birch,
    Pine,
}

impl TreeStructure {
    pub fn new(tree_type: TreeType) -> Self {
        Self { tree_type }
    }

    /// Choose a random tree type based on biome
    pub fn random_for_biome(biome: Biome, rng: &mut StdRng) -> Self {
        let tree_type = match biome {
            Biome::Tundra | Biome::Mountain => {
                // Cold biomes - mostly pine trees
                if rng.gen::<f32>() < 0.8 {
                    TreeType::Pine
                } else {
                    TreeType::Oak
                }
            }
            Biome::Desert => {
                // Desert biome - very sparse, mostly oak (representing hardy trees)
                TreeType::Oak
            }
            Biome::Forest => {
                // Forest biome - dense mixed forest
                match rng.gen_range(0..10) {
                    0..=2 => TreeType::Pine,
                    3..=6 => TreeType::Oak,
                    _ => TreeType::Birch,
                }
            }
            Biome::Plains => {
                // Plains biome - sparse mixed trees
                match rng.gen_range(0..10) {
                    0..=1 => TreeType::Pine,
                    2..=5 => TreeType::Oak,
                    _ => TreeType::Birch,
                }
            }
            Biome::Swamp => {
                // Swamp biome - mostly oak and birch
                if rng.gen::<f32>() < 0.6 {
                    TreeType::Oak
                } else {
                    TreeType::Birch
                }
            }
        };

        Self::new(tree_type)
    }
}

impl Structure for TreeStructure {
    fn generate(&self, rng: &mut StdRng) -> Vec<BlockPlacement> {
        let mut blocks = Vec::new();

        match self.tree_type {
            TreeType::Oak => {
                // Oak tree: 4-6 height trunk with spherical leaves
                let height = rng.gen_range(4..=6);

                // Trunk
                for y in 0..height {
                    blocks.push(BlockPlacement {
                        relative_pos: (0, y, 0),
                        block_type: BlockType::Wood,
                    });
                }

                // Leaves - spherical shape
                let leaf_start = height - 2;
                let leaf_radius = 2;

                for dy in -leaf_radius..=leaf_radius + 1 {
                    for dx in -leaf_radius..=leaf_radius {
                        for dz in -leaf_radius..=leaf_radius {
                            let y = leaf_start + dy;

                            // Skip if below trunk
                            if y < height - 1 {
                                continue;
                            }

                            // Calculate distance from center
                            let dist_sq = dx * dx + dy * dy + dz * dz;
                            let radius_sq = leaf_radius * leaf_radius;

                            // Add some randomness to make it less perfect
                            let threshold = radius_sq as f32 + rng.gen::<f32>() * 2.0;

                            if dist_sq as f32 <= threshold {
                                // Don't place leaves where trunk is
                                if !(dx == 0 && dz == 0 && y < height) {
                                    blocks.push(BlockPlacement {
                                        relative_pos: (dx, y, dz),
                                        block_type: BlockType::Leaves,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            TreeType::Birch => {
                // Birch tree: taller (5-7) with cylindrical leaves
                let height = rng.gen_range(5..=7);

                // Trunk
                for y in 0..height {
                    blocks.push(BlockPlacement {
                        relative_pos: (0, y, 0),
                        block_type: BlockType::Wood,
                    });
                }

                // Leaves - more cylindrical/oval shape
                let leaf_start = height - 3;
                let leaf_height = 4;

                for y in leaf_start..leaf_start + leaf_height {
                    let radius = if y == leaf_start || y == leaf_start + leaf_height - 1 {
                        1 // Smaller at top and bottom
                    } else {
                        2 // Wider in middle
                    };

                    for dx in -radius..=radius {
                        for dz in -radius..=radius {
                            let dx: i32 = dx;
                            let dz: i32 = dz;
                            // Make it more square than circular
                            if dx.abs() <= radius && dz.abs() <= radius {
                                // Add some randomness to edges
                                if dx.abs() == radius || dz.abs() == radius {
                                    if rng.gen::<f32>() < 0.7 {
                                        blocks.push(BlockPlacement {
                                            relative_pos: (dx, y, dz),
                                            block_type: BlockType::Leaves,
                                        });
                                    }
                                } else if !(dx == 0 && dz == 0 && y < height) {
                                    blocks.push(BlockPlacement {
                                        relative_pos: (dx, y, dz),
                                        block_type: BlockType::Leaves,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            TreeType::Pine => {
                // Pine tree: 6-8 height with conical leaves
                let height = rng.gen_range(6..=8);

                // Trunk
                for y in 0..height {
                    blocks.push(BlockPlacement {
                        relative_pos: (0, y, 0),
                        block_type: BlockType::Wood,
                    });
                }

                // Leaves - conical shape
                let leaf_start = 2; // Pine trees have leaves lower
                let leaf_layers = height - 1;

                for y in leaf_start..leaf_layers {
                    // Radius decreases as we go up
                    let max_radius = 3;
                    let progress = (y - leaf_start) as f32 / (leaf_layers - leaf_start) as f32;
                    let radius = ((1.0 - progress) * max_radius as f32).ceil() as i32;

                    for dx in -radius..=radius {
                        for dz in -radius..=radius {
                            // Make it diamond-shaped rather than square
                            if dx.abs() + dz.abs() <= radius {
                                // Don't place leaves where trunk is
                                if !(dx == 0 && dz == 0) {
                                    // Add some randomness to edges
                                    if dx.abs() + dz.abs() == radius {
                                        if rng.gen::<f32>() < 0.8 {
                                            blocks.push(BlockPlacement {
                                                relative_pos: (dx, y, dz),
                                                block_type: BlockType::Leaves,
                                            });
                                        }
                                    } else {
                                        blocks.push(BlockPlacement {
                                            relative_pos: (dx, y, dz),
                                            block_type: BlockType::Leaves,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }

                // Add a small tuft at the top
                blocks.push(BlockPlacement {
                    relative_pos: (0, height, 0),
                    block_type: BlockType::Leaves,
                });
            }
        }

        blocks
    }

    fn get_bounds(&self) -> (i32, i32, i32) {
        match self.tree_type {
            TreeType::Oak => (5, 8, 5),
            TreeType::Birch => (5, 10, 5),
            TreeType::Pine => (7, 9, 7),
        }
    }

    fn can_place_at_height(&self, height: i32) -> bool {
        (5..20).contains(&height) // Trees need some ground and shouldn't be too high
    }
}

/// House structure with walls, roof, windows, and doors
pub struct HouseStructure {
    pub house_type: HouseType,
}

#[derive(Debug, Clone, Copy)]
pub enum HouseType {
    Small,
    Medium,
}

impl HouseStructure {
    pub fn new(house_type: HouseType) -> Self {
        Self { house_type }
    }

    pub fn random(rng: &mut StdRng) -> Self {
        let house_type = if rng.gen::<f32>() < 0.7 {
            HouseType::Small
        } else {
            HouseType::Medium
        };

        Self::new(house_type)
    }
}

impl Structure for HouseStructure {
    fn generate(&self, _rng: &mut StdRng) -> Vec<BlockPlacement> {
        let mut blocks = Vec::new();

        match self.house_type {
            HouseType::Small => {
                // Small house: 5x5 footprint, 4 blocks tall + roof
                let width = 5;
                let depth = 5;
                let wall_height = 4;

                // Floor (optional - using cobblestone)
                for x in 0..width {
                    for z in 0..depth {
                        blocks.push(BlockPlacement {
                            relative_pos: (x, 0, z),
                            block_type: BlockType::Cobblestone,
                        });
                    }
                }

                // Walls
                for y in 1..=wall_height {
                    for x in 0..width {
                        for z in 0..depth {
                            // Only place blocks on edges for walls
                            if x == 0 || x == width - 1 || z == 0 || z == depth - 1 {
                                // Door at front center
                                if z == 0 && x == width / 2 && (y == 1 || y == 2) {
                                    continue; // Door opening
                                }

                                // Windows on sides
                                let is_window = y == 2
                                    && (
                                        (x == width - 1 || x == 0) && z == depth / 2 ||  // Left/Right windows
                                    (z == depth - 1 && x == width / 2)
                                        // Back window
                                    );

                                if is_window {
                                    blocks.push(BlockPlacement {
                                        relative_pos: (x, y, z),
                                        block_type: BlockType::Glass,
                                    });
                                } else {
                                    blocks.push(BlockPlacement {
                                        relative_pos: (x, y, z),
                                        block_type: BlockType::Planks,
                                    });
                                }
                            }
                        }
                    }
                }

                // Peaked roof
                let roof_height = 2;
                for roof_y in 0..roof_height {
                    let inset = roof_y;
                    for x in inset..width - inset {
                        for z in inset..depth - inset {
                            // Only place roof blocks at edges of this level
                            if x == inset
                                || x == width - inset - 1
                                || z == inset
                                || z == depth - inset - 1
                            {
                                blocks.push(BlockPlacement {
                                    relative_pos: (x, wall_height + 1 + roof_y, z),
                                    block_type: BlockType::Cobblestone,
                                });
                            }
                        }
                    }
                }

                // Fill in the roof top
                let top_y = wall_height + 1 + roof_height;
                for x in roof_height..width - roof_height {
                    for z in roof_height..depth - roof_height {
                        blocks.push(BlockPlacement {
                            relative_pos: (x, top_y, z),
                            block_type: BlockType::Cobblestone,
                        });
                    }
                }
            }
            HouseType::Medium => {
                // Medium house: 7x7 footprint, 5 blocks tall + roof
                let width = 7;
                let depth = 7;
                let wall_height = 5;

                // Floor
                for x in 0..width {
                    for z in 0..depth {
                        blocks.push(BlockPlacement {
                            relative_pos: (x, 0, z),
                            block_type: BlockType::Cobblestone,
                        });
                    }
                }

                // Walls
                for y in 1..=wall_height {
                    for x in 0..width {
                        for z in 0..depth {
                            // Only place blocks on edges for walls
                            if x == 0 || x == width - 1 || z == 0 || z == depth - 1 {
                                // Door at front center (2 blocks wide for medium house)
                                if z == 0
                                    && (x == width / 2 || x == width / 2 - 1)
                                    && (y == 1 || y == 2)
                                {
                                    continue; // Door opening
                                }

                                // More windows for medium house
                                let is_window = y == 2
                                    && (
                                        (x == width - 1 || x == 0) && (z == depth - 3 || z == 2) ||  // Left/Right windows
                                    (z == depth - 1 && (x == 2 || x == width - 3))
                                        // Back windows
                                    )
                                    || (y == 3 && z == 0 && (x == 1 || x == width - 2)); // Front upper windows

                                if is_window {
                                    blocks.push(BlockPlacement {
                                        relative_pos: (x, y, z),
                                        block_type: BlockType::Glass,
                                    });
                                } else {
                                    // Mix materials for variety
                                    let material = if y == 1 || (x + z) % 3 == 0 {
                                        BlockType::Cobblestone
                                    } else {
                                        BlockType::Planks
                                    };

                                    blocks.push(BlockPlacement {
                                        relative_pos: (x, y, z),
                                        block_type: material,
                                    });
                                }
                            }
                        }
                    }
                }

                // Peaked roof (taller for medium house)
                let roof_height = 3;
                for roof_y in 0..roof_height {
                    let inset = roof_y;
                    for x in inset..width - inset {
                        for z in inset..depth - inset {
                            // Only place roof blocks at edges of this level
                            if x == inset
                                || x == width - inset - 1
                                || z == inset
                                || z == depth - inset - 1
                            {
                                blocks.push(BlockPlacement {
                                    relative_pos: (x, wall_height + 1 + roof_y, z),
                                    block_type: BlockType::Cobblestone,
                                });
                            }
                        }
                    }
                }

                // Fill in the roof top
                let top_y = wall_height + 1 + roof_height;
                for x in roof_height..width - roof_height {
                    for z in roof_height..depth - roof_height {
                        blocks.push(BlockPlacement {
                            relative_pos: (x, top_y, z),
                            block_type: BlockType::Cobblestone,
                        });
                    }
                }
            }
        }

        blocks
    }

    fn get_bounds(&self) -> (i32, i32, i32) {
        match self.house_type {
            HouseType::Small => (5, 7, 5),
            HouseType::Medium => (7, 9, 7),
        }
    }

    fn can_place_at_height(&self, height: i32) -> bool {
        (8..18).contains(&height) // Houses need flat ground, not too high
    }
}

/// Manages structure generation and placement
pub struct StructureGenerator {
    structure_noise: Perlin,
    seed: u32,
}

impl StructureGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            structure_noise: Perlin::new(seed),
            seed,
        }
    }

    /// Determine if a structure should be placed at this position
    pub fn should_place_structure(&self, world_x: i32, world_z: i32) -> bool {
        // Use noise to determine structure placement
        let scale = 0.05; // Structures are relatively rare
        let noise_value = self
            .structure_noise
            .get([world_x as f64 * scale, world_z as f64 * scale]);

        // Only place structures at peaks in the noise
        noise_value > 0.4 // Reduced threshold to make structures more common
    }

    /// Get the type of structure to place based on biome and randomness
    pub fn get_structure_type(&self, world_x: i32, world_z: i32, biome: Biome) -> StructureType {
        // Create a deterministic RNG based on position
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        world_x.hash(&mut hasher);
        world_z.hash(&mut hasher);
        self.seed.hash(&mut hasher);
        let hash = hasher.finish();

        let mut rng = StdRng::seed_from_u64(hash);
        let structure_roll = rng.gen::<f32>();

        let config = biome.get_config();

        // Use biome-specific structure spawn rates
        if structure_roll < (config.tree_density * 100.0) as f32 {
            StructureType::Tree
        } else if structure_roll < ((config.tree_density + config.house_chance) * 100.0) as f32 {
            StructureType::House
        } else {
            // No structure for this position
            StructureType::Tree // Default fallback (should rarely happen with proper tuning)
        }
    }

    /// Generate structures for a chunk, including structures from neighboring chunks that extend into this chunk
    pub fn generate_structures_for_chunk(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        terrain_height_map: &[[usize; CHUNK_SIZE]; CHUNK_SIZE],
        biome_map: &[[Biome; CHUNK_SIZE]; CHUNK_SIZE],
        terrain: &crate::terrain::Terrain,
    ) -> Vec<PlacedStructure> {
        let mut structures = Vec::new();

        // Maximum structure bounds analysis shows largest structures are 7x7
        // So we need to check positions up to 4 blocks outside chunk boundaries
        let search_radius = 4;
        let spacing = 8;

        // Calculate the range of world coordinates we need to check
        let chunk_start_x = chunk_x * CHUNK_SIZE as i32;
        let chunk_start_z = chunk_z * CHUNK_SIZE as i32;
        let search_start_x = chunk_start_x - search_radius;
        let search_end_x = chunk_start_x + CHUNK_SIZE as i32 + search_radius;
        let search_start_z = chunk_start_z - search_radius;
        let search_end_z = chunk_start_z + CHUNK_SIZE as i32 + search_radius;

        // Check positions in expanded search area
        for world_x in (search_start_x..search_end_x).step_by(spacing) {
            for world_z in (search_start_z..search_end_z).step_by(spacing) {
                if !self.should_place_structure(world_x, world_z) {
                    continue;
                }

                // Calculate local coordinates relative to the current chunk
                let local_x = world_x - chunk_start_x;
                let local_z = world_z - chunk_start_z;

                // For positions outside the chunk, we need to calculate height and biome using terrain
                let (terrain_height, biome) = if local_x >= 0
                    && local_x < CHUNK_SIZE as i32
                    && local_z >= 0
                    && local_z < CHUNK_SIZE as i32
                {
                    // Position is within current chunk - use provided height map
                    (
                        terrain_height_map[local_x as usize][local_z as usize],
                        biome_map[local_x as usize][local_z as usize],
                    )
                } else {
                    // Position is outside current chunk - query terrain for values
                    let height = terrain.height_at(world_x, world_z);
                    let biome = terrain.biome_at(world_x, world_z);
                    (height, biome)
                };

                // Create deterministic RNG for this position
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                use std::hash::{Hash, Hasher};
                world_x.hash(&mut hasher);
                world_z.hash(&mut hasher);
                self.seed.hash(&mut hasher);
                let hash = hasher.finish();
                let mut rng = StdRng::seed_from_u64(hash);

                let structure_type = self.get_structure_type(world_x, world_z, biome);

                let structure: Box<dyn Structure> = match structure_type {
                    StructureType::Tree => {
                        Box::new(TreeStructure::random_for_biome(biome, &mut rng))
                    }
                    StructureType::House => Box::new(HouseStructure::random(&mut rng)),
                };

                // Check if structure can be placed at this height
                if !structure.can_place_at_height(terrain_height as i32) {
                    continue;
                }

                // Check if there's enough flat area for houses
                if matches!(structure_type, StructureType::House) {
                    let (width, _, depth) = structure.get_bounds();
                    let mut height_variance = 0i32;

                    for dx in 0..width {
                        for dz in 0..depth {
                            let check_world_x = world_x + dx;
                            let check_world_z = world_z + dz;
                            let check_local_x = check_world_x - chunk_start_x;
                            let check_local_z = check_world_z - chunk_start_z;

                            let check_height = if check_local_x >= 0
                                && check_local_x < CHUNK_SIZE as i32
                                && check_local_z >= 0
                                && check_local_z < CHUNK_SIZE as i32
                            {
                                terrain_height_map[check_local_x as usize][check_local_z as usize]
                                    as i32
                            } else {
                                terrain.height_at(check_world_x, check_world_z) as i32
                            };

                            height_variance =
                                height_variance.max((check_height - terrain_height as i32).abs());
                        }
                    }

                    // Skip if terrain is too uneven for a house
                    if height_variance > 1 {
                        continue;
                    }
                }

                structures.push(PlacedStructure {
                    world_x,
                    world_y: terrain_height as i32,
                    world_z,
                    structure_type,
                    blocks: structure.generate(&mut rng),
                });
            }
        }

        structures
    }
}

#[derive(Debug, Clone)]
pub enum StructureType {
    Tree,
    House,
}

/// A structure that has been placed in the world
#[derive(Debug, Clone)]
pub struct PlacedStructure {
    pub world_x: i32,
    pub world_y: i32,
    pub world_z: i32,
    pub structure_type: StructureType,
    pub blocks: Vec<BlockPlacement>,
}

impl PlacedStructure {
    /// Check if this structure contains a block at the given world position
    pub fn has_block_at(&self, world_x: i32, world_y: i32, world_z: i32) -> Option<BlockType> {
        for block in &self.blocks {
            let block_world_x = self.world_x + block.relative_pos.0;
            let block_world_y = self.world_y + block.relative_pos.1;
            let block_world_z = self.world_z + block.relative_pos.2;

            if block_world_x == world_x && block_world_y == world_y && block_world_z == world_z {
                return Some(block.block_type);
            }
        }
        None
    }
}
