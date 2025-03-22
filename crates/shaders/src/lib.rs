#![cfg_attr(target_arch = "spirv", no_std)]
// HACK(eddyb) can't easily see warnings otherwise from `spirv-builder` builds.
// #![deny(warnings)]

#[cfg(target_arch = "spirv")]
#[allow(unused_imports)]
use spirv_std::num_traits::Float;
use {
    glam::{Affine3A, Mat4, Vec4Swizzles},
    shader_types::{model::ModelVertex, Instance},
    spirv_std::{glam::Vec4, image::Image2d, spirv, Sampler},
};

#[spirv(fragment)]
/// texture: [config::TEXTURE] (0, 1)
pub fn main_fs(
    #[spirv(descriptor_set = 2, binding = 0)] image: &Image2d,
    #[spirv(descriptor_set = 2, binding = 1)] sampler: &Sampler,
    input: ModelVertex,
    output: &mut Vec4,
) {
    *output = image.sample(*sampler, input.tex_coords);
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
    vertex.position = *camera * (instance.position.xyz() + Affine3A::from_quat(instance.rotation).transform_point3(vertex.position.xyz())).extend(1.);

    *out_pos = vertex.position;
    *output = vertex;
}
