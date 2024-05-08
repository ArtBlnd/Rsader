#[cfg(not(target_arch = "wasm32"))]
mod impls {
    pub trait MaybeSend: Send {}
    impl<T> MaybeSend for T where T: Send {}

    pub trait MaybeSync: Sync {}
    impl<T> MaybeSync for T where T: Sync {}
}

#[cfg(any(target_arch = "wasm32"))]
mod impls {
    pub trait MaybeSend {}
    impl<T> MaybeSend for T {}

    pub trait MaybeSync {}
    impl<T> MaybeSync for T {}
}

pub use impls::*;
