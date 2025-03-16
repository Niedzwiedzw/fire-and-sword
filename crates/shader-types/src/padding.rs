use super::bytemuck::{Pod, Zeroable};

#[derive(Clone, Copy, Debug, derive_more::From)]
#[repr(C)]
pub struct WithPadding<const N: usize, T> {
    pub inner: T,
    padding: [f32; N],
}

impl<const N: usize, T> core::default::Default for WithPadding<N, T>
where
    T: core::default::Default,
{
    fn default() -> Self {
        Self::pad(T::default())
    }
}

impl<const N: usize, T> WithPadding<N, T> {
    pub const fn pad(inner: T) -> Self {
        Self { inner, padding: [0.; N] }
    }
}

pub const fn pad<const N: usize, T: Pod + Zeroable>(inner: T) -> WithPadding<N, T> {
    WithPadding::pad(inner)
}

unsafe impl<const N: usize, T> Zeroable for WithPadding<N, T> where T: Zeroable {}
unsafe impl<const N: usize, T> Pod for WithPadding<N, T> where T: Pod {}
