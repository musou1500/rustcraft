use crate::blocks::BlockType;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SlotVertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct SlotUniform {
    selected_slot: u32,
    _padding: [u32; 3], // 16-byte alignment
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct SlotInventoryData {
    // Each slot stores texture ID (0-15) and whether it has a block (0 or 1)
    // Using vec4 for proper alignment in WGSL
    slot_data_1: [u32; 4], // slots 0-3
    slot_data_2: [u32; 4], // slots 4-7
    slot_data_3: [u32; 4], // slots 8-9 (10 and 11 unused)
}

impl SlotVertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SlotVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub struct SlotUI {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    inventory_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    texture_bind_group: wgpu::BindGroup,
    selected_slot: usize, // 0-9, where 0 is leftmost
    num_indices: u32,
    inventory: [Option<BlockType>; 10], // 10 slots for blocks
}

impl SlotUI {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        texture_atlas: &crate::texture_atlas::TextureAtlas,
        window_width: u32,
        window_height: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Slot UI Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("slot_ui.wgsl").into()),
        });

        // Create uniform buffer
        let uniform = SlotUniform {
            selected_slot: 0,
            _padding: [0; 3],
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Slot UI Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create inventory buffer
        let inventory_data = SlotInventoryData {
            slot_data_1: [0; 4], // All slots start empty
            slot_data_2: [0; 4],
            slot_data_3: [0; 4],
        };

        let inventory_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Slot UI Inventory Buffer"),
            contents: bytemuck::cast_slice(&[inventory_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("slot_ui_bind_group_layout"),
        });

        // Create texture bind group layout
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

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: inventory_buffer.as_entire_binding(),
                },
            ],
            label: Some("slot_ui_bind_group"),
        });

        // Create texture bind group
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_atlas.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture_atlas.sampler),
                },
            ],
            label: Some("slot_ui_texture_bind_group"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Slot UI Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout, &texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Slot UI Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[SlotVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
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
                depth_write_enabled: false, // Don't write to depth for UI
                depth_compare: wgpu::CompareFunction::Always, // Always render UI on top
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

        let (vertices, indices) = Self::create_slot_geometry(window_width, window_height);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Slot UI Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Slot UI Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            inventory_buffer,
            bind_group,
            texture_bind_group,
            selected_slot: 0, // Start with leftmost slot selected
            num_indices: indices.len() as u32,
            inventory: [None; 10], // Initialize all slots as empty
        }
    }

    fn create_slot_geometry(window_width: u32, window_height: u32) -> (Vec<SlotVertex>, Vec<u16>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Fixed pixel dimensions
        const SLOT_SIZE_PX: f32 = 70.0; // 100px slots
        const GAP_PX: f32 = 8.0; // 8px gap between slots
        const BOTTOM_MARGIN_PX: f32 = 20.0; // 20px from bottom of screen

        // Convert pixels to normalized coordinates (-1 to 1)
        let slot_width_norm = (SLOT_SIZE_PX * 2.0) / window_width as f32;
        let slot_height_norm = (SLOT_SIZE_PX * 2.0) / window_height as f32;
        let gap_norm = (GAP_PX * 2.0) / window_width as f32;

        // Calculate total width and center horizontally
        let total_width_norm = slot_width_norm * 10.0 + gap_norm * 9.0;
        let start_x = -total_width_norm / 2.0;

        // Position at bottom with margin
        let bottom_margin_norm = (BOTTOM_MARGIN_PX * 2.0) / window_height as f32;
        let y_bottom = -1.0 + bottom_margin_norm;

        for i in 0..10 {
            let x_left = start_x + (slot_width_norm + gap_norm) * i as f32;
            let x_right = x_left + slot_width_norm;
            let y_top = y_bottom + slot_height_norm;

            let vertex_start = vertices.len() as u16;

            // Create quad for slot
            vertices.push(SlotVertex {
                position: [x_left, y_bottom],
                tex_coords: [0.0, 1.0],
            });
            vertices.push(SlotVertex {
                position: [x_right, y_bottom],
                tex_coords: [1.0, 1.0],
            });
            vertices.push(SlotVertex {
                position: [x_right, y_top],
                tex_coords: [1.0, 0.0],
            });
            vertices.push(SlotVertex {
                position: [x_left, y_top],
                tex_coords: [0.0, 0.0],
            });

            // Two triangles for the quad
            indices.extend(&[
                vertex_start,
                vertex_start + 1,
                vertex_start + 2,
                vertex_start,
                vertex_start + 2,
                vertex_start + 3,
            ]);
        }

        (vertices, indices)
    }

    pub fn get_selected_slot(&self) -> usize {
        self.selected_slot
    }

    pub fn set_selected_slot(&mut self, slot: usize, queue: &wgpu::Queue) {
        if slot < 10 {
            self.selected_slot = slot;

            // Update uniform buffer
            let uniform = SlotUniform {
                selected_slot: slot as u32,
                _padding: [0; 3],
            };
            queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));
        }
    }

    pub fn update_geometry(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        window_width: u32,
        window_height: u32,
    ) {
        let (vertices, _) = Self::create_slot_geometry(window_width, window_height);
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
    }

    pub fn put_block_in_selected_slot(&mut self, block_type: BlockType, queue: &wgpu::Queue) {
        self.inventory[self.selected_slot] = Some(block_type);
        println!("Put {:?} in slot {}", block_type, self.selected_slot);

        // Update the inventory buffer
        self.update_inventory_buffer(queue);
    }

    fn block_type_to_texture_id(block_type: BlockType) -> u32 {
        use crate::blocks::TextureId;
        match block_type {
            BlockType::Air => 0,
            BlockType::Stone => TextureId::Stone as u32,
            BlockType::Dirt => TextureId::Dirt as u32,
            BlockType::Grass => TextureId::GrassTop as u32, // Use grass top texture for inventory
            BlockType::Sand => TextureId::Sand as u32,
            BlockType::Water => TextureId::Water as u32,
            BlockType::Wood => TextureId::WoodTop as u32,
            BlockType::Leaves => TextureId::Leaves as u32,
            BlockType::Coal => TextureId::Coal as u32,
            BlockType::Iron => TextureId::Iron as u32,
            BlockType::Gold => TextureId::Gold as u32,
            BlockType::Snow => TextureId::Snow as u32,
        }
    }

    fn update_inventory_buffer(&self, queue: &wgpu::Queue) {
        let mut slot_data_1 = [0u32; 4];
        let mut slot_data_2 = [0u32; 4];
        let mut slot_data_3 = [0u32; 4];

        for (i, block_opt) in self.inventory.iter().enumerate() {
            let texture_id = if let Some(block_type) = block_opt {
                Self::block_type_to_texture_id(*block_type)
            } else {
                0 // Empty slot
            };

            if i < 4 {
                slot_data_1[i] = texture_id;
            } else if i < 8 {
                slot_data_2[i - 4] = texture_id;
            } else {
                slot_data_3[i - 8] = texture_id;
            }
        }

        let inventory_data = SlotInventoryData {
            slot_data_1,
            slot_data_2,
            slot_data_3,
        };
        queue.write_buffer(
            &self.inventory_buffer,
            0,
            bytemuck::cast_slice(&[inventory_data]),
        );
    }

    pub fn get_block_in_slot(&self, slot: usize) -> Option<BlockType> {
        if slot < 10 {
            self.inventory[slot]
        } else {
            None
        }
    }

    pub fn get_block_in_selected_slot(&self) -> Option<BlockType> {
        self.inventory[self.selected_slot]
    }

    pub fn clear_selected_slot(&mut self) {
        self.inventory[self.selected_slot] = None;
        println!("Cleared slot {}", self.selected_slot);
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_bind_group(1, &self.texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }
}
