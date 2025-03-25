use {
    crate::run::rendering::{camera::Camera, scene::Scene},
    shader_types::light_source::LightSource,
};

pub struct GameState {
    pub camera: Camera,
    pub scene: Option<Scene>,
    pub light_sources: Vec<LightSource>,
}
