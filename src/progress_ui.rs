use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct UIVertex {
    pub position: [f32; 2],
    pub color: [f32; 3],
}

impl UIVertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<UIVertex>() as wgpu::BufferAddress,
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
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct ProgressUI {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
}

impl ProgressUI {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Progress UI Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("progress_ui.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Progress UI Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Progress UI Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[UIVertex::desc()],
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
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // Create initial empty buffers
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Progress UI Vertex Buffer"),
            size: 1024, // Reserve space for vertices
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Progress UI Index Buffer"),
            size: 512, // Reserve space for indices
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            render_pipeline,
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn update_progress(&self, device: &wgpu::Device, queue: &wgpu::Queue, progress: f32, is_generating: bool) {
        if !is_generating {
            return;
        }

        // Create progress bar geometry
        let bar_width = 0.6; // 60% of screen width
        let bar_height = 0.05; // 5% of screen height
        let bar_x = -bar_width / 2.0; // Center horizontally
        let bar_y = -0.8; // Near bottom of screen

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Background bar (dark gray)
        vertices.extend_from_slice(&[
            UIVertex { position: [bar_x, bar_y], color: [0.2, 0.2, 0.2] },
            UIVertex { position: [bar_x + bar_width, bar_y], color: [0.2, 0.2, 0.2] },
            UIVertex { position: [bar_x + bar_width, bar_y + bar_height], color: [0.2, 0.2, 0.2] },
            UIVertex { position: [bar_x, bar_y + bar_height], color: [0.2, 0.2, 0.2] },
        ]);

        // Background bar indices
        indices.extend_from_slice(&[0, 1, 2, 2, 3, 0]);

        // Progress fill (green to yellow gradient based on progress)
        let progress_width = bar_width * progress;
        if progress_width > 0.0 {
            let color = if progress < 0.5 {
                // Red to yellow
                [progress * 2.0, 1.0, 0.0]
            } else {
                // Yellow to green
                [2.0 - progress * 2.0, 1.0, 0.0]
            };

            let fill_vertices = [
                UIVertex { position: [bar_x, bar_y], color },
                UIVertex { position: [bar_x + progress_width, bar_y], color },
                UIVertex { position: [bar_x + progress_width, bar_y + bar_height], color },
                UIVertex { position: [bar_x, bar_y + bar_height], color },
            ];

            let vertex_offset = vertices.len() as u16;
            vertices.extend_from_slice(&fill_vertices);
            
            // Progress fill indices
            indices.extend_from_slice(&[
                vertex_offset, vertex_offset + 1, vertex_offset + 2,
                vertex_offset + 2, vertex_offset + 3, vertex_offset,
            ]);
        }

        // Add loading text indicator (simple blocks representing "LOADING...")
        let text_y = bar_y + bar_height + 0.1;
        let block_size = 0.02;
        let spacing = 0.03;
        let start_x = -0.2;

        // Simple pattern for "LOADING" using blocks
        let loading_pattern = [
            // L
            [0, 0], [0, 1], [0, 2], [0, 3], [1, 0],
            // O
            [3, 0], [3, 1], [3, 2], [3, 3], [4, 0], [4, 3], [5, 0], [5, 1], [5, 2], [5, 3],
            // A
            [7, 0], [7, 1], [7, 2], [7, 3], [8, 2], [8, 3], [9, 0], [9, 1], [9, 2], [9, 3],
            // D
            [11, 0], [11, 1], [11, 2], [11, 3], [12, 0], [12, 3], [13, 1], [13, 2],
            // I
            [15, 0], [15, 1], [15, 2], [15, 3],
            // N
            [17, 0], [17, 1], [17, 2], [17, 3], [18, 2], [19, 0], [19, 1], [19, 2], [19, 3],
            // G
            [21, 0], [21, 1], [21, 2], [21, 3], [22, 0], [22, 2], [23, 0], [23, 2], [23, 3],
        ];

        for &[x, y] in &loading_pattern {
            let block_x = start_x + x as f32 * spacing;
            let block_y_pos = text_y + y as f32 * spacing;
            
            let vertex_offset = vertices.len() as u16;
            vertices.extend_from_slice(&[
                UIVertex { position: [block_x, block_y_pos], color: [1.0, 1.0, 1.0] },
                UIVertex { position: [block_x + block_size, block_y_pos], color: [1.0, 1.0, 1.0] },
                UIVertex { position: [block_x + block_size, block_y_pos + block_size], color: [1.0, 1.0, 1.0] },
                UIVertex { position: [block_x, block_y_pos + block_size], color: [1.0, 1.0, 1.0] },
            ]);

            indices.extend_from_slice(&[
                vertex_offset, vertex_offset + 1, vertex_offset + 2,
                vertex_offset + 2, vertex_offset + 3, vertex_offset,
            ]);
        }

        // Update buffers
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, is_generating: bool, num_indices: u32) {
        if !is_generating || num_indices == 0 {
            return;
        }

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..num_indices, 0, 0..1);
    }
}