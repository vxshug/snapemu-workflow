use crate::api::restful::admin::JwtClaims;
use crate::api::SnJson;
use crate::error::{ApiError, ApiResponseResult};
use crate::man::UserManager;
use crate::AppState;
use axum::extract::State;
use axum::routing::post;
use common_define::db::{SnapAdminColumn, SnapAdminEntity, UsersColumn, UsersEntity};
use common_define::time::Timestamp;
use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().route("/", post(login))
}

#[derive(Deserialize, utoipa::ToSchema)]
struct AdminLoginBody {
    #[schema(example = "snapemu")]
    username: String,
    #[schema(example = "snapemu")]
    password: String,
}

#[derive(Serialize, utoipa::ToSchema)]
struct JwtBody {
    #[schema(example = "<token>")]
    token: String,
}

use crate::load::load_config;
use crate::utils::PasswordHash;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

#[derive(OpenApi)]
#[openapi(paths(login,), components(schemas(AdminLoginBody, JwtBody)))]
pub struct UserApi;

///
/// Management login interface
#[utoipa::path(
    post,
    path = "/login",
    request_body = AdminLoginBody,
    security(
    ()
    ),
)]
async fn login(
    State(state): State<AppState>,
    SnJson(user): SnJson<AdminLoginBody>,
) -> ApiResponseResult<JwtBody> {
    let user_manager = UserManager::load_from_config()?;
    let auth_user = match user_manager {
        Some(user_manager) => {
            let auth_user =
                user_manager.password_login(user.username.as_str(), user.password.as_str()).await?;
            SnapAdminEntity::find()
                .filter(SnapAdminColumn::UId.eq(auth_user.id))
                .one(&state.db)
                .await?
                .ok_or_else(|| ApiError::User("Admin user does not exist".into()))?
        }
        None => {
            let db_user = UsersEntity::find()
                .filter(UsersColumn::UserLogin.eq(user.username.clone()))
                .one(&state.db)
                .await?
                .ok_or_else(|| ApiError::User("Admin user does not exist".into()))?;
            if !PasswordHash::check_password(&user.password, &db_user.password) {
                return Err(ApiError::User("Password incorrect".into()));
            }
            SnapAdminEntity::find()
                .filter(SnapAdminColumn::UId.eq(db_user.u_id))
                .one(&state.db)
                .await?
                .ok_or_else(|| ApiError::User("Admin user does not exist".into()))?
        }
    };
    let exp = Timestamp::now() + chrono::Duration::days(3);
    let claims = JwtClaims {
        uid: auth_user.u_id,
        aid: auth_user.id,
        exp: exp.timestamp_millis(),
        name: user.username,
    };
    let config = load_config();
    let jwt_key: Hmac<Sha256> = Hmac::new_from_slice(config.jwt_key.as_bytes())
        .map_err(|err| ApiError::User("jwt_key length is error".into()))?;
    let token_str = claims.sign_with_key(&jwt_key).unwrap();
    Ok(JwtBody { token: token_str }.into())
}
