use crate::error::{ApiError, ApiResult};
use crate::man::UserManager;
use crate::service::device::group::{DeviceGroupService, ReqDeviceGroup};
use crate::service::user::{Token, UserLang, UserService};
use crate::{run_with_user, tt, AppState};
use common_define::Id;
use serde::{Deserialize, Serialize};

use super::token::TokenService;

use common_define::db::{UsersActiveModel, UsersColumn, UsersEntity, UsersModel};
use common_define::time::Timestamp;
use redis_macros::{FromRedisValue, ToRedisArgs};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, TransactionTrait,
};

#[derive(Serialize, Deserialize, FromRedisValue, ToRedisArgs)]
pub(crate) struct RedisToken {
    pub id: Id,
    pub name: String,
}

impl RedisToken {
    pub fn new<N: Into<String>>(id: Id, name: N) -> Self {
        Self { id, name: name.into() }
    }
}

#[derive(Deserialize, Serialize, Debug, utoipa::ToSchema)]
pub(crate) struct LoginUser {
    #[schema(example = "snapemu")]
    username: String,
    #[schema(example = "snapemu")]
    password: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct TokenTime {
    expires: i64,
}

impl UserService {
    pub(crate) async fn local_login(
        lang: UserLang,
        req_user: LoginUser,
        state: &AppState,
    ) -> ApiResult<Token> {
        let is_email = req_user.username.contains("@");
        let user = if is_email {
            UsersEntity::find()
                .filter(UsersColumn::Email.eq(&req_user.username))
                .one(&state.db)
                .await?
        } else {
            UsersEntity::find()
                .filter(UsersColumn::UserLogin.eq(&req_user.username))
                .one(&state.db)
                .await?
        };
        let user = user.ok_or(ApiError::Auth(tt!(
            "messages.user.login.unfounded",
            name = req_user.username
        )))?;
        let res = crate::utils::PasswordHash::check_password(&req_user.password, &user.password);
        if !res {
            return Err(ApiError::Auth(tt!("messages.user.login.error")));
        }
        let token = TokenService::create_token(&user, state).await?;
        Ok(Token {
            access_token: token.access_token,
            expires: token.expires,
            refresh_token: token.refresh_token,
        })
    }

    pub(crate) async fn login(
        lang: UserLang,
        req_user: LoginUser,
        state: &AppState,
    ) -> ApiResult<Token> {
        let user_manager = UserManager::load_from_config()?;
        if let Some(user_manager) = user_manager {
            let auth_user = user_manager
                .password_login(req_user.username.as_str(), req_user.password.as_str())
                .await?;

            let mut user: Option<UsersModel> = UsersEntity::find()
                .filter(UsersColumn::UId.eq(auth_user.id))
                .one(&state.db)
                .await?;

            if user.is_none() {
                let group = ReqDeviceGroup { name: "All".into(), description: "All".into() };
                let model = UsersActiveModel {
                    id: Default::default(),
                    u_id: ActiveValue::Set(auth_user.id),
                    user_login: ActiveValue::Set(auth_user.username.clone()),
                    user_nick: ActiveValue::Set(auth_user.username),
                    password: ActiveValue::Set("".into()),
                    email: ActiveValue::Set(auth_user.email),
                    active: ActiveValue::Set(true),
                    active_token: ActiveValue::Set("".into()),
                    picture: ActiveValue::Set("".into()),
                    create_time: ActiveValue::Set(Timestamp::now()),
                };
                let mut redis = state.redis.get().await?;
                let r = state
                    .db
                    .transaction::<_, _, ApiError>(|ctx| {
                        Box::pin(async move {
                            let user = model.insert(ctx).await?;
                            run_with_user(user.id, user.user_login.clone(), async {
                                DeviceGroupService::create_group_default(group, &mut redis, ctx)
                                    .await
                            })
                            .await?;
                            Ok(user)
                        })
                    })
                    .await?;

                user = Some(r);
            }
            let user = user.ok_or(ApiError::Auth(tt!(
                "messages.user.login.unfounded",
                name = req_user.username
            )))?;
            let token = TokenService::create_token(&user, state).await?;
            return Ok(Token {
                access_token: token.access_token,
                expires: token.expires,
                refresh_token: token.refresh_token,
            });
        }
        Self::local_login(lang, req_user, state).await
    }
    pub(crate) async fn token_login<R: redis::aio::ConnectionLike + redis::AsyncCommands>(
        token: &str,
        conn: &mut R,
    ) -> ApiResult<RedisToken> {
        let token: Option<RedisToken> = conn
            .get_ex(
                TokenService::key(token),
                redis::Expiry::EX(TokenService::ACCESS_EXPIRES as u64),
            )
            .await?;

        let token = token.ok_or(ApiError::Access(tt!("messages.user.login.token_err")))?;
        Ok(token)
    }
}
