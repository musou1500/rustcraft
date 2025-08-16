# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Minecraft-like voxel game built in Rust using the wgpu graphics library. The game features:
- Chunk-based terrain generation with procedural noise
- Block breaking/placing mechanics with raycasting
- Real-time lighting
- Inventory system with 10 slots (keys 1-0)
- First-person camera with physics-based movement
- Texture atlas system for block rendering

## Build and Development Commands

```bash
# Build the project
cargo build

# Run the game
cargo run

# Build in release mode for better performance
cargo build --release
cargo run --release

# Check for compilation errors without building
cargo check
```

## Core Architecture

### Main Components

**Core System Files:**
- **main.rs**: Entry point, event loop, and main State struct that orchestrates all systems
- **world.rs**: High-level world management, chunk loading/unloading, and block modification
- **camera.rs**: First-person camera system with physics (gravity, jumping, collision detection)

**Terrain & Generation:**
- **terrain.rs**: Pure terrain generation with noise functions (height, biome, ore calculations)
- **chunk.rs**: Chunk data structures, generation orchestration, and mesh building with face culling
- **structures.rs**: Procedural structure generation system (trees, houses) with biome-aware placement

**Rendering & Graphics:**
- **voxel.rs**: Vertex data structures and cube mesh generation functions
- **texture_atlas.rs**: Manages block textures in a texture atlas
- **wireframe.rs**: Block selection wireframe overlay rendering

**Game Systems:**
- **blocks.rs**: Block type definitions, material properties, texture mapping registry, and generation logic
- **raycast.rs**: Ray-casting for block selection and interaction
- **slot_ui.rs**: Inventory slot rendering and UI management
- **light.rs**: Lighting system

**Debug & Development:**
- **chunk_debug.rs**: Debug visualization and chunk information display

### Rendering Pipeline

The game uses a single-pass rendering system:
1. **Main Pass**: Renders the world with lighting and UI elements

Shaders are located in src/ as .wgsl files:
- `shader.wgsl`: Main vertex/fragment shaders for world rendering
- `wireframe.wgsl`: Block selection wireframe rendering
- `slot_ui.wgsl`: Inventory slot rendering

### Key Systems

**Terrain Generation**: 
- 16x16 chunk system with 64-block world height limit (natural terrain limited to 24 blocks)
- Separated terrain generation from structure placement for better modularity
- Multi-octave Perlin noise for realistic height variation with centralized calculation methods
- Biome-aware block selection (snow, sand, grass) with dedicated biome noise
- Procedural structure system with trees (Oak, Birch, Pine) and houses (Small, Medium)
- Structure placement using dedicated noise and spacing algorithms with cross-chunk support
- Parallel chunk generation using rayon with pre-computed noise values
- Advanced face culling optimization for performance

**Block System**:
- Registry pattern for block types and properties
- Different textures per face (e.g., grass has green top, dirt sides)
- Material properties (hardness, transparency, emission)

**Physics**:
- Player collision detection with terrain
- Gravity and jumping mechanics
- Ground detection for landing

**Interaction**:
- Ray-casting for block selection (5-block reach)
- Left-click: break blocks (or place if slot has block)
- Right-click: pick up selected block into current slot
- Wireframe overlay shows selected block

## Development Notes

### Adding New Block Types
1. Add variant to `BlockType` enum in blocks.rs
2. Add corresponding `TextureId` if needed
3. Register the block in `BlockRegistry::register_defaults()`
4. Update generation logic in `blocks::generation` module if needed

### Performance Considerations
- Chunk loading/unloading happens dynamically based on camera position
- Face culling eliminates hidden block faces
- Parallel chunk generation prevents UI blocking
- Use `cargo run --release` for optimal performance

### Controls
- WASD: Movement
- Mouse: Look around  
- Space: Jump
- Ctrl: Run
- 1-0: Select inventory slots
- Left click: Break/place blocks
- Right click: Pick up blocks
- ESC: Toggle cursor lock/unlock
- F3: Toggle debug mode
- F5: Reload biome configuration from biome.toml

### Coordinate System
- X: East/West
- Y: Up/Down (0 = bottom, 63 = top for world height, natural terrain 0-23)
- Z: North/South
- Chunks are 16x16 blocks horizontally, 64 blocks tall (world height)
- Natural terrain generation limited to first 24 blocks vertically
- World coordinates are converted to chunk coordinates for terrain lookup

### Common Modifications
- Adjust `RENDER_DISTANCE` in world.rs to change view distance
- Modify noise parameters in `Terrain::calculate_height_at()` for different terrain generation
- Add new structure types by implementing the `Structure` trait in structures.rs
- Adjust structure placement frequency by modifying `should_place_structure()` thresholds
- Add new UI elements by following the pattern in slot_ui.rs
- Extend the block registry for new materials and textures

### Live Biome Configuration
The game now supports live reloading of biome configurations from `biome.toml`:

1. **Edit biome.toml**: Modify any biome parameters like height, frequency, amplitude, block types, temperature, humidity, tree density, or house spawn rates
2. **Press F5 in-game**: Instantly reload the configuration and regenerate all loaded chunks
3. **See changes immediately**: No need to restart the game or recompile

Example biome.toml modifications:
- Increase `amplitude` for more dramatic terrain variation
- Change `surface_block` to experiment with different biome appearances  
- Adjust `tree_density` to make forests denser or sparser
- Modify `base_height` to change biome elevation levels

**Note**: F5 clears all loaded chunks and regenerates them with the new configuration, so you'll see the changes applied to the current view area.