use crate::error::{ApiError, ApiResult};
use crate::service::device::group::DeviceGroupService;
use crate::{tt, CurrentUser};
use common_define::db::{DeviceMapGroupColumn, DeviceMapGroupEntity, DeviceMapGroupModel};
use common_define::Id;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel,
    QueryFilter, QueryOrder,
};
use tracing::instrument;

pub(crate) struct DeviceOrderService;

impl DeviceOrderService {
    pub(crate) async fn device_top<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        user: &CurrentUser,
        device_id: Id,
        group_id: Option<Id>,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult {
        match group_id {
            None => Self::device_default_group(user, device_id, redis, conn).await,
            Some(group_id) => Self::device_group(user, device_id, group_id, redis, conn).await,
        }
    }
    #[instrument(skip(conn))]
    pub(crate) async fn device_order_group<C: ConnectionTrait>(
        group: Id,
        conn: &C,
    ) -> ApiResult<Vec<DeviceMapGroupModel>> {
        DeviceMapGroupEntity::find()
            .filter(DeviceMapGroupColumn::GroupId.eq(group))
            .all(conn)
            .await
            .map(|mut v| {
                v.sort_by(|pre, cur| cur.dev_order.cmp(&pre.dev_order));
                v
            })
            .map_err(Into::into)
    }
    pub(crate) async fn device_default_group<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        user: &CurrentUser,
        device_id: Id,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult {
        let group = DeviceGroupService::query_user_default_group(user.id, redis, conn).await?;
        let dev_order = Self::device_group_max_order(user, group.id, conn).await?;
        let order = DeviceMapGroupEntity::find()
            .filter(
                DeviceMapGroupColumn::GroupId
                    .eq(group.id)
                    .and(DeviceMapGroupColumn::UserId.eq(user.id))
                    .and(DeviceMapGroupColumn::DeviceId.eq(device_id)),
            )
            .one(conn)
            .await?;
        if let Some(order) = order {
            if order.dev_order != dev_order || dev_order == 0 {
                let mut model = order.into_active_model();
                model.dev_order = ActiveValue::Set(dev_order + 1);
                model.update(conn).await?;
            }
        }
        Ok(())
    }

    pub(crate) async fn device_group_max_order<C: ConnectionTrait>(
        user: &CurrentUser,
        group: Id,
        conn: &C,
    ) -> ApiResult<i32> {
        let order = DeviceMapGroupEntity::find()
            .filter(
                DeviceMapGroupColumn::GroupId
                    .eq(group)
                    .and(DeviceMapGroupColumn::UserId.eq(user.id)),
            )
            .order_by_desc(DeviceMapGroupColumn::DevOrder)
            .one(conn)
            .await?;
        Ok(order.map(|it| it.dev_order).unwrap_or(0))
    }

    pub(crate) async fn device_group<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        user: &CurrentUser,
        device_id: Id,
        group_id: Id,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult {
        let dev_order = Self::device_group_max_order(user, group_id, conn).await?;
        let order = DeviceMapGroupEntity::find()
            .filter(
                DeviceMapGroupColumn::GroupId
                    .eq(group_id)
                    .and(DeviceMapGroupColumn::UserId.eq(user.id))
                    .and(DeviceMapGroupColumn::DeviceId.eq(device_id)),
            )
            .one(conn)
            .await?
            .ok_or(ApiError::User(tt!("messages.device.group.invalid")))?;
        if order.dev_order != dev_order || dev_order == 0 {
            let mut model = order.into_active_model();
            model.dev_order = ActiveValue::Set(dev_order + 1);
            model.update(conn).await?;
        }
        Ok(())
    }
}
