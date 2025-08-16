use bytemuck::{Pod, Zeroable};
use cgmath::*;
use std::time::Duration;
use wgpu::util::DeviceExt;
use winit::event::*;
use winit::keyboard::{KeyCode, PhysicalKey};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub struct Camera {
    pub position: Point3<f32>,
    yaw: Rad<f32>,
    pitch: Rad<f32>,
    pub aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl Camera {
    pub fn new(position: Point3<f32>, yaw: Deg<f32>, pitch: Deg<f32>, aspect: f32) -> Self {
        Self {
            position,
            yaw: yaw.into(),
            pitch: pitch.into(),
            aspect,
            fovy: Rad(45.0_f32.to_radians()),
            znear: 0.1,
            zfar: 100.0,
        }
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();

        let target =
            self.position + Vector3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw);

        let view = Matrix4::look_at_rh(self.position, target, Vector3::unit_y());
        let proj = perspective(self.fovy, self.aspect, self.znear, self.zfar);

        OPENGL_TO_WGPU_MATRIX * proj * view
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_proj: Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.calc_matrix().into();
    }
}

pub struct CameraController {
    speed: f32,
    run_speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    is_jump_pressed: bool,
    is_running: bool,
    mouse_dx: f32,
    mouse_dy: f32,
    sensitivity: f32,
    left_mouse_pressed: bool,
    right_mouse_pressed: bool,
    // Physics properties
    velocity_y: f32,
    is_grounded: bool,
    jump_speed: f32,
    gravity: f32,
    player_height: f32,
    eye_height: f32, // Height of eyes above feet
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            speed,
            run_speed: speed * 2.0, // Running is 2x normal speed
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_jump_pressed: false,
            is_running: false,
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            sensitivity,
            left_mouse_pressed: false,
            right_mouse_pressed: false,
            velocity_y: 0.0,
            is_grounded: false,
            jump_speed: 8.0,
            gravity: 25.0,
            player_height: 1.8,
            eye_height: 1.6, // Eyes are 1.6 blocks above feet
        }
    }

    pub fn process_window_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(keycode),
                        ..
                    },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    KeyCode::KeyW | KeyCode::ArrowUp => {
                        self.is_forward_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyA | KeyCode::ArrowLeft => {
                        self.is_left_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyS | KeyCode::ArrowDown => {
                        self.is_backward_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyD | KeyCode::ArrowRight => {
                        self.is_right_pressed = is_pressed;
                        true
                    }
                    KeyCode::Space => {
                        // Only register jump on key press, not hold
                        if is_pressed && !self.is_jump_pressed {
                            self.is_jump_pressed = true;
                        } else if !is_pressed {
                            self.is_jump_pressed = false;
                        }
                        true
                    }
                    KeyCode::ControlLeft | KeyCode::ControlRight => {
                        self.is_running = is_pressed;
                        true
                    }
                    _ => false,
                }
            }
            WindowEvent::MouseInput { state, button, .. } => match button {
                MouseButton::Left => {
                    self.left_mouse_pressed = *state == ElementState::Pressed;
                    true
                }
                MouseButton::Right => {
                    self.right_mouse_pressed = *state == ElementState::Pressed;
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    pub fn process_device_events(&mut self, event: &DeviceEvent) -> bool {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                self.mouse_dx += delta.0 as f32;
                self.mouse_dy += delta.1 as f32;
                true
            }
            _ => false,
        }
    }

    pub fn update_camera(
        &mut self,
        camera: &mut Camera,
        dt: Duration,
        world: &crate::world::World,
    ) {
        let dt = dt.as_secs_f32();

        // Handle mouse look
        camera.yaw += Rad(self.mouse_dx * self.sensitivity * dt);
        camera.pitch -= Rad(self.mouse_dy * self.sensitivity * dt);

        camera.pitch = Rad(camera.pitch.0.clamp(-1.54, 1.54));

        self.mouse_dx = 0.0;
        self.mouse_dy = 0.0;

        // Calculate movement vectors (horizontal only)
        let (yaw_sin, yaw_cos) = camera.yaw.0.sin_cos();
        let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();

        // Calculate desired horizontal movement
        let mut horizontal_movement = Vector3::new(0.0, 0.0, 0.0);

        if self.is_forward_pressed {
            horizontal_movement += forward;
        }
        if self.is_backward_pressed {
            horizontal_movement -= forward;
        }
        if self.is_right_pressed {
            horizontal_movement += right;
        }
        if self.is_left_pressed {
            horizontal_movement -= right;
        }

        // Normalize diagonal movement and apply appropriate speed
        if horizontal_movement.magnitude() > 0.0 {
            let current_speed = if self.is_running {
                self.run_speed
            } else {
                self.speed
            };
            horizontal_movement = horizontal_movement.normalize() * current_speed * dt;
        }

        // Apply horizontal movement with collision detection
        let new_x = camera.position.x + horizontal_movement.x;
        let new_z = camera.position.z + horizontal_movement.z;

        // Check X movement collision
        if !self.check_collision(
            Point3::new(new_x, camera.position.y, camera.position.z),
            world,
        ) {
            camera.position.x = new_x;
        }

        // Check Z movement collision
        if !self.check_collision(
            Point3::new(camera.position.x, camera.position.y, new_z),
            world,
        ) {
            camera.position.z = new_z;
        }

        // Handle jumping
        if self.is_jump_pressed && self.is_grounded {
            self.velocity_y = self.jump_speed;
            self.is_grounded = false;
            self.is_jump_pressed = false; // Consume the jump input
        }

        // Apply gravity
        self.velocity_y -= self.gravity * dt;

        // Apply vertical movement with collision detection
        let new_y = camera.position.y + self.velocity_y * dt;

        // Check if player would be underground or hit ceiling
        let collision_pos = Point3::new(camera.position.x, new_y, camera.position.z);

        if self.check_collision(collision_pos, world) {
            if self.velocity_y < 0.0 {
                // Hit ground
                self.velocity_y = 0.0;
                self.is_grounded = true;
                // Snap to ground level
                camera.position.y =
                    self.find_ground_level(camera.position.x, camera.position.z, world);
            } else {
                // Hit ceiling
                self.velocity_y = 0.0;
            }
        } else {
            camera.position.y = new_y;
            self.is_grounded = false;
        }
    }

    fn check_collision(&self, eye_position: Point3<f32>, world: &crate::world::World) -> bool {
        // Convert eye position to feet position
        let feet_position = Point3::new(
            eye_position.x,
            eye_position.y - self.eye_height,
            eye_position.z,
        );

        // Player bounding box: feet at feet_position.y, head at feet_position.y + player_height
        let feet_y = feet_position.y.floor() as i32;
        let head_y = (feet_position.y + self.player_height).floor() as i32;

        let player_x = feet_position.x.floor() as i32;
        let player_z = feet_position.z.floor() as i32;

        // Check blocks at player position for both feet and head levels
        for y in feet_y..=head_y {
            if world.is_block_solid(player_x, y, player_z) {
                return true;
            }
        }
        false
    }

    fn find_ground_level(&self, x: f32, z: f32, world: &crate::world::World) -> f32 {
        let block_x = x.floor() as i32;
        let block_z = z.floor() as i32;

        // Search downward for the highest solid block
        for y in (0..crate::chunk::WORLD_HEIGHT as i32).rev() {
            if world.is_block_solid(block_x, y, block_z) {
                // Return eye level position (feet position + eye height)
                return (y + 1) as f32 + self.eye_height;
            }
        }
        self.eye_height // Default to eye height above ground level if no solid block found
    }

    pub fn was_left_mouse_clicked(&mut self) -> bool {
        if self.left_mouse_pressed {
            self.left_mouse_pressed = false; // Reset the flag
            true
        } else {
            false
        }
    }

    pub fn was_right_mouse_clicked(&mut self) -> bool {
        if self.right_mouse_pressed {
            self.right_mouse_pressed = false; // Reset the flag
            true
        } else {
            false
        }
    }
}

pub struct CameraSystem {
    camera: Camera,
    controller: CameraController,
    uniform: CameraUniform,
    buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl CameraSystem {
    pub fn new(camera: Camera, device: &wgpu::Device) -> Self {
        let mut uniform = CameraUniform::new();
        uniform.update_view_proj(&camera);

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let controller = CameraController::new(4.0, 0.5);

        Self {
            camera,
            controller,
            uniform,
            buffer,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn process_window_events(&mut self, event: &WindowEvent) -> bool {
        self.controller.process_window_events(event)
    }

    pub fn process_device_events(&mut self, event: &DeviceEvent) -> bool {
        self.controller.process_device_events(event)
    }

    pub fn update(&mut self, dt: Duration, world: &crate::world::World) {
        self.controller.update_camera(&mut self.camera, dt, world);
        self.uniform.update_view_proj(&self.camera);
    }

    pub fn update_buffer(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }

    pub fn get_position(&self) -> Point3<f32> {
        self.camera.position
    }

    pub fn get_yaw(&self) -> f32 {
        self.camera.yaw.0
    }

    pub fn get_pitch(&self) -> f32 {
        self.camera.pitch.0
    }

    pub fn was_left_mouse_clicked(&mut self) -> bool {
        self.controller.was_left_mouse_clicked()
    }

    pub fn was_right_mouse_clicked(&mut self) -> bool {
        self.controller.was_right_mouse_clicked()
    }
}
