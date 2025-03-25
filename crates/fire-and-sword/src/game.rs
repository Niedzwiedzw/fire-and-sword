use {
    crate::run::rendering::{
        camera::Camera,
        render_pass::WithInstance,
        scene::{Node, WithTransform},
    },
    shader_types::light_source::LightSource,
};

pub struct GameState {
    pub camera: Camera,
    pub instances: Vec<WithInstance<WithTransform<Node>>>,
    pub light_sources: Vec<LightSource>,
}
