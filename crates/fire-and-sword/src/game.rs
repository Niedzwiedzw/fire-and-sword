use {
    crate::run::rendering::camera::Camera,
    shader_types::{light_source::LightSource, Instance},
};

pub struct GameState {
    pub camera: Camera,
    pub instances: Vec<Instance>,
    pub light_sources: Vec<LightSource>,
}
