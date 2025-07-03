use crate::error::{ApiError, ApiResponseResult};
use crate::AppState;
use axum::extract::{Query, State};
use axum::routing::get;
use common_define::db::{DeviceGroupColumn, DeviceGroupEntity, UsersEntity};
use common_define::time::Timestamp;
use common_define::Id;
use sea_orm::{EntityTrait, PaginatorTrait, QueryOrder};
use serde::{Deserialize, Serialize};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().route("/", get(get_all_group))
}

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct GroupQuery {
    #[param(value_type = Option<u64>, example = 0, minimum = 0, default = 0)]
    page: Option<u64>,
}

#[derive(Serialize)]
struct GroupPages {
    page: u64,
    count: u64,
    users: Vec<GroupPageItem>,
}

#[derive(Serialize)]
struct GroupPageItem {
    pub id: Id,
    pub name: String,
    pub description: String,
    pub default_group: bool,
    pub owner: Option<String>,
    pub owner_id: Id,
    pub create_time: Timestamp,
}

#[derive(OpenApi)]
#[openapi(
    paths(get_all_group),
    tags((name = "group", description = "Device Group control api")),
)]
pub struct GroupApi;

///
/// Get all group
#[utoipa::path(
    get,
    path = "/group",
    params(
        GroupQuery,
    ),
    responses(
            (status = 0, description = "group page"),
    )
)]
async fn get_all_group(
    State(state): State<AppState>,
    Query(page): Query<GroupQuery>,
) -> ApiResponseResult<GroupPages> {
    let page = page.page.unwrap_or(0);
    let groups_pages = DeviceGroupEntity::find()
        .find_also_related(UsersEntity)
        .order_by_asc(DeviceGroupColumn::Id)
        .paginate(&state.db, 50);
    let page_count = groups_pages.num_pages().await?;
    if page_count < page {
        return Err(ApiError::User(
            format!("page {} is more than max {}", page, page_count).into(),
        ));
    }
    let groups = groups_pages.fetch_page(page).await?;

    let v = groups
        .into_iter()
        .map(|(group, user)| GroupPageItem {
            id: group.id,
            name: group.name,
            description: group.description,
            default_group: group.default_group,
            owner: user.map(|u| u.user_login),
            owner_id: group.owner,
            create_time: group.create_time,
        })
        .collect();
    Ok(GroupPages { page, count: page_count, users: v }.into())
}
