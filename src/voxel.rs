use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
    pub texture_id: u32,
}

impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Texture coordinates
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // Normal
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() + std::mem::size_of::<[f32; 2]>())
                        as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Texture ID
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>()
                        + std::mem::size_of::<[f32; 2]>()
                        + std::mem::size_of::<[f32; 3]>())
                        as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

// Create cube vertices with proper UV mapping for Minecraft-like textures
pub fn create_cube_vertices_minecraft(
    x: f32,
    y: f32,
    z: f32,
    texture_ids: &FaceTextures,
) -> Vec<Vertex> {
    vec![
        // Front face (normal: +Z)
        Vertex {
            position: [x, y, z + 1.0],
            tex_coords: [0.0, 1.0],
            normal: [0.0, 0.0, 1.0],
            texture_id: texture_ids.front,
        },
        Vertex {
            position: [x + 1.0, y, z + 1.0],
            tex_coords: [1.0, 1.0],
            normal: [0.0, 0.0, 1.0],
            texture_id: texture_ids.front,
        },
        Vertex {
            position: [x + 1.0, y + 1.0, z + 1.0],
            tex_coords: [1.0, 0.0],
            normal: [0.0, 0.0, 1.0],
            texture_id: texture_ids.front,
        },
        Vertex {
            position: [x, y + 1.0, z + 1.0],
            tex_coords: [0.0, 0.0],
            normal: [0.0, 0.0, 1.0],
            texture_id: texture_ids.front,
        },
        // Back face (normal: -Z)
        Vertex {
            position: [x + 1.0, y, z],
            tex_coords: [0.0, 1.0],
            normal: [0.0, 0.0, -1.0],
            texture_id: texture_ids.back,
        },
        Vertex {
            position: [x, y, z],
            tex_coords: [1.0, 1.0],
            normal: [0.0, 0.0, -1.0],
            texture_id: texture_ids.back,
        },
        Vertex {
            position: [x, y + 1.0, z],
            tex_coords: [1.0, 0.0],
            normal: [0.0, 0.0, -1.0],
            texture_id: texture_ids.back,
        },
        Vertex {
            position: [x + 1.0, y + 1.0, z],
            tex_coords: [0.0, 0.0],
            normal: [0.0, 0.0, -1.0],
            texture_id: texture_ids.back,
        },
        // Left face (normal: -X)
        Vertex {
            position: [x, y, z],
            tex_coords: [0.0, 1.0],
            normal: [-1.0, 0.0, 0.0],
            texture_id: texture_ids.left,
        },
        Vertex {
            position: [x, y, z + 1.0],
            tex_coords: [1.0, 1.0],
            normal: [-1.0, 0.0, 0.0],
            texture_id: texture_ids.left,
        },
        Vertex {
            position: [x, y + 1.0, z + 1.0],
            tex_coords: [1.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
            texture_id: texture_ids.left,
        },
        Vertex {
            position: [x, y + 1.0, z],
            tex_coords: [0.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
            texture_id: texture_ids.left,
        },
        // Right face (normal: +X)
        Vertex {
            position: [x + 1.0, y, z + 1.0],
            tex_coords: [0.0, 1.0],
            normal: [1.0, 0.0, 0.0],
            texture_id: texture_ids.right,
        },
        Vertex {
            position: [x + 1.0, y, z],
            tex_coords: [1.0, 1.0],
            normal: [1.0, 0.0, 0.0],
            texture_id: texture_ids.right,
        },
        Vertex {
            position: [x + 1.0, y + 1.0, z],
            tex_coords: [1.0, 0.0],
            normal: [1.0, 0.0, 0.0],
            texture_id: texture_ids.right,
        },
        Vertex {
            position: [x + 1.0, y + 1.0, z + 1.0],
            tex_coords: [0.0, 0.0],
            normal: [1.0, 0.0, 0.0],
            texture_id: texture_ids.right,
        },
        // Top face (normal: +Y)
        Vertex {
            position: [x, y + 1.0, z + 1.0],
            tex_coords: [0.0, 0.0],
            normal: [0.0, 1.0, 0.0],
            texture_id: texture_ids.top,
        },
        Vertex {
            position: [x + 1.0, y + 1.0, z + 1.0],
            tex_coords: [1.0, 0.0],
            normal: [0.0, 1.0, 0.0],
            texture_id: texture_ids.top,
        },
        Vertex {
            position: [x + 1.0, y + 1.0, z],
            tex_coords: [1.0, 1.0],
            normal: [0.0, 1.0, 0.0],
            texture_id: texture_ids.top,
        },
        Vertex {
            position: [x, y + 1.0, z],
            tex_coords: [0.0, 1.0],
            normal: [0.0, 1.0, 0.0],
            texture_id: texture_ids.top,
        },
        // Bottom face (normal: -Y)
        Vertex {
            position: [x, y, z],
            tex_coords: [0.0, 0.0],
            normal: [0.0, -1.0, 0.0],
            texture_id: texture_ids.bottom,
        },
        Vertex {
            position: [x + 1.0, y, z],
            tex_coords: [1.0, 0.0],
            normal: [0.0, -1.0, 0.0],
            texture_id: texture_ids.bottom,
        },
        Vertex {
            position: [x + 1.0, y, z + 1.0],
            tex_coords: [1.0, 1.0],
            normal: [0.0, -1.0, 0.0],
            texture_id: texture_ids.bottom,
        },
        Vertex {
            position: [x, y, z + 1.0],
            tex_coords: [0.0, 1.0],
            normal: [0.0, -1.0, 0.0],
            texture_id: texture_ids.bottom,
        },
    ]
}

// Structure to hold texture IDs for each face of a cube
#[derive(Debug, Clone, Copy)]
pub struct FaceTextures {
    pub front: u32,
    pub back: u32,
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
}

impl FaceTextures {
    pub fn all_same(texture_id: u32) -> Self {
        Self {
            front: texture_id,
            back: texture_id,
            left: texture_id,
            right: texture_id,
            top: texture_id,
            bottom: texture_id,
        }
    }

    pub fn new(front: u32, back: u32, left: u32, right: u32, top: u32, bottom: u32) -> Self {
        Self {
            front,
            back,
            left,
            right,
            top,
            bottom,
        }
    }
}

// Generate only specific faces for optimization with proper UV mapping
pub fn create_cube_vertices_selective(
    x: f32,
    y: f32,
    z: f32,
    texture_ids: &FaceTextures,
    faces_to_render: &[usize],
) -> Vec<Vertex> {
    let mut vertices = Vec::new();

    // Define face vertex data: positions, texture coordinates, normals, and texture IDs
    let face_definitions = [
        // Face 0: Front face (normal: +Z)
        (
            [
                ([x, y, z + 1.0], [0.0, 1.0]),
                ([x + 1.0, y, z + 1.0], [1.0, 1.0]),
                ([x + 1.0, y + 1.0, z + 1.0], [1.0, 0.0]),
                ([x, y + 1.0, z + 1.0], [0.0, 0.0]),
            ],
            [0.0, 0.0, 1.0],
            texture_ids.front,
        ),
        // Face 1: Back face (normal: -Z)
        (
            [
                ([x + 1.0, y, z], [0.0, 1.0]),
                ([x, y, z], [1.0, 1.0]),
                ([x, y + 1.0, z], [1.0, 0.0]),
                ([x + 1.0, y + 1.0, z], [0.0, 0.0]),
            ],
            [0.0, 0.0, -1.0],
            texture_ids.back,
        ),
        // Face 2: Left face (normal: -X)
        (
            [
                ([x, y, z], [0.0, 1.0]),
                ([x, y, z + 1.0], [1.0, 1.0]),
                ([x, y + 1.0, z + 1.0], [1.0, 0.0]),
                ([x, y + 1.0, z], [0.0, 0.0]),
            ],
            [-1.0, 0.0, 0.0],
            texture_ids.left,
        ),
        // Face 3: Right face (normal: +X)
        (
            [
                ([x + 1.0, y, z + 1.0], [0.0, 1.0]),
                ([x + 1.0, y, z], [1.0, 1.0]),
                ([x + 1.0, y + 1.0, z], [1.0, 0.0]),
                ([x + 1.0, y + 1.0, z + 1.0], [0.0, 0.0]),
            ],
            [1.0, 0.0, 0.0],
            texture_ids.right,
        ),
        // Face 4: Top face (normal: +Y)
        (
            [
                ([x, y + 1.0, z + 1.0], [0.0, 0.0]),
                ([x + 1.0, y + 1.0, z + 1.0], [1.0, 0.0]),
                ([x + 1.0, y + 1.0, z], [1.0, 1.0]),
                ([x, y + 1.0, z], [0.0, 1.0]),
            ],
            [0.0, 1.0, 0.0],
            texture_ids.top,
        ),
        // Face 5: Bottom face (normal: -Y)
        (
            [
                ([x, y, z], [0.0, 0.0]),
                ([x + 1.0, y, z], [1.0, 0.0]),
                ([x + 1.0, y, z + 1.0], [1.0, 1.0]),
                ([x, y, z + 1.0], [0.0, 1.0]),
            ],
            [0.0, -1.0, 0.0],
            texture_ids.bottom,
        ),
    ];

    for &face_index in faces_to_render {
        if face_index < face_definitions.len() {
            let (vertex_data, normal, texture_id) = &face_definitions[face_index];

            for &(position, tex_coords) in vertex_data {
                vertices.push(Vertex {
                    position,
                    tex_coords,
                    normal: *normal,
                    texture_id: *texture_id,
                });
            }
        }
    }

    vertices
}

// Generate corresponding indices for selective faces
pub fn create_cube_indices_selective(faces_to_render: &[usize], vertex_offset: u32) -> Vec<u32> {
    let mut indices = Vec::new();

    for (local_face_index, &_) in faces_to_render.iter().enumerate() {
        let face_vertex_offset = vertex_offset + (local_face_index * 4) as u32;
        indices.extend(vec![
            face_vertex_offset,
            face_vertex_offset + 1,
            face_vertex_offset + 2,
            face_vertex_offset + 2,
            face_vertex_offset + 3,
            face_vertex_offset,
        ]);
    }

    indices
}

// Keep the old function for backward compatibility - now creates a simple textured cube
pub fn create_cube_vertices(x: f32, y: f32, z: f32, texture_id: u32) -> Vec<Vertex> {
    let textures = FaceTextures::all_same(texture_id);
    create_cube_vertices_minecraft(x, y, z, &textures)
}

pub fn create_cube_indices() -> Vec<u16> {
    vec![
        // Front face
        0, 1, 2, 2, 3, 0, // Back face
        4, 5, 6, 6, 7, 4, // Left face
        8, 9, 10, 10, 11, 8, // Right face
        12, 13, 14, 14, 15, 12, // Top face
        16, 17, 18, 18, 19, 16, // Bottom face
        20, 21, 22, 22, 23, 20,
    ]
}
