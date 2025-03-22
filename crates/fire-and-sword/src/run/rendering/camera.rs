use {
    super::wgpu_ext::{bind_group::HasBindGroup, buffer::uniform::UniformBuffer, global_context::device},
    crate::bind_group_layout,
    glam::{Mat4, Vec3},
    shader_types::glam,
    tap::prelude::*,
    wgpu::BindGroup,
};

pub const SENSITIVITY: f32 = 0.04;

// A basic camera struct with common properties
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    position: Vec3, // Camera position in world space
    yaw: f32,       // Rotation around Y-axis (left-right), in radians
    pitch: f32,     // Rotation around X-axis (up-down), in radians
    size: (f32, f32),
}

impl Camera {
    // Create a new camera with default values
    pub fn new(position: Vec3, size: (f32, f32)) -> Self {
        Camera {
            position,
            yaw: 0.,
            pitch: 0.,
            size,
        }
    }
    pub fn look(&self) -> Vec3 {
        Vec3::new(self.yaw.cos() * self.pitch.cos(), self.pitch.sin(), self.yaw.sin() * self.pitch.cos()).normalize()
    }
    // Update rotation based on mouse movement
    pub fn update_rotation(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw += delta_x * SENSITIVITY;
        self.pitch -= delta_y * SENSITIVITY;
        self.pitch = self
            .pitch
            .clamp(-std::f32::consts::FRAC_PI_2 + 0.01, std::f32::consts::FRAC_PI_2 - 0.01);
    }
    // Compute the view matrix (right-handed)
    pub fn get_view_projection(&self) -> Mat4 {
        let (width, height) = self.size;
        let forward = self.look();
        let target = self.position + forward; // Point the camera is looking at
        let up = Vec3::Y; // World up vector (Y-axis)
        let proj = Mat4::perspective_rh(45., width / height, 0.1, 100.);
        proj * Mat4::look_at_rh(self.position, target, up)
    }
    pub fn position_mut(&mut self, position: impl FnOnce(&mut Vec3)) {
        position(&mut self.position);
    }
}

pub struct CameraPlugin {
    pub buffer: UniformBuffer<Mat4>,
    pub bind_group: BindGroup,
}

bind_group_layout!(
    CameraPlugin,
    wgpu::BindGroupLayoutDescriptor {
        label: struct_label!(),
        entries: &[
            // CAMERA
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    }
);

impl CameraPlugin {
    pub fn new(camera: &Camera) -> Self {
        camera
            .get_view_projection()
            .pipe(|camera| UniformBuffer::new_init(&camera))
            .pipe(|buffer| {
                device()
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: struct_label!(),
                        layout: Self::bind_group_layout(),
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: buffer.as_ref().as_entire_binding(),
                        }],
                    })
                    .pipe(|bind_group| Self { buffer, bind_group })
            })
    }
}
