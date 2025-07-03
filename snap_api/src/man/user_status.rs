use crate::error::{ApiError, ApiResult};
use crate::{tt, AppState};
use async_graphql::async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use derive_new::new;
use redis::AsyncCommands;

#[derive(new)]
pub struct UserStatus {
    conn: redis::aio::MultiplexedConnection,
}

#[derive(new)]
pub struct EmailCount {
    conn: redis::aio::MultiplexedConnection,
    key: String,
    pub count: i32,
}

impl EmailCount {
    const MAX_COUNT: i32 = 5;

    pub fn limit(&self) -> bool {
        self.count < Self::MAX_COUNT
    }

    pub async fn update_email_send_count(&mut self) -> ApiResult {
        let count = if self.count >= Self::MAX_COUNT { Self::MAX_COUNT } else { self.count + 1 };

        redis::cmd("SET")
            .arg(&self.key)
            .arg(count)
            .arg("EX")
            .arg(100)
            .query_async(&mut self.conn)
            .await?;
        Ok(())
    }
}

impl UserStatus {
    fn key(email: &str) -> String {
        format!("email:{}", email)
    }
    fn code(email: &str, code: &str) -> String {
        format!("code:{}", email)
    }
    pub(crate) async fn valid_code(&mut self, email: &str, code: &str) -> ApiResult {
        let k = Self::code(email, code);
        let s: Option<String> = self.conn.get(&k).await?;
        if s.is_none() {
            return Err(ApiError::User(tt!("messages.user.signup.not_valid_code")));
        }
        Ok(())
    }
    pub async fn gen_code(&self, email: &str) -> ApiResult<String> {
        let mut conn = self.conn.clone();
        let code: u32 = rand::random();
        let mut code = code.to_be_bytes();
        for x in code.as_mut() {
            let c = *x;
            let mut r1 = c & 0x0F;
            let mut r2 = (c & 0xF0) >> 4;
            if r1 > 9 {
                r1 -= 9;
            }
            if r2 > 9 {
                r2 -= 9;
            }
            *x = (r2 << 4) + r1;
        }
        let code = hex::encode(code);
        let k = Self::code(email, &code);
        redis::cmd("SET").arg(&k).arg("").arg("EX").arg(100).query_async(&mut conn).await?;
        Ok(code)
    }

    pub async fn email_send_count(&self, email: &str) -> ApiResult<EmailCount> {
        let k = Self::key(email);
        let mut conn = self.conn.clone();
        let s: Option<i32> = conn.get(&k).await?;
        Ok(EmailCount::new(conn, k, s.unwrap_or(0)))
    }
}

#[async_trait]
impl FromRequestParts<AppState> for UserStatus {
    type Rejection = ApiError;
    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let conn = state.redis.get().await?;
        Ok(Self::new(conn))
    }
}
