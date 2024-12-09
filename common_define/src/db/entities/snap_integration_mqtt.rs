//! `SeaORM` Entity, @generated by sea-orm-codegen 1.0.1

use sea_orm::entity::prelude::*;
use crate::Id;
use crate::time::Timestamp;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "snap_integration_mqtt")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Id,
    pub user_id: Id,
    pub share: Id,
    #[sea_orm(column_type = "Text")]
    pub share_type: String,
    #[sea_orm(column_type = "Text")]
    pub name: String,
    pub enable: bool,
    #[sea_orm(column_type = "Text")]
    pub token: String,
    pub create_time: Timestamp,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
}

impl ActiveModelBehavior for ActiveModel {}
