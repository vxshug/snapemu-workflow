use crate::service::user::UserLang;
use common_define::Id;
use derive_new::new;
use std::future::Future;
use tracing::{debug, Instrument};

tokio::task_local! {
    static USERNAME: &'static str;
}

tokio::task_local! {
    static USER: CurrentUser;
}

tokio::task_local! {
    static LANG: UserLang;
}

#[derive(Copy, Clone, new)]
pub struct CurrentUser {
    pub id: Id,
    pub username: &'static str,
}

pub async fn run_with_user<F>(id: Id, username: String, f: F) -> F::Output
where
    F: Future,
{
    let span_user = username.clone();
    let s = Box::into_raw(username.into_boxed_str());
    let t = unsafe { Box::from_raw(s) };
    let user = unsafe { CurrentUser { id, username: &*s } };
    let response = USER
        .scope(user, f)
        .instrument(tracing::debug_span!("user", user_id = id.to_string(), username = span_user))
        .await;
    debug!(auth = t.as_ref(), "run with user");
    response
}

pub fn get_current_user() -> CurrentUser {
    USER.get()
}

pub async fn run_with_lang<F>(lang: UserLang, f: F) -> F::Output
where
    F: Future,
{
    LANG.scope(lang, f).await
}

pub fn get_lang() -> UserLang {
    LANG.try_with(|e| *e).unwrap_or_default()
}
