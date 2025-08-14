use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};
use crate::terrain::{ChunkPos, CHUNK_SIZE, WORLD_HEIGHT};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ChunkDebugVertex {
    pub position: [f32; 3],
}

impl ChunkDebugVertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ChunkDebugVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct ChunkDebugRenderer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    current_chunks: Vec<ChunkPos>,
}

impl ChunkDebugRenderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat, camera_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Chunk Debug Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("chunk_debug.wgsl").into()),
        });
        
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Chunk Debug Pipeline Layout"),
            bind_group_layouts: &[camera_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Chunk Debug Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[ChunkDebugVertex::desc()],
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
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: -50,
                    slope_scale: -0.5,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
        
        // Create empty buffers initially
        let empty_vertices: Vec<ChunkDebugVertex> = Vec::new();
        let empty_indices: Vec<u16> = Vec::new();
        
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk Debug Vertex Buffer"),
            contents: bytemuck::cast_slice(&empty_vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk Debug Index Buffer"),
            contents: bytemuck::cast_slice(&empty_indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });
        
        Self {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices: 0,
            current_chunks: Vec::new(),
        }
    }
    
    pub fn update_chunks(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, chunk_positions: &[ChunkPos]) {
        // Only update if chunks have changed
        if self.current_chunks.len() == chunk_positions.len() && 
           self.current_chunks.iter().all(|pos| chunk_positions.contains(pos)) {
            return;
        }
        
        self.current_chunks = chunk_positions.to_vec();
        
        let (vertices, indices) = self.generate_chunk_boundary_geometry(chunk_positions);
        
        // Recreate buffers if needed
        if !vertices.is_empty() {
            self.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Chunk Debug Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            
            self.index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Chunk Debug Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });
            
            self.num_indices = indices.len() as u32;
        } else {
            self.num_indices = 0;
        }
    }
    
    fn generate_chunk_boundary_geometry(&self, chunk_positions: &[ChunkPos]) -> (Vec<ChunkDebugVertex>, Vec<u16>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        for chunk_pos in chunk_positions {
            let start_vertex = vertices.len() as u16;
            
            // Calculate world position of chunk corner
            let world_x = chunk_pos.x * CHUNK_SIZE as i32;
            let world_z = chunk_pos.z * CHUNK_SIZE as i32;
            let world_x_f = world_x as f32;
            let world_z_f = world_z as f32;
            let chunk_size_f = CHUNK_SIZE as f32;
            let world_height_f = WORLD_HEIGHT as f32;
            
            // Create vertices for chunk boundary corners
            // Bottom corners
            vertices.push(ChunkDebugVertex { position: [world_x_f, 0.0, world_z_f] });                    // 0
            vertices.push(ChunkDebugVertex { position: [world_x_f + chunk_size_f, 0.0, world_z_f] });    // 1
            vertices.push(ChunkDebugVertex { position: [world_x_f + chunk_size_f, 0.0, world_z_f + chunk_size_f] }); // 2
            vertices.push(ChunkDebugVertex { position: [world_x_f, 0.0, world_z_f + chunk_size_f] });    // 3
            
            // Top corners
            vertices.push(ChunkDebugVertex { position: [world_x_f, world_height_f, world_z_f] });                    // 4
            vertices.push(ChunkDebugVertex { position: [world_x_f + chunk_size_f, world_height_f, world_z_f] });    // 5
            vertices.push(ChunkDebugVertex { position: [world_x_f + chunk_size_f, world_height_f, world_z_f + chunk_size_f] }); // 6
            vertices.push(ChunkDebugVertex { position: [world_x_f, world_height_f, world_z_f + chunk_size_f] });    // 7
            
            // Bottom face edges
            indices.extend(&[
                start_vertex + 0, start_vertex + 1,  // Bottom front edge
                start_vertex + 1, start_vertex + 2,  // Bottom right edge
                start_vertex + 2, start_vertex + 3,  // Bottom back edge
                start_vertex + 3, start_vertex + 0,  // Bottom left edge
            ]);
            
            // Top face edges
            indices.extend(&[
                start_vertex + 4, start_vertex + 5,  // Top front edge
                start_vertex + 5, start_vertex + 6,  // Top right edge
                start_vertex + 6, start_vertex + 7,  // Top back edge
                start_vertex + 7, start_vertex + 4,  // Top left edge
            ]);
            
            // Vertical edges
            indices.extend(&[
                start_vertex + 0, start_vertex + 4,  // Front left vertical
                start_vertex + 1, start_vertex + 5,  // Front right vertical
                start_vertex + 2, start_vertex + 6,  // Back right vertical
                start_vertex + 3, start_vertex + 7,  // Back left vertical
            ]);
        }
        
        (vertices, indices)
    }
    
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, camera_bind_group: &'a wgpu::BindGroup) {
        if self.num_indices > 0 {
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }
    }
}