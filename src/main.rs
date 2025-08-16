use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

mod biome;
mod blocks;
mod camera;
mod chunk;
mod chunk_debug;
mod light;
mod raycast;
mod slot_ui;
mod structures;
mod terrain;
mod texture_atlas;
mod texture_parser;
mod voxel;
mod wireframe;
mod world;

use biome::{Biome, BiomeManager};
use camera::CameraSystem;
use chunk_debug::ChunkDebugRenderer;
use light::DirectionalLight;
use raycast::{create_camera_ray, raycast_blocks, RaycastHit};
use slot_ui::SlotUI;
use texture_atlas::TextureAtlas;
use wireframe::WireframeRenderer;
use world::World;

struct State<'window> {
    surface: wgpu::Surface<'window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    camera: CameraSystem,
    world: World,
    light: DirectionalLight,
    render_pipeline: wgpu::RenderPipeline,
    texture_atlas: TextureAtlas,
    _texture_bind_group_layout: wgpu::BindGroupLayout,
    wireframe_renderer: WireframeRenderer,
    chunk_debug_renderer: ChunkDebugRenderer,
    slot_ui: SlotUI,
    window: &'window Window,
    game_mode: bool,
    window_focused: bool,
    selected_block: Option<RaycastHit>,
    debug_mode: bool,
    current_biome: Option<Biome>,
    biome_manager: BiomeManager,
}

impl<'window> State<'window> {
    async fn new(window: &'window Window) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let camera = CameraSystem::new(
            camera::Camera::new(
                cgmath::point3(0.0, 20.0, 0.0), // Higher spawn position
                cgmath::Deg(-90.0),
                cgmath::Deg(0.0),
                config.width as f32 / config.height as f32,
            ),
            &device,
        );

        let world = World::new();
        let light = DirectionalLight::new(&device);

        // Create texture atlas bind group layout
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        // Create texture atlas
        let texture_atlas = TextureAtlas::new(&device, &queue, &texture_bind_group_layout);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Main render pipeline layout
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &camera.bind_group_layout,
                    &light.bind_group_layout,
                    &texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let wireframe_renderer =
            WireframeRenderer::new(&device, surface_format, &camera.bind_group_layout);
        let chunk_debug_renderer =
            ChunkDebugRenderer::new(&device, surface_format, &camera.bind_group_layout);
        let slot_ui = SlotUI::new(
            &device,
            surface_format,
            &texture_atlas,
            config.width,
            config.height,
        );

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[voxel::Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            camera,
            world,
            light,
            render_pipeline,
            texture_atlas,
            _texture_bind_group_layout: texture_bind_group_layout,
            wireframe_renderer,
            chunk_debug_renderer,
            slot_ui,
            window,
            game_mode: true,
            window_focused: true,
            selected_block: None,
            debug_mode: false,
            current_biome: None,
            biome_manager: BiomeManager::load_from_file("biome.toml").unwrap_or_else(|e| {
                println!("Failed to load biome.toml: {}. Using default configs.", e);
                BiomeManager::new()
            }),
        })
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            // Update slot UI geometry for new window size (fixed 100px slots)
            self.slot_ui
                .update_geometry(&self.queue, new_size.width, new_size.height);
        }
    }

    fn input_window(&mut self, event: &WindowEvent) -> bool {
        // Handle slot selection first
        if let WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    state: ElementState::Pressed,
                    physical_key: PhysicalKey::Code(key_code),
                    ..
                },
            ..
        } = event
        {
            match key_code {
                KeyCode::Digit1 => {
                    self.slot_ui.set_selected_slot(0, &self.queue);
                    return true;
                }
                KeyCode::Digit2 => {
                    self.slot_ui.set_selected_slot(1, &self.queue);
                    return true;
                }
                KeyCode::Digit3 => {
                    self.slot_ui.set_selected_slot(2, &self.queue);
                    return true;
                }
                KeyCode::Digit4 => {
                    self.slot_ui.set_selected_slot(3, &self.queue);
                    return true;
                }
                KeyCode::Digit5 => {
                    self.slot_ui.set_selected_slot(4, &self.queue);
                    return true;
                }
                KeyCode::Digit6 => {
                    self.slot_ui.set_selected_slot(5, &self.queue);
                    return true;
                }
                KeyCode::Digit7 => {
                    self.slot_ui.set_selected_slot(6, &self.queue);
                    return true;
                }
                KeyCode::Digit8 => {
                    self.slot_ui.set_selected_slot(7, &self.queue);
                    return true;
                }
                KeyCode::Digit9 => {
                    self.slot_ui.set_selected_slot(8, &self.queue);
                    return true;
                }
                KeyCode::Digit0 => {
                    self.slot_ui.set_selected_slot(9, &self.queue);
                    return true;
                }
                KeyCode::Delete | KeyCode::Backspace => {
                    self.slot_ui.clear_selected_slot();
                    self.slot_ui.update_inventory_buffer(&self.queue);
                    return true;
                }
                KeyCode::F3 => {
                    self.debug_mode = !self.debug_mode;
                    println!("Debug mode: {}", if self.debug_mode { "ON" } else { "OFF" });
                    return true;
                }
                KeyCode::F5 => {
                    match self.biome_manager.reload_from_file("biome.toml") {
                        Ok(()) => {
                            // Clear and regenerate all chunks
                            self.world.clear_all_chunks();
                            println!("Biome configuration reloaded! All chunks regenerated.");
                        }
                        Err(e) => {
                            println!("Failed to reload biome.toml: {}", e);
                        }
                    }
                    return true;
                }
                _ => {}
            }
        }

        // Handle mouse clicks for game mode resumption
        if let WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button: MouseButton::Left,
            ..
        } = event
        {
            // If in menu mode and window is focused, resume game
            if !self.game_mode && self.window_focused {
                self.game_mode = true;
                self.camera.reset_mouse_deltas(); // Clear accumulated mouse movement
                self.update_cursor_state();
                println!("🎮 Game resumed!");
                return true;
            }
        }

        // If not a slot key or resume click, pass to camera
        self.camera.process_window_events(event)
    }

    fn input_device(&mut self, event: &DeviceEvent) -> bool {
        // Only process mouse movement when in game mode and window is focused
        if self.game_mode && self.window_focused {
            self.camera.process_device_events(event)
        } else {
            false
        }
    }

    fn toggle_game_mode(&mut self) {
        self.game_mode = !self.game_mode;
        self.update_cursor_state();
    }

    fn update_cursor_state(&mut self) {
        if self.game_mode && self.window_focused {
            // Game mode: center cursor, confine to window and hide it
            let window_size = self.window.inner_size();
            let center_x = window_size.width as f64 / 2.0;
            let center_y = window_size.height as f64 / 2.0;
            let _ = self
                .window
                .set_cursor_position(winit::dpi::PhysicalPosition::new(center_x, center_y));
            let _ = self
                .window
                .set_cursor_grab(winit::window::CursorGrabMode::Confined);
            self.window.set_cursor_visible(false);
        } else {
            // Menu mode: free cursor and show it
            let _ = self
                .window
                .set_cursor_grab(winit::window::CursorGrabMode::None);
            self.window.set_cursor_visible(true);
        }
    }

    fn update(&mut self, dt: std::time::Duration) {
        self.camera.update(dt, &self.world);
        self.camera.update_buffer(&self.queue);
        self.light.update_buffer(&self.queue);

        let camera_pos = self.camera.get_position();
        self.world
            .update(camera_pos, &self.device, &self.biome_manager);

        // Check for biome changes
        let world_x = camera_pos.x.floor() as i32;
        let world_z = camera_pos.z.floor() as i32;
        let current_biome = self.world.get_terrain().biome_at(world_x, world_z);

        if self.current_biome != Some(current_biome) {
            println!("Entered {} biome", current_biome.name());
            self.current_biome = Some(current_biome);
        }

        // Update chunk debug renderer if debug mode is enabled
        if self.debug_mode {
            let chunk_positions = self.world.get_loaded_chunk_positions();
            self.chunk_debug_renderer
                .update_chunks(&self.device, &chunk_positions);
        }

        // Update block selection (only when in game mode and window focused)
        if self.game_mode && self.window_focused {
            self.update_block_selection();

            // Check for block interaction (place or break)
            if self.camera.was_left_mouse_clicked() {
                self.handle_left_click();
            }

            // Check for putting block in slot
            if self.camera.was_right_mouse_clicked() {
                self.put_selected_block_in_slot();
            }
        }
    }

    fn update_block_selection(&mut self) {
        let camera_pos = self.camera.get_position();
        let camera_yaw = self.camera.get_yaw();
        let camera_pitch = self.camera.get_pitch();
        let ray = create_camera_ray(camera_pos, camera_yaw, camera_pitch);
        let new_selection = raycast_blocks(ray, 5.0, &self.world); // 5 block reach distance
        self.selected_block = new_selection;
    }

    fn handle_left_click(&mut self) {
        if let Some(hit) = self.selected_block {
            // Check if current slot has a block
            if let Some(block_type) = self.slot_ui.get_block_in_selected_slot() {
                // Place block mode
                self.place_block_from_slot(hit, block_type);
            } else {
                // Remove block mode (original behavior)
                println!("Breaking block at: {:?}", hit.block_pos);

                // Actually remove the block from terrain
                let removed_block_type = self.world.remove_block(
                    hit.block_pos[0],
                    hit.block_pos[1],
                    hit.block_pos[2],
                    &self.device,
                );

                if let Some(block_type) = removed_block_type {
                    println!(
                        "Successfully removed {:?} block at: {:?}",
                        block_type, hit.block_pos
                    );
                    // Clear selection since the block is gone
                    self.selected_block = None;
                } else {
                    println!("Failed to remove block at: {:?}", hit.block_pos);
                }
            }
        }
    }

    fn place_block_from_slot(&mut self, hit: raycast::RaycastHit, block_type: blocks::BlockType) {
        // Calculate placement position based on face normal
        let placement_pos = [
            hit.block_pos[0] + hit.face_normal.x as i32,
            hit.block_pos[1] + hit.face_normal.y as i32,
            hit.block_pos[2] + hit.face_normal.z as i32,
        ];

        println!(
            "Attempting to place {:?} block at: {:?}",
            block_type, placement_pos
        );

        // Validate placement position
        if !self.is_valid_placement_position(placement_pos) {
            println!("Invalid placement position!");
            return;
        }

        // Add block to terrain
        let success = self.world.add_block(
            placement_pos[0],
            placement_pos[1],
            placement_pos[2],
            block_type,
            &self.device,
        );

        if success {
            println!(
                "Successfully placed {:?} block at: {:?}",
                block_type, placement_pos
            );
            // Note: We don't remove the block from inventory (infinite blocks)
        } else {
            println!("Failed to place block at: {:?}", placement_pos);
        }
    }

    fn is_valid_placement_position(&self, pos: [i32; 3]) -> bool {
        // Check if position is within world bounds
        if pos[1] < 0 || pos[1] >= chunk::WORLD_HEIGHT as i32 {
            return false;
        }

        // Check if player would collide with placed block
        let player_eye_pos = self.camera.get_position();
        // Convert eye position to feet position (eyes are 1.6 blocks above feet)
        let player_feet_y = player_eye_pos.y - 1.6;
        let player_block_x = player_eye_pos.x.floor() as i32;
        let player_feet_block_y = player_feet_y.floor() as i32;
        let player_head_block_y = (player_feet_y + 1.8).floor() as i32;
        let player_block_z = player_eye_pos.z.floor() as i32;

        // Player occupies space from feet to head (1.8 blocks tall)
        if pos[0] == player_block_x && pos[2] == player_block_z {
            for y in player_feet_block_y..=player_head_block_y {
                if pos[1] == y {
                    println!("Cannot place block inside player position!");
                    return false;
                }
            }
        }

        true
    }

    fn put_selected_block_in_slot(&mut self) {
        if let Some(hit) = self.selected_block {
            // Get the block type at the selected position
            if let Some(block_type) =
                self.world
                    .get_block_type(hit.block_pos[0], hit.block_pos[1], hit.block_pos[2])
            {
                // Don't put air blocks in slots
                if block_type != blocks::BlockType::Air {
                    self.slot_ui
                        .put_block_in_selected_slot(block_type, &self.queue);
                    println!(
                        "Put {:?} block in slot {}",
                        block_type,
                        self.slot_ui.get_selected_slot()
                    );
                }
            }
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: None,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Main render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.5,
                            g: 0.8,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Render normal terrain
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera.bind_group, &[]);
            render_pass.set_bind_group(1, &self.light.bind_group, &[]);
            render_pass.set_bind_group(2, &self.texture_atlas.bind_group, &[]);
            self.world.render(&mut render_pass);

            // Render block selection wireframe
            if let Some(hit) = self.selected_block {
                // Debug: Print when wireframe is being rendered
                static mut LAST_WIREFRAME_POS: Option<[i32; 3]> = None;
                unsafe {
                    if LAST_WIREFRAME_POS != Some(hit.block_pos) {
                        LAST_WIREFRAME_POS = Some(hit.block_pos);
                    }
                }

                self.wireframe_renderer.update_position(
                    &self.queue,
                    hit.block_pos[0] as f32,
                    hit.block_pos[1] as f32,
                    hit.block_pos[2] as f32,
                );
                self.wireframe_renderer
                    .render(&mut render_pass, &self.camera.bind_group);
            }

            // Render chunk boundaries if debug mode is enabled
            if self.debug_mode {
                self.chunk_debug_renderer
                    .render(&mut render_pass, &self.camera.bind_group);
            }

            // Always render slot UI on top
            self.slot_ui.render(&mut render_pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    println!("🎮 Starting Voxel Game...");

    // Initialize the block registry
    blocks::init_block_registry();

    let event_loop = EventLoop::new()?;
    let window = winit::window::WindowBuilder::new()
        .with_title("Voxel Game")
        .with_inner_size(winit::dpi::LogicalSize::new(1280, 800))
        .build(&event_loop)?;

    // Properly confine the cursor for FPS-style camera movement
    // Center the cursor first, then confine it within window bounds
    let window_size = window.inner_size();
    let center_x = window_size.width as f64 / 2.0;
    let center_y = window_size.height as f64 / 2.0;
    let _ = window.set_cursor_position(winit::dpi::PhysicalPosition::new(center_x, center_y));
    let _ = window.set_cursor_grab(winit::window::CursorGrabMode::Confined);
    window.set_cursor_visible(false);

    let window_id = window.id();
    let mut state = pollster::block_on(State::new(&window))?;
    let mut last_render_time = std::time::Instant::now();

    println!("🌍 Use WASD to move, mouse to look around, Space to jump, Ctrl to run");
    println!("🖱️  Press ESC to pause/resume game, ESC again in pause mode to exit");
    println!("🔨 Left click to break blocks (bright red outline shows selected block)");
    println!("📦 Right click to put selected block into current inventory slot");
    println!("🎒 Use number keys 1-0 to select inventory slots (1=leftmost, 0=rightmost)");

    event_loop.run(move |event, elwt| {
        match event {
            Event::DeviceEvent { ref event, .. } => {
                state.input_device(event);
            }
            Event::WindowEvent {
                ref event,
                window_id: w_id,
            } if w_id == window_id => {
                if !state.input_window(event) {
                    match event {
                        WindowEvent::CloseRequested => elwt.exit(),
                        WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    state: ElementState::Pressed,
                                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                                    ..
                                },
                            ..
                        } => {
                            if state.game_mode {
                                // In game mode: pause game (enter menu mode)
                                state.game_mode = false;
                                state.update_cursor_state();
                                println!("🎮 Game paused. Click on window to resume or press ESC again to exit.");
                            } else {
                                // In menu mode: exit game
                                elwt.exit();
                            }
                        }
                        WindowEvent::Focused(focused) => {
                            state.window_focused = *focused;
                            // Auto-pause when window loses focus
                            if !focused && state.game_mode {
                                state.game_mode = false;
                                println!("🎮 Game auto-paused (window unfocused). Click to resume.");
                            }
                            state.update_cursor_state();
                        }
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::RedrawRequested => {
                            let now = std::time::Instant::now();
                            let dt = now - last_render_time;
                            last_render_time = now;

                            state.update(dt);
                            match state.render() {
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                                Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                                Err(e) => eprintln!("{:?}", e),
                            }
                        }
                        _ => {}
                    }
                }
            }
            Event::AboutToWait => {
                state.window.request_redraw();
            }
            _ => {}
        }
        elwt.set_control_flow(ControlFlow::Poll);
    })?;

    Ok(())
}
