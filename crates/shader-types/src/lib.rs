#![no_std]

pub use {
    bytemuck::{self, Pod, Zeroable},
    glam::{self, Quat, Vec2, Vec3, Vec4},
    tap,
};

#[derive(Default, Clone, Copy, Debug, derive_more::From, Pod, Zeroable, derive_more::Constructor)]
#[repr(C)]
pub struct Color(pub [f32; 4]);

pub mod padding;

#[derive(Default, Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct Instance {
    pub position: Vec4,
    pub rotation: Quat,
}

pub mod model;
