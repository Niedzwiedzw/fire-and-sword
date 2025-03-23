use {
    glam::{Vec3, Vec4, Vec4Swizzles},
    shader_types::{light_source::LightSource, model::ModelVertex, tap::prelude::*, Color},
};

pub struct LightContext {
    model_vertex: ModelVertex,
    light_source: LightSource,
    light_direction: Vec3,
    light_color: Vec3,
}

impl LightContext {
    pub fn new(model_vertex: ModelVertex, light_source: LightSource) -> Self {
        Self {
            light_direction: light_source.position.xyz() - model_vertex.position.xyz(),
            light_color: light_source
                .color
                .pipe(|Color([r, g, b, _])| Vec3::new(r, g, b)),
            model_vertex,
            light_source,
        }
    }
    fn apply_specular(&self, light_buffer: &mut Vec3) {}
    fn apply_ambient(&self, light_buffer: &mut Vec3) {
        self.pipe(
            |Self {
                 model_vertex,
                 light_color,
                 light_source: LightSource { position, color },
                 light_direction,
             }| {
                *light_buffer += light_color * 0.1;
            },
        )
    }
    /// object should be completely black if there is no diffuse lighting
    fn apply_diffuse(&self, light_buffer: &mut Vec3) {
        self.pipe(
            |Self {
                 light_color,
                 model_vertex:
                     ModelVertex {
                         position,
                         normal,
                         tex_coords: _,
                         padding: _,
                     },
                 light_source: LightSource { position: _, color: _ },
                 light_direction,
             }| {
                // let eye_direction = (-position.xyz()).normalize();
                // let reflection_direction = (-light_direction.normalize().reflect(normal.xyz())).normalize();
                let diffuse_intensity = normal.xyz().dot(light_direction.normalize()).max(0.);
                *light_buffer += diffuse_intensity * light_color;
            },
        )
    }

    pub fn apply_light(&self, light_buffer: &mut Vec3) {
        // self.apply_ambient(light_buffer);
        self.apply_diffuse(light_buffer);
    }
}
