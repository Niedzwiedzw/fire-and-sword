#![no_std]

pub use {
    bytemuck::{self, Pod, Zeroable},
    glam::{self, Vec2, Vec3, Vec4},
    tap,
};
use {glam::Quat, padding::WithPadding};

#[derive(Default, Clone, Copy, Debug, derive_more::From, Pod, Zeroable, derive_more::Constructor)]
#[repr(C)]
pub struct Color(pub [f32; 4]);

pub mod padding;

#[derive(Default, Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec4,
    pub tex_coords: Vec2,
    pub padding: WithPadding<2, ()>,
}

#[derive(Default, Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct Instance {
    pub position: Vec4,
    pub rotation: Quat,
}

pub mod model {
    use {
        crate::padding::WithPadding,
        bytemuck::{Pod, Zeroable},
        glam::{Vec2, Vec4},
    };

    #[repr(C)]
    #[derive(Copy, Clone, Debug, Pod, Zeroable)]
    pub struct ModelVertex {
        pub position: Vec4,
        pub normal: Vec4,
        pub tex_coords: Vec2,
        pub padding: WithPadding<2, ()>,
    }
}
