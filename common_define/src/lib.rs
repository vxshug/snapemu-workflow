#[macro_use]
pub mod event;
mod client_id;
pub mod db;
pub mod decode;
mod id;
mod key;
pub mod lora;
pub mod lorawan_bridge;
pub mod product;
pub mod time;
mod user;

pub use key::last_device_data_key;

pub use client_id::ClientId;
pub use id::Id;

#[macro_export]
macro_rules! sea_string_type {
    ($ident:ident) => {
        impl std::convert::From<$ident> for sea_orm::Value {
            fn from(source: $ident) -> Self {
                source.as_ref().into()
            }
        }
        impl sea_orm::TryGetable for $ident {
            fn try_get_by<I: sea_orm::ColIdx>(
                res: &sea_orm::QueryResult,
                idx: I,
            ) -> std::result::Result<Self, sea_orm::TryGetError> {
                use std::str::FromStr;
                <String as sea_orm::TryGetable>::try_get_by(res, idx).and_then(|v| {
                    $ident::from_str(&v).map_err(|e| {
                        sea_orm::TryGetError::DbErr(sea_orm::DbErr::Custom(e.to_string()))
                    })
                })
            }
        }
        impl sea_orm::sea_query::ValueType for $ident {
            fn try_from(
                v: sea_orm::Value,
            ) -> std::result::Result<Self, sea_orm::sea_query::ValueTypeErr> {
                use std::str::FromStr;

                <String as sea_orm::sea_query::ValueType>::try_from(v).and_then(|a| {
                    $ident::from_str(&a).map_err(|_| sea_orm::sea_query::ValueTypeErr)
                })
            }
            fn type_name() -> std::string::String {
                stringify!($ident).to_owned()
            }
            fn array_type() -> sea_orm::sea_query::ArrayType {
                sea_orm::sea_query::ArrayType::String
            }
            fn column_type() -> sea_orm::sea_query::ColumnType {
                sea_orm::prelude::ColumnType::Text
            }
        }
    };
}
