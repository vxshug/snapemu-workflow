use crate::error::{ApiError, ApiResult};
use crate::man::user_status::UserStatus;
use crate::man::{EmailManager, UserManager};
use crate::service::decode::DecodeService;
use crate::service::device::group::{DeviceGroupService, ReqDeviceGroup};
use crate::service::device::DeviceService;
use crate::service::user::{TokenService, UserService};
use crate::utils::{Checker, PasswordHash};
use crate::{run_with_user, tt, utils, AppState, AppString};
use common_define::db::{UsersActiveModel, UsersColumn, UsersEntity};
use common_define::time::Timestamp;
use common_define::Id;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel,
    QueryFilter, TransactionTrait,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, utoipa::ToSchema)]
pub struct UserInfo {
    pub username: String,
    pub password: String,
    pub email: String,
}

#[derive(Deserialize, Debug, utoipa::ToSchema)]
pub struct UserReset {
    pub email: String,
}

#[derive(Deserialize, Debug, utoipa::ToSchema)]
pub struct UserResetPassword {
    pub email: String,
    pub code: String,
    pub password: String,
}

#[derive(Deserialize, Debug, utoipa::ToSchema)]
pub struct UserDelete {
    pub email: String,
    pub code: String,
    pub password: String,
}

impl UserService {
    pub(crate) async fn signup(info: UserInfo, state: &AppState) -> ApiResult<Id> {
        if !Checker::email(info.email.as_str()) {
            return Err(ApiError::User(tt!(
                "messages.user.signup.email_format",
                email = info.email
            )));
        }
        if !Checker::username(info.username.as_str()) {
            return Err(ApiError::User(tt!(
                "messages.user.signup.name_format",
                name = info.username
            )));
        }
        let user = UsersEntity::find()
            .filter(UsersColumn::UserLogin.eq(info.username.as_str()))
            .one(&state.db)
            .await?;

        if user.is_some() {
            return Err(ApiError::User(tt!(
                "messages.user.signup.name_exist",
                name = info.username
            )));
        }
        let user = UsersEntity::find()
            .filter(UsersColumn::Email.eq(info.email.as_str()))
            .one(&state.db)
            .await?;
        if user.is_some() {
            return Err(ApiError::User(tt!(
                "messages.user.signup.email_exist",
                email = info.email
            )));
        }
        let user_manager = UserManager::load_from_config()?;
        let (user_id, email) = match user_manager {
            Some(user_manager) => {
                let auth_user = user_manager
                    .password_signup(
                        info.username.as_str(),
                        info.password.as_str(),
                        info.email.as_str(),
                    )
                    .await?;
                if let Some(email) = &auth_user.email {
                    user_manager.active_email(auth_user.id, email).await?;
                }
                (auth_user.id, auth_user.email)
            }
            None => (uuid::Uuid::new_v4(), Some(info.email)),
        };

        let token =
            format!("{}:{}{}", "ok", utils::Rand::string(8), Timestamp::now().timestamp_millis());
        let password = PasswordHash::gen_password(&info.password);

        let model = UsersActiveModel {
            id: Default::default(),
            u_id: ActiveValue::Set(user_id),
            user_login: ActiveValue::Set(info.username.clone()),
            user_nick: ActiveValue::Set(info.username.clone()),
            password: ActiveValue::Set(password),
            email: ActiveValue::Set(email),
            active: ActiveValue::Set(true),
            active_token: ActiveValue::Set(token),
            picture: ActiveValue::Set("".into()),
            create_time: ActiveValue::Set(Timestamp::now()),
        };
        let mut redis = state.redis.get().await?;
        let user = state
            .db
            .transaction::<_, _, ApiError>(|ctx| {
                Box::pin(async move {
                    let user = model.insert(ctx).await?;
                    let group = ReqDeviceGroup { name: "All".into(), description: "All".into() };
                    run_with_user(
                        user.id,
                        user.user_login.clone(),
                        DeviceGroupService::create_group_default(group, &mut redis, ctx),
                    )
                    .await?;
                    Ok(user)
                })
            })
            .await?;

        Ok(user.id)
    }

    pub(crate) async fn verify_active_token<C: ConnectionTrait>(
        token: &str,
        user_manager: UserManager,
        conn: &C,
    ) -> ApiResult<String> {
        if token.is_empty() {
            return Err(ApiError::Auth(tt!("messages.user.login.token_err")));
        }
        let db_user =
            UsersEntity::find().filter(UsersColumn::ActiveToken.eq(token)).one(conn).await?;

        match db_user {
            None => Err(ApiError::Auth(tt!("messages.user.login.token_err"))),
            Some(user) => {
                if user.active {
                    return Err(ApiError::User(tt!("messages.user.signup.already_active")));
                }
                match user.email {
                    Some(email) => {
                        user_manager.active_email(user.u_id, email.as_str()).await?;
                        Ok("active success".to_string())
                    }
                    None => Err(ApiError::Auth(tt!("messages.user.login.token_err"))),
                }
            }
        }
    }

    pub(crate) async fn reset_password(
        mut status: UserStatus,
        user: &UserResetPassword,
        state: &AppState,
    ) -> ApiResult<String> {
        status.valid_code(&user.email, &user.code).await?;
        let user_manager = UserManager::load_from_config()?;
        if let Some(user_manager) = user_manager {
            let user_remote = user_manager.check_email(&user.email).await?.ok_or(
                ApiError::User(tt!("messages.user.signup.not_found_email", email = user.email)),
            )?;
            user_manager.reset_password(user_remote.id, &user.password).await?;
            return Ok("password is changed".to_string());
        }
        let model = UsersEntity::find()
            .filter(UsersColumn::Email.eq(&user.email))
            .one(&state.db)
            .await?
            .ok_or(ApiError::User(tt!(
                "messages.user.signup.not_found_email",
                email = user.email
            )))?;

        let mut model = model.into_active_model();
        let s = PasswordHash::gen_password(user.password.as_str());
        model.password = ActiveValue::Set(s);
        model.update(&state.db).await?;
        Ok("password is changed".to_string())
    }

    pub(crate) async fn reset(
        status: UserStatus,
        user: &UserReset,
        state: &AppState,
    ) -> ApiResult<AppString> {
        let user_manager = UserManager::load_from_config()?;
        let username = match user_manager {
            Some(user_manager) => {
                let auth_user = user_manager.check_email(user.email.as_str()).await?.ok_or(
                    ApiError::User(tt!("messages.user.signup.not_found_email", email = user.email)),
                )?;
                auth_user.username
            }
            None => {
                let model = UsersEntity::find()
                    .filter(UsersColumn::Email.eq(&user.email))
                    .one(&state.db)
                    .await?
                    .ok_or(ApiError::User(tt!(
                        "messages.user.signup.not_found_email",
                        email = user.email
                    )))?;
                model.user_login
            }
        };

        let mut c = status.email_send_count(&user.email).await?;

        if !c.limit() {
            return Err(ApiError::User(tt!("messages.user.signup.email_limit")))?;
        }

        c.update_email_send_count().await?;

        let code = status.gen_code(&user.email).await?;
        let email_manager = EmailManager::new()?;
        if let Some(email_manager) = email_manager {
            email_manager.valid_code(&username, &user.email, &code).await?;
        }

        Ok(tt!("messages.user.signup.email_send_success"))
    }

    pub(crate) async fn delete_local_user(user_id: Id, state: &AppState) -> ApiResult<()> {
        let redis = state.redis.clone();
        state
            .db
            .transaction::<_, _, ApiError>(|ctx| {
                Box::pin(async move {
                    let redis = &mut redis.get().await?;
                    DecodeService::delete_user_script(user_id, ctx).await?;
                    DeviceService::delete_by_user_id(user_id, redis, ctx).await?;
                    TokenService::delete_user_token(user_id, ctx, redis).await?;
                    DeviceGroupService::delete_by_user(user_id, redis, ctx).await?;
                    UsersEntity::delete_by_id(user_id).exec(ctx).await?;
                    Ok(())
                })
            })
            .await?;
        Ok(())
    }
    pub(crate) async fn delete_user(
        mut status: UserStatus,
        user: &UserDelete,
        state: &AppState,
    ) -> ApiResult<String> {
        status.valid_code(&user.email, &user.code).await?;
        let user_manager = UserManager::load_from_config()?;
        if let Some(user_manager) = user_manager {
            let user_remote = user_manager.check_email(&user.email).await?.ok_or(
                ApiError::User(tt!("messages.user.signup.not_found_email", email = user.email)),
            )?;
            let user_db = UsersEntity::find()
                .filter(UsersColumn::UId.eq(user_remote.id))
                .one(&state.db)
                .await?;
            if let Some(user_db) = user_db {
                Self::delete_local_user(user_db.id, state).await?;
                user_manager.delete_user(user_remote.id).await?;
                return Ok("user deleted".to_string());
            }
        }
        let user = UsersEntity::find()
            .filter(UsersColumn::Email.eq(&user.email))
            .one(&state.db)
            .await?
            .ok_or_else(|| ApiError::User("user not found".into()))?;
        Self::delete_local_user(user.id, state).await?;
        Ok("user deleted".to_string())
    }

    pub(crate) async fn delete(
        status: UserStatus,
        user: &UserReset,
        state: &AppState,
    ) -> ApiResult<AppString> {
        if let Some(user_manager) = UserManager::load_from_config()? {
            let auth_user = user_manager.check_email(user.email.as_str()).await?.ok_or(
                ApiError::User(tt!("messages.user.signup.not_found_email", email = user.email)),
            )?;
        }
        let auth_user = UsersEntity::find()
            .filter(UsersColumn::Email.eq(&user.email))
            .one(&state.db)
            .await?
            .ok_or(ApiError::User(tt!(
                "messages.user.signup.not_found_email",
                email = user.email
            )))?;

        let mut c = status.email_send_count(&user.email).await?;

        if !c.limit() {
            return Err(ApiError::User(tt!("messages.user.signup.email_limit")))?;
        }

        c.update_email_send_count().await?;

        let code = status.gen_code(&user.email).await?;
        let email_manager = EmailManager::new()?;
        if let Some(email_manager) = email_manager {
            email_manager.delete_valid_code(&auth_user.user_login, &user.email, &code).await?;
        }
        Ok(tt!("messages.user.signup.email_send_success"))
    }
}
