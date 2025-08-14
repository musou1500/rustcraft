use crate::world::World;
use cgmath::*;

/// Represents a 3D ray for raycasting
#[derive(Debug)]
pub struct Ray {
    pub origin: Point3<f32>,
    pub direction: Vector3<f32>,
}

impl Ray {
    pub fn new(origin: Point3<f32>, direction: Vector3<f32>) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    /// Get a point along the ray at distance t
    pub fn point_at(&self, t: f32) -> Point3<f32> {
        self.origin + self.direction * t
    }
}

/// Result of a raycast hit
#[derive(Debug, Clone, Copy)]
pub struct RaycastHit {
    pub block_pos: [i32; 3],
    pub distance: f32,
    pub hit_point: Point3<f32>,
    pub face_normal: Vector3<f32>,
}

/// Perform DDA (Digital Differential Analyzer) raycasting to find block intersections
pub fn raycast_blocks(ray: Ray, max_distance: f32, world: &World) -> Option<RaycastHit> {
    let max_steps = (max_distance * 2.0) as i32; // Reasonable step limit

    // Current position in the grid
    let mut current_block = [
        ray.origin.x.floor() as i32,
        ray.origin.y.floor() as i32,
        ray.origin.z.floor() as i32,
    ];

    // Direction to step in each axis (-1, 0, or 1)
    let step = [
        if ray.direction.x > 0.0 { 1 } else { -1 },
        if ray.direction.y > 0.0 { 1 } else { -1 },
        if ray.direction.z > 0.0 { 1 } else { -1 },
    ];

    // Calculate delta distances (how far we travel along the ray for each axis step)
    let delta_dist = [
        if ray.direction.x.abs() < f32::EPSILON {
            f32::INFINITY
        } else {
            (1.0 / ray.direction.x).abs()
        },
        if ray.direction.y.abs() < f32::EPSILON {
            f32::INFINITY
        } else {
            (1.0 / ray.direction.y).abs()
        },
        if ray.direction.z.abs() < f32::EPSILON {
            f32::INFINITY
        } else {
            (1.0 / ray.direction.z).abs()
        },
    ];

    // Calculate initial side distances
    let mut side_dist = [0.0f32; 3];

    for i in 0..3 {
        if step[i] > 0 {
            side_dist[i] = (current_block[i] as f32 + 1.0
                - match i {
                    0 => ray.origin.x,
                    1 => ray.origin.y,
                    _ => ray.origin.z,
                })
                * delta_dist[i];
        } else {
            side_dist[i] = (match i {
                0 => ray.origin.x,
                1 => ray.origin.y,
                _ => ray.origin.z,
            } - current_block[i] as f32)
                * delta_dist[i];
        }
    }

    let mut last_side = 0; // Which axis was crossed last

    // DDA algorithm
    for _ in 0..max_steps {
        // Check if current block is solid (not air)
        if world.is_block_solid(current_block[0], current_block[1], current_block[2]) {
            // Calculate hit distance
            let distance = match last_side {
                0 => {
                    (current_block[0] as f32 - ray.origin.x + if step[0] > 0 { 0.0 } else { 1.0 })
                        / ray.direction.x
                }
                1 => {
                    (current_block[1] as f32 - ray.origin.y + if step[1] > 0 { 0.0 } else { 1.0 })
                        / ray.direction.y
                }
                _ => {
                    (current_block[2] as f32 - ray.origin.z + if step[2] > 0 { 0.0 } else { 1.0 })
                        / ray.direction.z
                }
            };

            if distance > max_distance {
                break;
            }

            let hit_point = ray.point_at(distance);

            // Calculate face normal based on which side was hit
            let face_normal = match last_side {
                0 => Vector3::new(-step[0] as f32, 0.0, 0.0),
                1 => Vector3::new(0.0, -step[1] as f32, 0.0),
                _ => Vector3::new(0.0, 0.0, -step[2] as f32),
            };

            return Some(RaycastHit {
                block_pos: current_block,
                distance,
                hit_point,
                face_normal,
            });
        }

        // Move to next block boundary
        if side_dist[0] < side_dist[1] && side_dist[0] < side_dist[2] {
            side_dist[0] += delta_dist[0];
            current_block[0] += step[0];
            last_side = 0;
        } else if side_dist[1] < side_dist[2] {
            side_dist[1] += delta_dist[1];
            current_block[1] += step[1];
            last_side = 1;
        } else {
            side_dist[2] += delta_dist[2];
            current_block[2] += step[2];
            last_side = 2;
        }

        // Check distance limit
        let current_distance = (current_block[0] as f32 - ray.origin.x).abs()
            + (current_block[1] as f32 - ray.origin.y).abs()
            + (current_block[2] as f32 - ray.origin.z).abs();
        if current_distance > max_distance * 2.0 {
            break;
        }
    }

    None
}

/// Create a ray from camera position and direction
pub fn create_camera_ray(camera_pos: Point3<f32>, camera_yaw: f32, camera_pitch: f32) -> Ray {
    let (sin_pitch, cos_pitch) = camera_pitch.sin_cos();
    let (sin_yaw, cos_yaw) = camera_yaw.sin_cos();

    let direction = Vector3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw);

    Ray::new(camera_pos, direction)
}
