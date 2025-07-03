use std::sync::atomic::{AtomicU64, Ordering};

static GLOBAL_CLIENT_ID_COUNT: AtomicU64 = AtomicU64::new(0);

#[derive(Ord, PartialOrd, Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub struct ClientId(u64);

impl ClientId {
    pub fn next() -> Self {
        Self(GLOBAL_CLIENT_ID_COUNT.fetch_add(1, Ordering::SeqCst))
    }
}
