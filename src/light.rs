use cgmath::*;
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct LightUniform {
    pub direction: [f32; 3],
    pub _padding: f32,
    pub color: [f32; 3],
    pub intensity: f32,
    pub light_space_matrix: [[f32; 4]; 4],
}

pub struct DirectionalLight {
    pub direction: Vector3<f32>,
    pub color: Vector3<f32>,
    pub intensity: f32,
    uniform: LightUniform,
    buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl DirectionalLight {
    pub fn new(device: &wgpu::Device) -> Self {
        let direction = Vector3::new(-0.5, -1.0, -0.5).normalize(); // More angled sunlight
        let color = Vector3::new(1.0, 1.0, 1.0); // Pure white light
        let intensity = 1.0;

        let mut uniform = LightUniform {
            direction: direction.into(),
            _padding: 0.0,
            color: color.into(),
            intensity,
            light_space_matrix: Matrix4::identity().into(),
        };

        // Create light space matrix for shadow mapping
        let light_projection = ortho(-20.0, 20.0, -20.0, 20.0, 1.0, 100.0);
        let light_view = Matrix4::look_at_rh(
            Point3::new(0.0, 0.0, 0.0) + (-direction * 50.0),
            Point3::origin(),
            Vector3::unit_y(),
        );
        uniform.light_space_matrix = (light_projection * light_view).into();

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("light_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("light_bind_group"),
        });

        Self {
            direction,
            color,
            intensity,
            uniform,
            buffer,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn update_buffer(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }

    pub fn get_light_space_matrix(&self) -> Matrix4<f32> {
        Matrix4::from(self.uniform.light_space_matrix)
    }
}