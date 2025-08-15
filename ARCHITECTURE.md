# Rustcraft Architecture

This document describes the architecture of Rustcraft, a Minecraft-like voxel game built in Rust using the wgpu graphics library.

## System Overview

Rustcraft is designed with clear domain separation and modular architecture. The system follows several key architectural principles:

- **Domain Separation**: Each major concern (terrain, structures, rendering, etc.) is isolated into its own module
- **Orchestration Pattern**: Higher-level modules coordinate between specialized subsystems
- **Registry Pattern**: Centralized configuration and lookup for shared resources
- **Component-Based Architecture**: The main State struct owns all systems as components
- **Configuration-Driven Generation**: Biome configurations drive terrain shape, not just cosmetic changes

**Technology Stack:**
- **Rust** - Systems programming language for performance and safety
- **wgpu** - Modern graphics API for cross-platform GPU access
- **winit** - Window management and input handling
- **cgmath** - 3D mathematics library
- **noise** - Procedural generation algorithms

## Architectural Domains

### Application Layer

#### Main Orchestrator (`main.rs`)
**Responsibility:** Central coordination of all game systems

**Key Components:**
- `State` struct - Owns all subsystems as components
- Event loop management via winit
- Update/render cycle orchestration
- Input event routing to appropriate systems

**Dependencies:** All other modules (acts as the integration layer)

---

### World Generation Domains

The world generation system is architected as four separate, coordinated domains:

#### Biome System (`biome.rs`)
**Responsibility:** Biome classification, configuration, and selection

**Key Components:**
- `Biome` enum - Explicit biome types (Plains, Desert, Mountain, Tundra, Forest, Swamp, etc.)
- `BiomeConfig` struct - Per-biome configuration:
  - Terrain shape parameters (base_height, height_variation, roughness)
  - Block palette (surface_block, subsurface_block, stone_block)
  - Temperature and humidity ranges
  - Structure spawn rates by type
- `BiomeSelector` - Determines biome from world position

**Key Functions:**
- `select_biome(x, z, temperature, humidity)` - Returns Biome enum
- `get_config(biome)` - Returns BiomeConfig for terrain generation
- `get_blend_factor(biome1, biome2, distance)` - Smooth transitions

**Dependencies:** None (pure configuration domain)

**Design Notes:**
- Configuration-driven approach for terrain variation
- Each biome defines how terrain should be shaped
- Supports biome blending at boundaries

#### Terrain Generation (`terrain.rs`)
**Responsibility:** Terrain calculation with biome-aware shaping and block selection

**Key Components:**
- `Terrain` struct - Terrain generator with noise functions
- Temperature and humidity noise for biome selection
- Biome-specific height calculation
- Block selection logic (moved from blocks::generation)

**Key Functions:**
- `calculate_height_at(x, z, biome_config)` - Biome-shaped terrain height
- `select_biome_at(x, z)` - Determines biome using temperature/humidity
- `generate_terrain_blocks()` - Creates terrain with biome-appropriate blocks
- `get_block_for_position(x, y, z, height, biome)` - Block type selection
- `apply_biome_surface(base_block, biome, y, height)` - Surface modifications

**Dependencies:** 
- `blocks` (for BlockType enum only)
- `biome` (for BiomeConfig and selection)

**Design Notes:**
- Contains all logic for converting position/height/biome to blocks
- Biome configs modify terrain shape, not just block types
- Mountains: height_variation *= 2.0, roughness *= 1.5
- Deserts: height_variation *= 0.5, smooth terrain
- Natural terrain height varies by biome (mountains: 0-40, plains: 0-20, etc.)

#### Structure Generation (`structures.rs`)
**Responsibility:** Procedural structure placement and generation

**Key Components:**
- `StructureGenerator` - Manages structure placement logic
- `TreeStructure` - Oak, Birch, Pine tree generation
- `HouseStructure` - Small, Medium house generation
- `PlacedStructure` - Positioned structure instances

**Key Functions:**
- `generate_structures_for_chunk()` - Determines structure placement
- `should_place_structure()` - Spacing and placement rules
- Structure-specific generation methods for trees and houses

**Dependencies:** 
- `blocks` (for BlockType)
- `terrain` (reads terrain data for placement decisions)
- `biome` (for biome-specific structures)

**Design Notes:**
- Separate noise functions from terrain for independent structure placement
- Cross-chunk structure support for large buildings
- Uses explicit Biome enum for structure selection
- Biome-specific structure types (cacti in deserts, pine forests in tundra)
- Structure density varies by biome configuration

#### Chunk Orchestration (`chunk.rs`, `world.rs`)
**Responsibility:** Combines terrain and structures into final world data

**Key Components (`chunk.rs`):**
- `ChunkGenerator` - Orchestrates the generation pipeline
- `Chunk` - Contains block data and GPU mesh
- `ChunkData` - Raw block data before mesh generation

**Key Components (`world.rs`):**
- `World` - High-level chunk management
- Dynamic chunk loading/unloading based on camera position
- Block modification API (add/remove blocks)

**Generation Pipeline:**
1. `BiomeSelector::select_biome()` - Determine biome from temperature/humidity
2. `Biome::get_config()` - Get biome-specific terrain parameters
3. `Terrain::calculate_height_at()` - Generate biome-shaped terrain
4. `Terrain::generate_terrain_blocks()` - Fill chunk with appropriate blocks
5. `StructureGenerator::generate_structures()` - Add biome structures
6. `Chunk::build_mesh()` - Create GPU-ready geometry

**Dependencies:** `blocks`, `biome`, `terrain`, `structures`, `voxel`

---

### Game Logic Domains

#### Block System (`blocks.rs`)
**Responsibility:** Block type definitions and property registry

**Key Components:**
- `BlockType` enum - All block types (Stone, Dirt, Grass, etc.)
- `BlockMaterial` - Physical properties (hardness, transparency, emission)
- `BlockRegistry` - Singleton registry for block lookups
- `TextureId` and `FaceTextures` - Visual properties

**Architectural Pattern:**
- Pure data domain - no generation logic
- Registry pattern with global singleton access via `get_block_registry()`
- Lazy initialization on first access
- Centralized texture mapping via `FaceTextures`
- Defines WHAT blocks exist, not WHERE they go

**Dependencies:** `voxel` (for FaceTextures only)

**Design Notes:**
- No longer contains generation logic (moved to terrain.rs)
- Focused solely on block definitions and properties
- Clear separation of concerns

#### Physics & Movement (`camera.rs`)
**Responsibility:** Player movement, view control, and physics simulation

**Key Components:**
- `Camera` - View and projection matrices
- `CameraController` - Input handling (WASD, mouse)
- `CameraSystem` - Combines camera + controller with physics

**Physics Features:**
- Gravity and jumping mechanics
- Collision detection with terrain blocks
- Ground detection for landing
- Smooth movement with frame-rate independence

**Dependencies:** `world` (for collision queries via `is_block_solid()`)

#### Interaction System (`raycast.rs`)
**Responsibility:** Block selection and targeting

**Key Components:**
- DDA (Digital Differential Analyzer) raycasting algorithm
- `RaycastHit` - Hit position, face normal, and block coordinates
- 5-block reach distance for player interactions

**Key Functions:**
- `create_camera_ray()` - Converts view direction to ray
- `raycast_blocks()` - Finds first solid block intersection

**Dependencies:** `world` (for block queries)

---

### Rendering Domains

#### Mesh Generation (`voxel.rs`)
**Responsibility:** Converting block data to GPU-ready geometry

**Key Components:**
- `Vertex` struct - Position, texture coordinates, normals
- `FaceTextures` - Per-face texture mapping for blocks
- Face culling optimization (hidden faces eliminated)

**Key Functions:**
- `create_cube_vertices_selective()` - Generates only visible faces
- `create_cube_indices_selective()` - Index buffer for selected faces
- Texture coordinate calculation for atlas mapping

**Performance Optimizations:**
- Only generates faces adjacent to air blocks
- Efficient vertex sharing between faces
- GPU-optimized vertex layout with bytemuck

**Dependencies:** None (pure geometry functions)

#### Texture Management (`texture_atlas.rs`)
**Responsibility:** GPU texture resource management

**Key Components:**
- `TextureAtlas` - Manages texture data and GPU resources
- Procedural texture generation (16x16 pixel textures)
- Texture atlas organization for efficient GPU access

**Features:**
- Automatic bind group creation for shaders
- Texture atlas coordinates for block face mapping
- GPU memory management

#### Lighting System (`light.rs`)
**Responsibility:** Illumination and shadow mapping

**Key Components:**
- `DirectionalLight` - Sun-like lighting with shadows
- Shadow mapping with depth texture
- Light space matrix calculation for shadow projection

**Rendering Integration:**
- Two-pass rendering: shadow pass → main pass
- Shadow texture binding for main shader
- Orthographic projection for consistent shadows

#### Debug Rendering (`wireframe.rs`, `chunk_debug.rs`)
**Responsibility:** Development and debugging visualization

**Key Components:**
- `WireframeRenderer` - Block selection outline
- `ChunkDebugRenderer` - Chunk boundary visualization
- Separate render pipelines for debug overlays

---

### UI Layer

#### Inventory System (`slot_ui.rs`)
**Responsibility:** Player inventory and HUD rendering

**Key Components:**
- `SlotUI` - 10-slot inventory management
- Block storage and selection (keys 1-0)
- HUD rendering with separate shader pipeline

**Features:**
- Visual slot highlighting for selected inventory position
- Block type storage and retrieval
- UI rendering overlay on main game view
