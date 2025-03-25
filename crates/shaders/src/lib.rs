#![cfg_attr(target_arch = "spirv", no_std)]
// #![cfg_attr(not(debug_assertions), deny(warnings))]

// #[cfg(target_arch = "spirv")]
// use spirv_std::num_traits::Float as _;
use {
    glam::{Affine3A, Mat4, Vec3, Vec4Swizzles},
    lighting::LightContext,
    shader_types::{light_source::LightSource, model::ModelVertex, Instance},
    spirv_std::{glam::Vec4, image::Image2d, spirv, Sampler},
};

pub mod lighting;

#[spirv(fragment)]
pub fn main_fs(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] camera: &Mat4,
    #[spirv(descriptor_set = 2, binding = 0)] image: &Image2d,
    #[spirv(descriptor_set = 2, binding = 1)] sampler: &Sampler,
    #[spirv(storage_buffer, descriptor_set = 4, binding = 0)] light_sources: &[LightSource],
    model_vertex: ModelVertex,
    output: &mut Vec4,
) {
    let image_color = image.sample(*sampler, model_vertex.tex_coords);
    {
        let mut lighting = Vec3::new(0., 0., 0.);

        // no iterators, need to use loop
        let mut idx = 0;
        loop {
            if idx == light_sources.len() {
                break;
            }
            let light_source = light_sources[idx];
            if model_vertex
                .position
                .xyz()
                .distance_squared(light_source.position.xyz())
                <= 2000.
            {
                let light_context = LightContext::new(model_vertex, light_source, camera);
                light_context.apply_light(&mut lighting);
            }

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
    vertex.normal = (Affine3A::from_quat(instance.rotation).transform_vector3(vertex.normal.xyz())).extend(0.);

    *out_pos = *camera * vertex.position;
    *output = vertex;
}
