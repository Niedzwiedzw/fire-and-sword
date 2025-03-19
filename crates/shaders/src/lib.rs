#![cfg_attr(target_arch = "spirv", no_std)]
// HACK(eddyb) can't easily see warnings otherwise from `spirv-builder` builds.
// #![deny(warnings)]

#[cfg(target_arch = "spirv")]
#[allow(unused_imports)]
use spirv_std::num_traits::Float;
use {
    glam::{Mat4, Vec4Swizzles},
    shader_types::{Color, Vertex},
    spirv_std::{
        glam::{vec4, Vec4},
        image::Image2d,
        spirv,
        Image,
        Sampler,
    },
};

#[spirv(fragment)]
pub fn main_fs(
    #[spirv(descriptor_set = 0, binding = 1)] image: &Image2d,
    #[spirv(descriptor_set = 0, binding = 2)] sampler: &Sampler,
    input: Vertex,
    output: &mut Vec4,
) {
    *output = image.sample(*sampler, input.tex_coords);
}

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(vertex_index)] in_vertex_index: i32,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] input: &[Vertex],
    #[spirv(uniform, descriptor_set = 0, binding = 3)] camera: &Mat4,
    #[spirv(position)] out_pos: &mut Vec4,
    output: &mut Vertex,
) {
    let mut vertex = input[in_vertex_index as usize];
    vertex.position = *camera * vertex.position;

    *out_pos = vertex.position;
    *output = vertex;
}
