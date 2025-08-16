use crate::voxel::FaceTextures;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents different types of blocks in the world
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockType {
    Air,
    Stone,
    Dirt,
    Grass,
    Sand,
    Water,
    Wood,
    Leaves,
    Snow,
    Planks,
    Cobblestone,
    Glass,
}

/// Texture atlas indices for different block textures
#[derive(Debug, Clone, Copy)]
pub enum TextureId {
    Stone = 0,
    Dirt = 1,
    GrassTop = 2,
    GrassSide = 3,
    Sand = 4,
    Water = 5,
    WoodTop = 6,
    WoodSide = 7,
    Leaves = 8,
    Snow = 9,
    Bedrock = 10,
    Planks = 11,
    Cobblestone = 12,
    Glass = 13,
}

/// Material properties for a block type
#[derive(Debug, Clone)]
pub struct BlockMaterial {
    pub name: &'static str,
    pub textures: FaceTextures,
    pub hardness: f32,
    pub is_solid: bool,
    pub is_transparent: bool,
    pub emission: f32, // For glowing blocks
}

/// Registry for all block types and their properties
pub struct BlockRegistry {
    materials: HashMap<BlockType, BlockMaterial>,
}

impl BlockRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            materials: HashMap::new(),
        };

        // Register default block types
        registry.register_defaults();
        registry
    }

    /// Register a new block type with its material properties
    pub fn register(&mut self, block_type: BlockType, material: BlockMaterial) {
        self.materials.insert(block_type, material);
    }

    /// Get material properties for a block type
    pub fn get_material(&self, block_type: BlockType) -> Option<&BlockMaterial> {
        self.materials.get(&block_type)
    }

    /// Get the texture mapping for a block type
    pub fn get_textures(&self, block_type: BlockType) -> FaceTextures {
        self.materials
            .get(&block_type)
            .map(|m| m.textures)
            .unwrap_or(FaceTextures::all_same(TextureId::Stone as u32)) // Stone for missing blocks
    }

    /// Check if a block is solid
    pub fn is_solid(&self, block_type: BlockType) -> bool {
        self.materials
            .get(&block_type)
            .map(|m| m.is_solid)
            .unwrap_or(false)
    }

    /// Register all default block types
    fn register_defaults(&mut self) {
        // Air - invisible, non-solid
        self.register(
            BlockType::Air,
            BlockMaterial {
                name: "Air",
                textures: FaceTextures::all_same(TextureId::Stone as u32), // Air doesn't render anyway
                hardness: 0.0,
                is_solid: false,
                is_transparent: true,
                emission: 0.0,
            },
        );

        // Stone - gray, very hard
        self.register(
            BlockType::Stone,
            BlockMaterial {
                name: "Stone",
                textures: FaceTextures::all_same(TextureId::Stone as u32),
                hardness: 3.0,
                is_solid: true,
                is_transparent: false,
                emission: 0.0,
            },
        );

        // Dirt - brown, medium hardness
        self.register(
            BlockType::Dirt,
            BlockMaterial {
                name: "Dirt",
                textures: FaceTextures::all_same(TextureId::Dirt as u32),
                hardness: 1.0,
                is_solid: true,
                is_transparent: false,
                emission: 0.0,
            },
        );

        // Grass - green top, dirt sides
        self.register(
            BlockType::Grass,
            BlockMaterial {
                name: "Grass",
                textures: FaceTextures::new(
                    TextureId::GrassSide as u32, // front
                    TextureId::GrassSide as u32, // back
                    TextureId::GrassSide as u32, // left
                    TextureId::GrassSide as u32, // right
                    TextureId::GrassTop as u32,  // top
                    TextureId::Dirt as u32,      // bottom
                ),
                hardness: 1.0,
                is_solid: true,
                is_transparent: false,
                emission: 0.0,
            },
        );

        // Sand - yellowish, easy to break
        self.register(
            BlockType::Sand,
            BlockMaterial {
                name: "Sand",
                textures: FaceTextures::all_same(TextureId::Sand as u32),
                hardness: 0.8,
                is_solid: true,
                is_transparent: false,
                emission: 0.0,
            },
        );

        // Water - blue, transparent
        self.register(
            BlockType::Water,
            BlockMaterial {
                name: "Water",
                textures: FaceTextures::all_same(TextureId::Water as u32),
                hardness: 0.0,
                is_solid: false,
                is_transparent: true,
                emission: 0.0,
            },
        );

        // Wood - brown, medium hardness
        self.register(
            BlockType::Wood,
            BlockMaterial {
                name: "Wood",
                textures: FaceTextures::new(
                    TextureId::WoodSide as u32, // front
                    TextureId::WoodSide as u32, // back
                    TextureId::WoodSide as u32, // left
                    TextureId::WoodSide as u32, // right
                    TextureId::WoodTop as u32,  // top
                    TextureId::WoodTop as u32,  // bottom
                ),
                hardness: 2.0,
                is_solid: true,
                is_transparent: false,
                emission: 0.0,
            },
        );

        // Leaves - green, soft
        self.register(
            BlockType::Leaves,
            BlockMaterial {
                name: "Leaves",
                textures: FaceTextures::all_same(TextureId::Leaves as u32),
                hardness: 0.3,
                is_solid: true,
                is_transparent: true,
                emission: 0.0,
            },
        );


        // Snow - white, soft
        self.register(
            BlockType::Snow,
            BlockMaterial {
                name: "Snow",
                textures: FaceTextures::all_same(TextureId::Snow as u32),
                hardness: 0.2,
                is_solid: true,
                is_transparent: false,
                emission: 0.0,
            },
        );

        // Planks - wooden planks for construction
        self.register(
            BlockType::Planks,
            BlockMaterial {
                name: "Planks",
                textures: FaceTextures::all_same(TextureId::Planks as u32),
                hardness: 2.0,
                is_solid: true,
                is_transparent: false,
                emission: 0.0,
            },
        );

        // Cobblestone - stone blocks for construction
        self.register(
            BlockType::Cobblestone,
            BlockMaterial {
                name: "Cobblestone",
                textures: FaceTextures::all_same(TextureId::Cobblestone as u32),
                hardness: 3.5,
                is_solid: true,
                is_transparent: false,
                emission: 0.0,
            },
        );

        // Glass - transparent windows
        self.register(
            BlockType::Glass,
            BlockMaterial {
                name: "Glass",
                textures: FaceTextures::all_same(TextureId::Glass as u32),
                hardness: 0.5,
                is_solid: true,
                is_transparent: true,
                emission: 0.0,
            },
        );
    }
}


use std::sync::OnceLock;

/// Global block registry instance
static BLOCK_REGISTRY: OnceLock<BlockRegistry> = OnceLock::new();

/// Initialize the global block registry
pub fn init_block_registry() {
    BLOCK_REGISTRY.get_or_init(BlockRegistry::new);
}

/// Get reference to the global block registry
pub fn get_block_registry() -> &'static BlockRegistry {
    BLOCK_REGISTRY
        .get()
        .expect("Block registry not initialized")
}
