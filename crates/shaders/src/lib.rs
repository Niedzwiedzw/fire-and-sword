#![cfg_attr(target_arch = "spirv", no_std)]
// HACK(eddyb) can't easily see warnings otherwise from `spirv-builder` builds.
// #![deny(warnings)]

#[cfg(target_arch = "spirv")]
#[allow(unused_imports)]
use spirv_std::num_traits::Float;
use {
    glam::{Affine3A, Mat4, Vec3, Vec4Swizzles},
    shader_types::{light_source::LightSource, model::ModelVertex, Color, Instance},
    spirv_std::{glam::Vec4, image::Image2d, spirv, Sampler},
};

#[spirv(fragment)]
pub fn main_fs(
    #[spirv(descriptor_set = 2, binding = 0)] image: &Image2d,
    #[spirv(descriptor_set = 2, binding = 1)] sampler: &Sampler,
    #[spirv(storage_buffer, descriptor_set = 4, binding = 0)] light_sources: &[LightSource],
    ModelVertex {
        position: pixel_position,
        normal,
        tex_coords,
        padding: _,
    }: ModelVertex,
    output: &mut Vec4,
) {
    let normal = normal.normalize_or_zero();
    let image_color = image.sample(*sampler, tex_coords);
    {
        let mut lighting = Vec3::new(0., 0., 0.);

        // no iterators, need to use loop
        let mut idx = 0;
        loop {
            if idx == light_sources.len() {
                break;
            }
            let LightSource {
                position: light_source,
                color: Color([r, g, b, _a]),
            } = light_sources[idx];

            let light_color = Vec3::new(r, g, b);

            // AMBIENT
            let ambient = light_color * 0.05;
            lighting += ambient;

            // DIFFUSE
            let light_ray = pixel_position - light_source;
            let weaken = (1.0 / light_ray.length()).powf(0.2);
            let diffuse_strength = (-light_ray).xyz().dot(normal.xyz()).max(0.);
            let diffuse = diffuse_strength * light_color * weaken;
            lighting += diffuse * 0.06;

            // SPECULAR
            let reflect_source = light_ray.xyz().reflect(normal.xyz()).normalize_or_zero();
            let specular_strength = Vec3::new(0., 0., 1.).dot(reflect_source).max(0.).powf(256.);
            let specular = specular_strength * light_color;
            lighting += specular * 0.2;

            idx += 1;
        }
        *output = image_color * lighting.extend(1.);
    }
}

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(vertex_index)] in_vertex_index: i32,
    #[spirv(instance_index)] in_instance_index: i32,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] camera: &Mat4,
    #[spirv(storage_buffer, descriptor_set = 1, binding = 0)] input: &[ModelVertex],
    #[spirv(storage_buffer, descriptor_set = 3, binding = 0)] instances: &[Instance],
    #[spirv(position)] out_pos: &mut Vec4,
    output: &mut ModelVertex,
) {
    let mut vertex = input[in_vertex_index as usize];
    let instance = instances[in_instance_index as usize];
    vertex.position = (instance.position.xyz() + Affine3A::from_quat(instance.rotation).transform_point3(vertex.position.xyz())).extend(1.);
    // vertex.normal = (Affine3A::from_quat(instance.rotation).transform_vector3(vertex.normal.xyz())).extend(0.);

    *out_pos = *camera * vertex.position;
    *output = vertex;
}
