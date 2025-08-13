use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct WireframeVertex {
    pub position: [f32; 3],
}


impl WireframeVertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<WireframeVertex>() as wgpu::BufferAddress,
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

pub struct WireframeRenderer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
}

impl WireframeRenderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat, camera_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        // Create wireframe cube vertices (just corners)
        let vertices = create_wireframe_cube_vertices(0.0, 0.0, 0.0);
        let indices = create_wireframe_cube_indices();
        
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Wireframe Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Wireframe Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Wireframe Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("wireframe.wgsl").into()),
        });
        
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Wireframe Pipeline Layout"),
            bind_group_layouts: &[camera_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Wireframe Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[WireframeVertex::desc()],
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
                cull_mode: None, // Don't cull wireframe
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false, // Don't write to depth for wireframe
                depth_compare: wgpu::CompareFunction::LessEqual, // Render wireframe with depth testing
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: -100, // Pull wireframe forward to avoid z-fighting
                    slope_scale: -1.0,
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
        
        Self {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
        }
    }
    
    pub fn update_position(&self, device: &wgpu::Device, queue: &wgpu::Queue, x: f32, y: f32, z: f32) {
        let vertices = create_wireframe_cube_vertices(x, y, z);
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
    }
    
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, camera_bind_group: &'a wgpu::BindGroup) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }
}

fn create_wireframe_cube_vertices(x: f32, y: f32, z: f32) -> Vec<WireframeVertex> {
    let offset = 0.05; // Larger offset for better visibility
    vec![
        // Bottom face corners  
        WireframeVertex { position: [x - offset, y - offset, z - offset] },           // 0
        WireframeVertex { position: [x + 1.0 + offset, y - offset, z - offset] },     // 1
        WireframeVertex { position: [x + 1.0 + offset, y - offset, z + 1.0 + offset] }, // 2
        WireframeVertex { position: [x - offset, y - offset, z + 1.0 + offset] },     // 3
        
        // Top face corners
        WireframeVertex { position: [x - offset, y + 1.0 + offset, z - offset] },           // 4
        WireframeVertex { position: [x + 1.0 + offset, y + 1.0 + offset, z - offset] },     // 5
        WireframeVertex { position: [x + 1.0 + offset, y + 1.0 + offset, z + 1.0 + offset] }, // 6
        WireframeVertex { position: [x - offset, y + 1.0 + offset, z + 1.0 + offset] },     // 7
    ]
}

fn create_wireframe_cube_indices() -> Vec<u16> {
    vec![
        // Bottom face edges
        0, 1,  1, 2,  2, 3,  3, 0,
        // Top face edges
        4, 5,  5, 6,  6, 7,  7, 4,
        // Vertical edges
        0, 4,  1, 5,  2, 6,  3, 7,
    ]
}