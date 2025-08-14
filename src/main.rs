use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

mod blocks;
mod camera;
mod chunk;
mod chunk_debug;
mod light;
mod progress_ui;
mod raycast;
mod slot_ui;
mod structures;
mod terrain;
mod texture_atlas;
mod voxel;
mod wireframe;
mod world;

use camera::CameraSystem;
use chunk_debug::ChunkDebugRenderer;
use light::DirectionalLight;
use progress_ui::ProgressUI;
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
    shadow_pipeline: wgpu::RenderPipeline,
    _shadow_texture: wgpu::Texture,
    shadow_view: wgpu::TextureView,
    _shadow_sampler: wgpu::Sampler,
    shadow_bind_group: wgpu::BindGroup,
    _shadow_bind_group_layout: wgpu::BindGroupLayout,
    texture_atlas: TextureAtlas,
    _texture_bind_group_layout: wgpu::BindGroupLayout,
    wireframe_renderer: WireframeRenderer,
    chunk_debug_renderer: ChunkDebugRenderer,
    progress_ui: ProgressUI,
    slot_ui: SlotUI,
    window: &'window Window,
    last_progress_output: f32,
    last_progress_indices: u32,
    cursor_locked: bool,
    window_focused: bool,
    selected_block: Option<RaycastHit>,
    debug_mode: bool,
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

        // Create shadow map texture
        const SHADOW_MAP_SIZE: u32 = 2048;
        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: SHADOW_MAP_SIZE,
                height: SHADOW_MAP_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("Shadow Map"),
            view_formats: &[],
        });
        let shadow_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        let shadow_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Depth,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                ],
                label: Some("shadow_bind_group_layout"),
            });

        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &shadow_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
            ],
            label: Some("shadow_bind_group"),
        });

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

        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shadow Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shadow.wgsl").into()),
        });

        // Shadow pipeline layout
        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Shadow Pipeline Layout"),
                bind_group_layouts: &[&light.bind_group_layout],
                push_constant_ranges: &[],
            });

        // Main render pipeline layout
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &camera.bind_group_layout,
                    &light.bind_group_layout,
                    &shadow_bind_group_layout,
                    &texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        // Shadow pipeline
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shadow Pipeline"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shadow_shader,
                entry_point: "vs_main",
                buffers: &[voxel::Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shadow_shader,
                entry_point: "fs_main",
                targets: &[],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Front), // Front face culling for shadow mapping
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

        let wireframe_renderer =
            WireframeRenderer::new(&device, surface_format, &camera.bind_group_layout);
        let chunk_debug_renderer =
            ChunkDebugRenderer::new(&device, surface_format, &camera.bind_group_layout);
        let progress_ui = ProgressUI::new(&device, surface_format);
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
                    blend: Some(wgpu::BlendState::REPLACE),
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
            shadow_pipeline,
            _shadow_texture: shadow_texture,
            shadow_view,
            _shadow_sampler: shadow_sampler,
            shadow_bind_group,
            _shadow_bind_group_layout: shadow_bind_group_layout,
            texture_atlas,
            _texture_bind_group_layout: texture_bind_group_layout,
            wireframe_renderer,
            chunk_debug_renderer,
            progress_ui,
            slot_ui,
            window,
            last_progress_output: -1.0,
            last_progress_indices: 0,
            cursor_locked: true,
            window_focused: true,
            selected_block: None,
            debug_mode: false,
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
                _ => {}
            }
        }

        // If not a slot key, pass to camera
        self.camera.process_window_events(event)
    }

    fn input_device(&mut self, event: &DeviceEvent) -> bool {
        // Only process mouse movement when cursor is locked and window is focused
        if self.cursor_locked && self.window_focused {
            self.camera.process_device_events(event)
        } else {
            false
        }
    }

    fn toggle_cursor_lock(&mut self) {
        self.cursor_locked = !self.cursor_locked;
        self.update_cursor_state();
    }

    fn update_cursor_state(&mut self) {
        if self.cursor_locked && self.window_focused {
            // Game mode: confine cursor to window and hide it
            let _ = self
                .window
                .set_cursor_grab(winit::window::CursorGrabMode::Confined);
            self.window.set_cursor_visible(false);
        } else {
            // Desktop mode: free cursor and show it
            let _ = self
                .window
                .set_cursor_grab(winit::window::CursorGrabMode::None);
            self.window.set_cursor_visible(true);

            // Center cursor when switching to desktop mode
            if !self.cursor_locked {
                let window_size = self.window.inner_size();
                let center_x = window_size.width as f64 / 2.0;
                let center_y = window_size.height as f64 / 2.0;
                let _ = self
                    .window
                    .set_cursor_position(winit::dpi::PhysicalPosition::new(center_x, center_y));
            }
        }
    }

    fn update(&mut self, dt: std::time::Duration) {
        self.camera.update(dt, &self.world);
        self.camera.update_buffer(&self.queue);
        self.light.update_buffer(&self.queue);

        let camera_pos = self.camera.get_position();
        self.world.update(camera_pos, &self.device);

        // Update chunk debug renderer if debug mode is enabled
        if self.debug_mode {
            let chunk_positions = self.world.get_loaded_chunk_positions();
            self.chunk_debug_renderer
                .update_chunks(&self.device, &chunk_positions);
        }

        // Update block selection (only when cursor is locked and window focused)
        if self.cursor_locked && self.window_focused {
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

        // Debug: Print when selection changes
        match (&self.selected_block, &new_selection) {
            (None, Some(hit)) => {
                // Verify this is actually a solid block
                let is_solid =
                    self.world
                        .is_block_solid(hit.block_pos[0], hit.block_pos[1], hit.block_pos[2]);
                println!(
                    "Block selected at: {:?} (is_solid: {})",
                    hit.block_pos, is_solid
                );
                if !is_solid {
                    println!("WARNING: Selected block is not solid! This shouldn't happen.");
                }
            }
            (Some(_), None) => {
                println!("Block deselected");
            }
            (Some(old), Some(new)) if old.block_pos != new.block_pos => {
                let is_solid =
                    self.world
                        .is_block_solid(new.block_pos[0], new.block_pos[1], new.block_pos[2]);
                println!(
                    "Block selection changed from {:?} to {:?} (is_solid: {})",
                    old.block_pos, new.block_pos, is_solid
                );
                if !is_solid {
                    println!("WARNING: Selected block is not solid! This shouldn't happen.");
                }
            }
            _ => {} // No change
        }

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
        // Update progress UI
        let progress = self.world.progress.get_progress();
        let is_generating = self.world.progress.is_generating;

        // Calculate number of indices for rendering
        if is_generating {
            // Background bar (6 indices) + progress fill (6 indices if progress > 0) + loading text (~200 indices)
            let progress_fill_indices = if progress > 0.0 { 6 } else { 0 };
            self.last_progress_indices = 6 + progress_fill_indices + 210; // Approximate for loading text

            self.progress_ui
                .update_progress(&self.queue, progress, is_generating);

            // Update window title
            if progress >= 1.0 {
                self.window.set_title("Voxel Game - Ready!");
            } else {
                self.window.set_title(&format!(
                    "Voxel Game - Generating Terrain {:.0}%",
                    progress * 100.0
                ));
            }
        } else {
            self.last_progress_indices = 0;
            if self.last_progress_output != -1.0 {
                self.window.set_title("Voxel Game");
                self.last_progress_output = -1.0;
            }
        }

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

        // Shadow pass
        {
            let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Shadow Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            shadow_pass.set_pipeline(&self.shadow_pipeline);
            shadow_pass.set_bind_group(0, &self.light.bind_group, &[]);
            self.world.render(&mut shadow_pass);
        }

        // Main render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(if self.world.progress.is_generating {
                            // Darker background during generation
                            wgpu::Color {
                                r: 0.2,
                                g: 0.3,
                                b: 0.4,
                                a: 1.0,
                            }
                        } else {
                            // Normal sky color
                            wgpu::Color {
                                r: 0.5,
                                g: 0.8,
                                b: 1.0,
                                a: 1.0,
                            }
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

            if self.world.progress.is_generating {
                // Render progress UI during generation
                self.progress_ui.render(
                    &mut render_pass,
                    is_generating,
                    self.last_progress_indices,
                );
            } else {
                // Render normal terrain
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.camera.bind_group, &[]);
                render_pass.set_bind_group(1, &self.light.bind_group, &[]);
                render_pass.set_bind_group(2, &self.shadow_bind_group, &[]);
                render_pass.set_bind_group(3, &self.texture_atlas.bind_group, &[]);
                self.world.render(&mut render_pass);

                // Render block selection wireframe
                if let Some(hit) = self.selected_block {
                    // Debug: Print when wireframe is being rendered
                    static mut LAST_WIREFRAME_POS: Option<[i32; 3]> = None;
                    unsafe {
                        if LAST_WIREFRAME_POS != Some(hit.block_pos) {
                            println!("Rendering wireframe at: {:?}", hit.block_pos);
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
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
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
    println!("🖱️  Press ESC to toggle cursor lock/unlock (currently locked)");
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
                            if state.cursor_locked {
                                state.toggle_cursor_lock();
                            } else {
                                elwt.exit();
                            }
                        }
                        WindowEvent::Focused(focused) => {
                            state.window_focused = *focused;
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
