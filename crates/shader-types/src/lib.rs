#![no_std]

pub use {
    bytemuck::{self, Pod, Zeroable},
    glam::{self, Vec3, Vec4},
    tap,
};

#[derive(Default, Clone, Copy, Debug, derive_more::From, Pod, Zeroable, derive_more::Constructor)]
#[repr(C)]
pub struct Color(pub [f32; 4]);

#[derive(Default, Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub color: Color,
    pub position: Vec4,
}

pub trait FromBufferAtIndex {
    fn from_buffer_u32_at_index(buffer: &[u32], index: i32) -> &Self;
    fn from_buffer_u8_at_index(buffer: &[u8], index: i32) -> &Self;
}

impl<T: Pod + Sized> FromBufferAtIndex for T {
    fn from_buffer_u32_at_index(buffer: &[u32], index: i32) -> &Self {
        let data = &buffer[(index as usize)..(index as usize + core::mem::align_of::<[Self; 1]>())];
        let data = unsafe { core::mem::transmute::<&[u32], &[Self]>(data) };
        &data[0]
    }
    fn from_buffer_u8_at_index(buffer: &[u8], index: i32) -> &Self {
        let data = &buffer[(index as usize)..(index as usize + core::mem::align_of::<[Self; 1]>())];
        let data = unsafe { core::mem::transmute::<&[u8], &[Self]>(data) };
        &data[0]
    }
}
