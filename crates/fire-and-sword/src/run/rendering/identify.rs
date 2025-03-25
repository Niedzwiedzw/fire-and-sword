use {
    derivative::Derivative,
    std::{rc::Rc, sync::atomic::AtomicU16},
};

pub static ID_COUNTER: AtomicU16 = AtomicU16::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
pub struct Id(u16);

impl Id {
    fn generate() -> Self {
        Self(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

#[derive(Derivative)]
#[derivative(Hash(bound = ""))]
#[derivative(Clone(bound = ""))]
#[derivative(PartialEq(bound = ""))]
#[derivative(Eq(bound = ""))]
#[derivative(PartialOrd(bound = ""))]
#[derivative(Ord(bound = ""))]
pub struct WithId<T> {
    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    #[derivative(PartialOrd = "ignore")]
    #[derivative(Ord = "ignore")]
    inner: Rc<T>,
    id: Id,
}

impl<T> std::fmt::Debug for WithId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WithId<{}, ({})>", std::any::type_name::<Self>(), self.id)
    }
}

impl<T> WithId<T> {
    #[deprecated = "registration should only happen in loaders"]
    pub fn register(inner: T) -> Self {
        Self {
            inner: Rc::new(inner),
            id: Id::generate(),
        }
    }
}

impl<T> AsRef<T> for WithId<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}
