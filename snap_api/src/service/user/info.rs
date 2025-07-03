use crate::error::{ApiError, ApiResult};
use crate::load::load_config;
use crate::man::UserManager;
use crate::service::user::UserService;
use crate::utils::{Base64, PasswordHash, Rand};
use crate::{tt, AppState, CurrentUser};
use common_define::db::{UsersEntity, UsersModel};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColIdx, ConnectionTrait, EntityTrait, IntoActiveModel,
};
use std::borrow::Cow;
use std::fs;
use tracing::warn;

#[derive(serde::Deserialize, serde::Serialize, Debug, utoipa::ToSchema)]
pub(crate) struct UserPutInfo {
    pub(crate) password: Option<String>,
    pub(crate) old_password: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, utoipa::ToSchema)]
pub(crate) struct UserRespInfo {
    pub(crate) username: String,
    pub(crate) picture: String,
    pub(crate) email: String,
}

impl From<UsersModel> for UserRespInfo {
    fn from(value: UsersModel) -> Self {
        Self {
            username: value.user_login,
            picture: value.picture,
            email: value.email.unwrap_or_default(),
        }
    }
}

pub(crate) struct Picture<'a> {
    pub(crate) suffix: &'a str,
    pub(crate) content_type: &'a str,
    pub(crate) bytes: Vec<u8>,
}

impl Picture<'_> {
    pub(crate) fn new<'a, D: Into<Vec<u8>>>(
        suffix: &'a str,
        content_type: &'a str,
        bytes: D,
    ) -> Picture<'a> {
        Picture { suffix, content_type, bytes: bytes.into() }
    }
}

#[derive(serde::Serialize)]
struct OssPolicyCondition {
    bucket: String,
}

#[derive(serde::Serialize)]

struct OssPolicy {
    expiration: chrono::DateTime<chrono::Utc>,
    conditions: Vec<OssPolicyCondition>,
}

#[derive(Clone)]
pub struct AliyunOSS {
    prefix: &'static str,
    bucket: &'static str,
    bucket_url: &'static str,
    file_url: &'static str,
    key_id: &'static str,
    key_secret: &'static str,
}

impl AliyunOSS {
    fn load_from_config() -> Option<Self> {
        let config = load_config();
        if let Some(ref oss) = config.api.oss {
            let prefix = oss.prefix.clone();
            let bucket = oss.bucket.clone();
            let bucket_url = oss.bucket.clone();
            let file_url = oss.file_url.clone();
            let key_id = oss.key.clone();
            let key_secret = oss.secret.clone();

            let bucket_url: &'static str = if let Some(p) = bucket_url.strip_suffix('/') {
                p.to_string().leak()
            } else {
                bucket_url.leak()
            };

            let file_url = if let Some(p) = file_url.strip_suffix('/') {
                p.to_string().leak()
            } else {
                file_url.leak()
            };

            let prefix = if prefix.is_empty() {
                ""
            } else {
                let mut s = prefix.as_str();
                if let Some(p) = s.strip_prefix('/') {
                    s = p
                }
                if let Some(p) = s.strip_suffix('/') {
                    s = p
                }
                s.to_string().leak()
            };

            return Some(Self {
                prefix,
                bucket: bucket.leak(),
                bucket_url,
                file_url,
                key_id: key_id.leak(),
                key_secret: key_secret.leak(),
            });
        };
        None
    }
    async fn save_picture(&self, name: &str, picture: Picture<'_>) -> ApiResult<String> {
        let policy = OssPolicy {
            expiration: chrono::Utc::now() + chrono::Duration::days(1),
            conditions: vec![OssPolicyCondition { bucket: self.bucket.to_string() }],
        };
        let policy = Base64::standard_encode(serde_json::to_string(&policy)?);
        let sign = Base64::standard_encode(hmac_sha1::hmac_sha1(
            self.key_secret.as_bytes(),
            policy.as_bytes(),
        ));
        let mut header = reqwest::header::HeaderMap::new();
        let content_type = picture
            .content_type
            .parse()
            .map_err(|_| ApiError::User(tt!("messages.user.picture.content_type_err")))?;
        let name = if self.prefix.is_empty() {
            name.to_string() + "." + picture.suffix
        } else {
            self.prefix.to_string() + "/" + name + "." + picture.suffix
        };
        header.insert(reqwest::header::CONTENT_TYPE, content_type);
        let part = reqwest::multipart::Part::bytes(Cow::Owned(picture.bytes))
            .file_name("user")
            .headers(header);
        let form = reqwest::multipart::Form::new()
            .text("key", name.clone())
            .text("OSSAccessKeyId", self.key_id)
            .text("policy", policy)
            .text("Signature", sign)
            .part("file", part);
        let client = reqwest::Client::new();
        client.post(self.bucket_url).multipart(form).send().await?;
        Ok(format!("{}/{}", self.file_url, name))
    }
}

impl UserService {
    pub(crate) async fn info(user: &CurrentUser, info: UserPutInfo, state: &AppState) -> ApiResult {
        if info.password.is_none() {
            return Err(ApiError::User(tt!("messages.user.reset_password.password")));
        }
        if info.old_password.is_none() {
            return Err(ApiError::User(tt!("messages.user.reset_password.old_password")));
        }
        let password = info.password.unwrap();
        let old_password = info.old_password.unwrap();
        if password.len() < 8 {
            return Err(ApiError::User(tt!("messages.user.reset_password.short")));
        }

        let u = UsersEntity::find_by_id(user.id).one(&state.db).await?;

        match u {
            None => return Err(ApiError::User(tt!("messages.user.info.not_found"))),
            Some(u) => {
                let user_manager = UserManager::load_from_config()?;
                if let Some(user_manager) = user_manager {
                    user_manager
                        .change_password(u.u_id, password.as_str(), old_password.as_str())
                        .await?;
                }
                if !PasswordHash::check_password(old_password.as_str(), u.password.as_str()) {
                    return Err(ApiError::User("password error".into()));
                }
                let new_password = PasswordHash::gen_password(password.as_str());
                let mut m = u.into_active_model();
                m.password = ActiveValue::Set(new_password);
                m.update(&state.db).await?;
            }
        }

        Ok(())
    }

    pub(crate) async fn get_info(user: &CurrentUser, state: &AppState) -> ApiResult<UserRespInfo> {
        let user = UsersEntity::find_by_id(user.id).one(&state.db).await?;
        user.map(Into::into).ok_or(ApiError::User(tt!("messages.user.info.not_found")))
    }

    pub(crate) async fn picture<C: ConnectionTrait>(
        user: &CurrentUser,
        picture: Picture<'_>,
        conn: &C,
    ) -> ApiResult<String> {
        let s = user.id.to_string() + &Rand::string(8);
        let name = save_picture(picture, s).await?;
        let user = UsersEntity::find_by_id(user.id).one(conn).await?.ok_or_else(|| {
            warn!("user id {}, not found", user.id);
            ApiError::User("invalid user".into())
        })?;
        let mut model = user.into_active_model();
        model.picture = ActiveValue::Set(name.clone());
        model.update(conn).await?;
        Ok(name)
    }
}

pub async fn save_picture(picture: Picture<'_>, s: String) -> ApiResult<String> {
    let oss = AliyunOSS::load_from_config();
    match oss {
        Some(oss) => {
            let name = oss.save_picture(&s, picture).await?;
            Ok(name)
        }
        None => {
            let s = "assets/".to_string() + &s + "." + picture.suffix;
            let path = std::path::Path::new("./assets");
            if !path.is_dir() {
                fs::create_dir(path).or(Err(ApiError::User(tt!("messages.user.picture.write"))))?;
            }
            fs::write(&s, picture.bytes)
                .or(Err(ApiError::User(tt!("messages.user.picture.write"))))?;
            let config = load_config();
            Ok(format!("{}/{}", config.api.web_url, s))
        }
    }
}
