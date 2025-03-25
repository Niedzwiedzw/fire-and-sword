use {
    crate::run::rendering::{
        camera::Camera,
        render_pass::WithInstance,
        scene::{Node, Scene, WithTransform},
    },
    shader_types::light_source::LightSource,
};

pub struct GameState {
    pub camera: Camera,
    pub scene: Option<Scene>,
    pub light_sources: Vec<LightSource>,
}
