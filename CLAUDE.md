# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Minecraft-like voxel game built in Rust using the wgpu graphics library. The game features:
- Chunk-based terrain generation with procedural noise
- Block breaking/placing mechanics with raycasting
- Real-time lighting and shadow mapping
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

- **main.rs**: Entry point, event loop, and main State struct that orchestrates all systems
- **terrain.rs**: Chunk-based world generation with noise-based terrain using Perlin noise
- **camera.rs**: First-person camera system with physics (gravity, jumping, collision detection)  
- **voxel.rs**: Vertex data structures and cube mesh generation functions
- **blocks.rs**: Block type definitions, material properties, and texture mapping registry
- **raycast.rs**: Ray-casting for block selection and interaction
- **texture_atlas.rs**: Manages block textures in a texture atlas

### Rendering Pipeline

The game uses a dual-pass rendering system:
1. **Shadow Pass**: Renders depth information for shadow mapping
2. **Main Pass**: Renders the world with lighting, shadows, and UI elements

Shaders are located in src/ as .wgsl files:
- `shader.wgsl`: Main vertex/fragment shaders for world rendering
- `shadow.wgsl`: Shadow mapping shaders
- `wireframe.wgsl`: Block selection wireframe rendering
- `progress_ui.wgsl`: Loading screen UI
- `slot_ui.wgsl`: Inventory slot rendering

### Key Systems

**Terrain Generation**: 
- 16x16 chunk system with 24-block height limit
- Multi-octave Perlin noise for realistic height variation
- Biome-based block selection (snow, sand, grass)
- Parallel chunk generation using rayon
- Face culling optimization for performance

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

### Coordinate System
- X: East/West
- Y: Up/Down (0 = bottom, 23 = top)
- Z: North/South
- Chunks are 16x16 blocks horizontally, 24 blocks tall
- World coordinates are converted to chunk coordinates for terrain lookup

### Common Modifications
- Adjust `RENDER_DISTANCE` in terrain.rs to change view distance
- Modify noise parameters in `generate_chunk_data()` for different terrain
- Add new UI elements by following the pattern in slot_ui.rs/progress_ui.rs
- Extend the block registry for new materials and textures