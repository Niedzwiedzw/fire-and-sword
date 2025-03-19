use {
    glam::{Mat4, Quat, Vec3},
    shader_types::glam,
    tap::prelude::*,
};

pub const SENSITIVITY: f32 = 0.04;

// A basic camera struct with common properties
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    position: Vec3,
    look: Vec3,
    up: Vec3,
    fov: f32,         // Field of view in radians
    aspect: f32,      // Aspect ratio (width/height)
    near: f32,        // Near clipping plane
    far: f32,         // Far clipping plane
    view: Mat4,       // Cached view matrix
    projection: Mat4, // Cached projection matrix
}

impl Camera {
    pub fn default_for_size((width, height): (f32, f32)) -> Self {
        Self::new((0.0, 1.0, 2.0).into(), -Vec3::Z, glam::Vec3::Y, 45., width / height, 0.1, 100.)
    }
}

impl Camera {
    // Create a new camera with default values
    pub fn new(position: Vec3, look: Vec3, up: Vec3, fov: f32, aspect: f32, near: f32, far: f32) -> Self {
        Camera {
            position,
            look,
            up,
            fov,
            aspect,
            near,
            far,
            view: Mat4::IDENTITY,
            projection: Mat4::IDENTITY,
        }
        .tap_mut(|c| c.update_matrices())
    }
    pub fn look(&self) -> &Vec3 {
        &self.look
    }
    // Update view and projection matrices
    fn update_matrices(&mut self) {
        // Create view matrix (camera's orientation and position)
        self.view = Mat4::look_at_rh(self.position, self.position + self.look, self.up);

        // Create projection matrix
        self.projection = Mat4::perspective_rh(self.fov, self.aspect, self.near, self.far);
    }

    // Get the combined view-projection matrix for shader use
    pub fn get_view_projection(&self) -> Mat4 {
        self.projection * self.view
    }

    // Update camera position
    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
        self.update_matrices();
    }

    pub fn position_mut(&mut self, position: impl FnOnce(&mut Vec3)) {
        position(&mut self.position);
        self.update_matrices();
    }

    // Update camera target
    pub fn set_look(&mut self, look: Vec3) {
        self.look = look;
        self.update_matrices();
    }

    // Update aspect ratio (when window resizes)
    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
        self.update_matrices();
    }

    // Simple orbit control
    pub fn orbit(&mut self, delta_yaw: f32, delta_pitch: f32) {
        let forward = self.look.normalize();
        let right = forward.cross(self.up).normalize();

        // Calculate distance to target
        let distance = 1.;

        // Create rotation quaternions
        let yaw = Quat::from_axis_angle(self.up, delta_yaw);
        let pitch = Quat::from_axis_angle(right, delta_pitch);

        // Combine rotations
        let rotation = yaw * pitch;

        // Update position
        let direction = rotation * forward;
        self.position = self.position + self.look.normalize() - direction * distance;

        self.update_matrices();
    }
}
