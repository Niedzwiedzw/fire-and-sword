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
