use {crate::run::rendering::camera::Camera, shader_types::Instance};

pub struct GameState {
    pub camera: Camera,
    pub instances: Vec<Instance>,
}
