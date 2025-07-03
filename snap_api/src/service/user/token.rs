use crate::error::{ApiError, ApiResult};
use crate::utils::{Base64, Hash, Rand};
use crate::{tt, AppState};
use std::str::FromStr;

use crate::service::user::RedisToken;
use chrono::{Duration, Utc};
use common_define::db::{
    UserTokenActiveModel, UserTokenColumn, UserTokenEntity, UserTokenModel, UsersEntity, UsersModel,
};
use common_define::time::Timestamp;
use common_define::Id;
use deadpool_redis::redis::AsyncCommands;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use tracing::warn;
use utils::base64::EncodeBase64;

#[derive(serde::Deserialize, serde::Serialize, Copy, Clone, strum::EnumString, strum::AsRefStr)]
pub(crate) enum TokenType {
    Access,
    Refresh,
}

pub(crate) struct TokenService;

fn gen_access_token(id: Id) -> String {
    let mut buf = [0; 8];
    buf[0..4].copy_from_slice(Utc::now().timestamp_subsec_millis().to_le_bytes().as_slice());
    buf[4..8].copy_from_slice(Rand::u32().to_le_bytes().as_slice());
    Base64::standard_encode_no_pad(id.to_string()) + &Hash::sha256_base64(buf)[0..20]
}
fn gen_refresh_token(id: Id) -> String {
    let mut buf = [0; 24];
    buf[0..16].copy_from_slice(uuid::Uuid::new_v4().as_ref());
    buf[16..20].copy_from_slice(Utc::now().timestamp_subsec_millis().to_le_bytes().as_slice());
    buf[20..24].copy_from_slice(Rand::u32().to_le_bytes().as_slice());
    let mut s = Hash::sha256(buf);
    s.extend_from_slice(uuid::Uuid::new_v4().as_ref());
    s.encode_base64()
}

/// Certified token
#[derive(Deserialize, Serialize, Debug, utoipa::ToSchema)]
pub(crate) struct Token {
    /// Access token Used to access resources
    #[schema(example = "PLx4ZR0szB/7i9qGS+m4RGwsxSmOfuw5oAhQUcTKQo1JJjDf")]
    pub(crate) access_token: String,
    /// The expiration time of the access resource is expressed in seconds
    #[schema(example = "864000")]
    pub(crate) expires: i64,
    /// Refreshing a token is used to update the access token
    #[schema(example = "xlXCP39TBdK8Kjo50T+cjetguSGwTQqi")]
    pub(crate) refresh_token: String,
}

impl TokenService {
    pub(crate) const ACCESS_EXPIRES: i64 = 60 * 60 * 24 * 10;
    pub(crate) const REFRESH_EXPIRES: i64 = 60 * 60 * 24 * 15;

    pub(crate) async fn access_token<C: ConnectionTrait>(
        user_id: Id,
        conn: &C,
    ) -> ApiResult<String> {
        let token = gen_access_token(user_id);
        let expires_time = Utc::now() + Duration::seconds(Self::ACCESS_EXPIRES);
        let model = UserTokenActiveModel {
            id: Default::default(),
            user_id: ActiveValue::Set(user_id),
            token: ActiveValue::Set(token.clone()),
            token_type: ActiveValue::Set(TokenType::Access.as_ref().to_string()),
            enable: ActiveValue::Set(true),
            expires_time: ActiveValue::Set(expires_time.into()),
            create_time: ActiveValue::Set(Timestamp::now()),
        };
        model.insert(conn).await?;
        Ok(token)
    }
    pub(crate) async fn refresh_token<C: ConnectionTrait>(
        user_id: Id,
        conn: &C,
    ) -> ApiResult<String> {
        let token = gen_refresh_token(user_id);
        let expires_time = Utc::now() + Duration::seconds(Self::REFRESH_EXPIRES);
        let model = UserTokenActiveModel {
            id: Default::default(),
            user_id: ActiveValue::Set(user_id),
            token: ActiveValue::Set(token.clone()),
            token_type: ActiveValue::Set(TokenType::Refresh.as_ref().to_string()),
            enable: ActiveValue::Set(true),
            expires_time: ActiveValue::Set(expires_time.into()),
            create_time: ActiveValue::Set(Timestamp::now()),
        };
        model.insert(conn).await?;
        Ok(token)
    }

    pub(crate) fn auth_token(token: &UserTokenModel) -> ApiResult<TokenType> {
        let token_type = TokenType::from_str(token.token_type.as_str()).map_err(|_e| {
            warn!("invalid token type: `{}`", token.token_type);
            ApiError::Access(tt!("messages.user.login.token_err"))
        })?;
        match token_type {
            TokenType::Access => {
                if token.expires_time < Timestamp::now() {
                    Err(ApiError::Access(tt!("messages.user.access.access_timeout")))
                } else if !token.enable {
                    Err(ApiError::Access(tt!("messages.user.access.access_timeout")))
                } else {
                    Ok(token_type)
                }
            }
            TokenType::Refresh => {
                if token.expires_time < Timestamp::now() {
                    Err(ApiError::Auth(tt!("messages.user.access.refresh_timeout")))
                } else if !token.enable {
                    Err(ApiError::User(tt!("messages.user.access.refresh_disabled")))
                } else {
                    Ok(token_type)
                }
            }
        }
    }

    pub(crate) async fn refresh_key(token: &str, state: &AppState) -> ApiResult<Token> {
        let (refresh, user) = UserTokenEntity::find()
            .filter(UserTokenColumn::Token.eq(token))
            .find_also_related(UsersEntity)
            .one(&state.db)
            .await?
            .ok_or(ApiError::Access(tt!("messages.user.login.token_err")))?;
        let token_type = Self::auth_token(&refresh)?;

        match token_type {
            TokenType::Access => Err(ApiError::User("require refresh token".into())),
            TokenType::Refresh => match user {
                Some(user) => Self::create_token(&user, state).await,
                None => Err(ApiError::User("Invalid token".into())),
            },
        }
    }
    pub(crate) fn key(token: &str) -> String {
        format!("token:{}", token)
    }

    pub(crate) async fn create_token(user: &UsersModel, state: &AppState) -> ApiResult<Token> {
        let user_id = user.id;
        let (access, refresh) = state
            .db
            .transaction::<_, _, ApiError>(|ctx| {
                Box::pin(async move {
                    let access = Self::access_token(user_id, ctx).await?;
                    let refresh = Self::refresh_token(user_id, ctx).await?;
                    Ok((access, refresh))
                })
            })
            .await?;

        let mut redis = state.redis.get().await?;

        let r = RedisToken::new(user.id, &user.user_login);
        redis.set_ex(Self::key(&access), &r, Self::ACCESS_EXPIRES as u64).await?;
        Ok(Token { access_token: access, expires: Self::ACCESS_EXPIRES, refresh_token: refresh })
    }

    pub(crate) async fn delete_user_token<
        C: ConnectionTrait,
        R: redis::aio::ConnectionLike + redis::AsyncCommands,
    >(
        user_id: Id,
        conn: &C,
        redis: &mut R,
    ) -> ApiResult {
        let express = Timestamp::now() - Duration::seconds(Self::ACCESS_EXPIRES);
        let access_token = UserTokenEntity::find()
            .filter(
                UserTokenColumn::UserId
                    .eq(user_id)
                    .and(UserTokenColumn::TokenType.eq(TokenType::Access.as_ref()))
                    .and(UserTokenColumn::ExpiresTime.gt(express)),
            )
            .all(conn)
            .await?;

        if !access_token.is_empty() {
            let token: Vec<_> = access_token.iter().map(|t| t.token.as_str()).collect();
            redis.del(token.as_slice()).await?;
        }
        UserTokenEntity::delete_many()
            .filter(UserTokenColumn::UserId.eq(user_id))
            .exec(conn)
            .await?;
        Ok(())
    }
}
