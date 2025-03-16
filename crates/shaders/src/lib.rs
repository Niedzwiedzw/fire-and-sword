#![cfg_attr(target_arch = "spirv", no_std)]
// HACK(eddyb) can't easily see warnings otherwise from `spirv-builder` builds.
// #![deny(warnings)]

#[cfg(target_arch = "spirv")]
#[allow(unused_imports)]
use spirv_std::num_traits::Float;
use {
    shader_types::{Color, Vertex},
    spirv_std::{
        glam::{vec4, Vec4},
        spirv,
    },
};

#[spirv(fragment)]
pub fn main_fs(output: &mut Vec4) {
    *output = vec4(1.0, 0.0, 0.0, 1.0);
}

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(vertex_index)] in_vertex_index: i32,

    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] input: &[Vertex],
    #[spirv(position, invariant)] out_pos: &mut Vec4,
) {
    let Vertex {
        color: Color([r, g, b, a]),
        position,
    } = &input[in_vertex_index as usize];

    *out_pos = position.with_w(2.)
}
