mod base64;
mod check;
mod hash;
mod model;
mod passwd;
mod random;

pub(super) use base64::Base64;
pub(crate) use check::Checker;
pub(super) use hash::Hash;
pub(crate) use passwd::PasswordHash;
pub(crate) use random::Rand;

pub(crate) fn block_on<F: std::future::Future + Send + 'static>(future: F) -> F::Output
where
    F::Output: Send,
{
    let handle = tokio::runtime::Handle::current();
    let r = std::thread::spawn(move || handle.block_on(future));
    r.join().unwrap()
}
