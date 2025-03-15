#![cfg_attr(target_arch = "spirv", no_std)]
// HACK(eddyb) can't easily see warnings otherwise from `spirv-builder` builds.
#![deny(warnings)]

#[cfg(target_arch = "spirv")]
#[allow(unused_imports)]
use spirv_std::num_traits::Float;
use spirv_std::{
    glam::{vec4, Vec4},
    spirv,
};

#[spirv(fragment)]
pub fn main_fs(output: &mut Vec4) {
    *output = vec4(0.3, 0.2, 0.1, 1.0);
}

#[spirv(vertex)]
pub fn main_vs(#[spirv(vertex_index)] in_vertex_index: i32, #[spirv(position, invariant)] out_pos: &mut Vec4) {
    let x = (1 - in_vertex_index) as f32 * 0.5;
    let y = ((in_vertex_index & 1) * 2 - 1) as f32 * 0.5;

    *out_pos = vec4(x, y, 0.0, 1.0);
}
