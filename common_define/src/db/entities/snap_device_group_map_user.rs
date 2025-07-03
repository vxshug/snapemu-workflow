use crate::db::GroupPermission;
use crate::time::Timestamp;
use crate::Id;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, serde::Serialize, serde::Deserialize)]
#[sea_orm(table_name = "snap_device_group_map_user")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Id,
    pub creator: Id,
    pub group_id: Id,
    pub user_id: Id,
    pub permission: GroupPermission,
    pub create_time: Timestamp,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
