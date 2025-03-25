#[cfg(target_arch = "spirv")]
#[allow(unused_imports)]
use spirv_std::num_traits::Float;
use {
    core::ops::{Add, Div, Mul},
    glam::{Mat4, Vec3, Vec4Swizzles},
    shader_types::{light_source::LightSource, model::ModelVertex, tap::prelude::*, Color},
};

pub struct LightContext {
    eye_direction: Vec3,
    model_vertex: ModelVertex,
    light_source: LightSource,
    light_ray: Vec3,
    light_color: Vec3,
    dampen: f32,
}

impl LightContext {
    pub fn new(model_vertex: ModelVertex, light_source: LightSource, camera: &Mat4) -> Self {
        let light_ray = light_source.position.xyz() - model_vertex.position.xyz();
        Self {
            dampen: (1. / light_ray.length_squared().add(0.0001).div(10.)).min(1.),
            eye_direction: camera.transform_vector3(Vec3::Z).normalize(),
            light_ray,
            light_color: light_source
                .color
                .pipe(|Color([r, g, b, _])| Vec3::new(r, g, b)),
            model_vertex,
            light_source,
        }
    }
    fn apply_specular(&self, light_buffer: &mut Vec3) {
        let eye_direction = self.eye_direction;
        let reflection_direction = (-self.light_ray.reflect(self.model_vertex.normal.xyz())).normalize();
        let specular_intensity = reflection_direction.dot(-eye_direction).max(0.);
        *light_buffer += self.light_color
            * specular_intensity
                .powf(128.)
                .mul(2.0f32.powf(1.4))
                .clamp(0., 1.)
            * 0.33
            * self.dampen;
    }
    fn apply_ambient(&self, light_buffer: &mut Vec3) {
        self.pipe(
            |Self {
                 model_vertex: _,
                 light_color,
                 light_source: _,
                 light_ray: _,
                 eye_direction: _,
                 dampen: _,
             }| {
                let light = light_color * 0.06;
                // let light = light * dampen;
                *light_buffer += light;
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
                         position: _,
                         normal,
                         tex_coords: _,
                         padding: _,
                     },
                 light_source: LightSource { position: _, color: _ },
                 light_ray,
                 eye_direction: _,
                 dampen,
             }| {
                let diffuse_intensity = normal.xyz().dot(light_ray.normalize()).max(0.);
                // let diffuse_intensity = diffuse_intensity * dampen;
                *light_buffer += diffuse_intensity * light_color;
            },
        )
    }

    pub fn apply_light(&self, light_buffer: &mut Vec3) {
        self.apply_ambient(light_buffer);
        self.apply_diffuse(light_buffer);
        self.apply_specular(light_buffer);
    }
}
