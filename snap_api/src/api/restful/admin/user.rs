use crate::api::SnPath;
use crate::error::{ApiError, ApiResponseResult};
use crate::AppState;
use axum::extract::{Query, State};
use axum::routing::get;
use common_define::db::{
    DeviceGroupColumn, DeviceGroupEntity, DevicesEntity, Eui, UsersColumn, UsersEntity,
};
use common_define::product::DeviceType;
use common_define::time::Timestamp;
use common_define::Id;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use tracing::warn;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().route("/", get(get_all_users)).route("/info/:id", get(get_user_info))
}

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct UserQuery {
    #[param(value_type = Option<u64>, example = 0, minimum = 0, default = 0)]
    page: Option<u64>,
}

#[derive(Serialize)]
struct UserPages {
    page: u64,
    count: u64,
    users: Vec<UserPageItem>,
}

#[derive(Serialize)]
struct UserPageItem {
    pub id: Id,
    pub u_id: uuid::Uuid,
    pub user_login: String,
    pub user_nick: String,
    pub email: Option<String>,
    pub active: bool,
    pub active_token: String,
    pub picture: String,
    pub create_time: Timestamp,
}

#[derive(Serialize)]
struct UserInfo {
    pub id: Id,
    pub u_id: uuid::Uuid,
    pub user_login: String,
    pub user_nick: String,
    pub email: Option<String>,
    pub active: bool,
    pub active_token: String,
    pub picture: String,
    pub groups: Vec<UserGroup>,
    pub create_time: Timestamp,
}

#[derive(Serialize)]
struct UserGroup {
    pub id: Id,
    pub name: String,
    pub description: String,
    pub default_group: bool,
    pub devices: Vec<UserDevice>,
    pub create_time: Timestamp,
}

#[derive(Serialize)]
struct UserDevice {
    pub id: Id,
    pub eui: Eui,
    pub name: String,
    pub description: String,
    pub enable: bool,
    pub device_type: DeviceType,
    pub create_time: Timestamp,
}

#[derive(OpenApi)]
#[openapi(
    paths(get_all_users,get_user_info),
    tags((name = "user", description = "User control api")),
)]
pub struct UserApi;

///
/// Get all users
#[utoipa::path(
    get,
    path = "/user",
    params(
        UserQuery,
    ),
    responses(
            (status = 0, description = "user page"),
    )
)]
async fn get_all_users(
    State(state): State<AppState>,
    Query(page): Query<UserQuery>,
) -> ApiResponseResult<UserPages> {
    let page = page.page.unwrap_or(0);
    let users_pages = UsersEntity::find().order_by_asc(UsersColumn::Id).paginate(&state.db, 50);
    let page_count = users_pages.num_pages().await?;
    if page_count < page {
        return Err(ApiError::User(
            format!("page {} is more than max {}", page, page_count).into(),
        ));
    }
    let users = users_pages.fetch_page(page).await?;

    let v = users
        .into_iter()
        .map(|it| UserPageItem {
            id: it.id,
            u_id: it.u_id,
            user_login: it.user_login,
            user_nick: it.user_nick,
            email: it.email,
            active: it.active,
            active_token: it.active_token,
            picture: it.picture,
            create_time: it.create_time,
        })
        .collect();
    Ok(UserPages { page, count: page_count, users: v }.into())
}

///
/// Get User Info
#[utoipa::path(
    get,
    path = "/user/info/{id}",
    params(
        ("id", description = "User id"),
    ),
    responses(
            (status = 0, description = "user page"),
    )
)]
async fn get_user_info(
    State(state): State<AppState>,
    SnPath(user_id): SnPath<Id>,
) -> ApiResponseResult<UserInfo> {
    let user = UsersEntity::find_by_id(user_id).one(&state.db).await?.ok_or_else(|| {
        warn!("Failed to find user with id {}", user_id);
        ApiError::User("User not found".into())
    })?;
    let groups = DeviceGroupEntity::find()
        .filter(DeviceGroupColumn::Owner.eq(user_id))
        .find_with_related(DevicesEntity)
        .all(&state.db)
        .await?;

    let groups = groups
        .into_iter()
        .map(|(group, devices)| {
            let devices = devices
                .into_iter()
                .map(|it| UserDevice {
                    id: it.id,
                    eui: it.eui,
                    name: it.name,
                    description: it.description,
                    enable: it.enable,
                    device_type: it.device_type,
                    create_time: it.create_time,
                })
                .collect();
            UserGroup {
                id: group.id,
                name: group.name,
                description: group.description,
                default_group: group.default_group,
                devices,
                create_time: group.create_time,
            }
        })
        .collect();

    Ok(UserInfo {
        id: user.id,
        u_id: user.u_id,
        user_login: user.user_login,
        user_nick: user.user_nick,
        email: user.email,
        active: user.active,
        active_token: user.active_token,
        picture: user.picture,
        groups,
        create_time: user.create_time,
    }
    .into())
}
