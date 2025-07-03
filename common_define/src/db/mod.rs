mod addr;
mod data;
mod entities;
mod eui;
mod group_permission;
mod key;
mod map;

pub use addr::LoRaAddr;
pub use data::DbDecodeData;
pub use group_permission::GroupPermission;
pub use key::Key;

pub use eui::Eui;
pub use map::CodeMapItem;
pub use map::CustomDecodeMap;
pub use map::CustomMapItem;
pub use map::DecodeMap;

#[derive(thiserror::Error, Debug)]
pub enum DbErr {
    #[error("Parse")]
    Parse,
    #[error("{0}")]
    Len(String),
    #[error("Max is 80000000")]
    Max,
}

pub use data::DataError;

pub use entities::snap_device_data_name::ActiveModel as SnapDeviceDataNameActiveModel;
pub use entities::snap_device_data_name::Column as SnapDeviceDataNameColumn;
pub use entities::snap_device_data_name::Entity as SnapDeviceDataNameEntity;
pub use entities::snap_device_data_name::Model as SnapDeviceDataNameModel;

pub use entities::snap_product_info::ActiveModel as SnapProductInfoActiveModel;
pub use entities::snap_product_info::Column as SnapProductInfoColumn;
pub use entities::snap_product_info::Entity as SnapProductInfoEntity;
pub use entities::snap_product_info::Model as SnapProductInfoModel;

pub use entities::snap_downlink::ActiveModel as SnapDownLinkActiveModel;
pub use entities::snap_downlink::Column as SnapDownLinkColumn;
pub use entities::snap_downlink::Entity as SnapDownLinkEntity;
pub use entities::snap_downlink::Model as SnapDownLinkModel;

pub use entities::snap_config::ActiveModel as SnapConfigActiveModel;
pub use entities::snap_config::Column as SnapConfigColumn;
pub use entities::snap_config::Entity as SnapConfigEntity;
pub use entities::snap_config::Model as SnapConfigModel;

pub use entities::snap_admin::ActiveModel as SnapAdminActiveModel;
pub use entities::snap_admin::Column as SnapAdminColumn;
pub use entities::snap_admin::Entity as SnapAdminEntity;
pub use entities::snap_admin::Model as SnapAdminModel;

pub use entities::snap_device_group_map_user::ActiveModel as SnapDeviceGroupMapUserActiveModel;
pub use entities::snap_device_group_map_user::Column as SnapDeviceGroupMapUserColumn;
pub use entities::snap_device_group_map_user::Entity as SnapDeviceGroupMapUserEntity;
pub use entities::snap_device_group_map_user::Model as SnapDeviceGroupMapUserModel;

pub use entities::snap_decode_script::ActiveModel as DecodeScriptActiveModel;
pub use entities::snap_decode_script::Column as DecodeScriptColumn;
pub use entities::snap_decode_script::Entity as DecodeScriptEntity;
pub use entities::snap_decode_script::Model as DecodeScriptModel;
pub use entities::snap_device_authority::ActiveModel as DeviceAuthorityActiveModel;
pub use entities::snap_device_authority::Column as DeviceAuthorityColumn;
pub use entities::snap_device_authority::Entity as DeviceAuthorityEntity;
pub use entities::snap_device_authority::Model as DeviceAuthorityModel;
pub use entities::snap_device_data::ActiveModel as DeviceDataActiveModel;
pub use entities::snap_device_data::Column as DeviceDataColumn;
pub use entities::snap_device_data::Entity as DeviceDataEntity;
pub use entities::snap_device_data::Model as DeviceDataModel;
pub use entities::snap_device_data_name::ActiveModel as DeviceDataNameActiveModel;
pub use entities::snap_device_data_name::Column as DeviceDataNameColumn;
pub use entities::snap_device_data_name::Entity as DeviceDataNameEntity;
pub use entities::snap_device_data_name::Model as DeviceDataNameModel;
pub use entities::snap_device_function::ActiveModel as DeviceFunctionActiveModel;
pub use entities::snap_device_function::Column as DeviceFunctionColumn;
pub use entities::snap_device_function::Entity as DeviceFunctionEntity;
pub use entities::snap_device_function::Model as DeviceFunctionModel;
pub use entities::snap_device_group::ActiveModel as DeviceGroupActiveModel;
pub use entities::snap_device_group::Column as DeviceGroupColumn;
pub use entities::snap_device_group::Entity as DeviceGroupEntity;
pub use entities::snap_device_group::Model as DeviceGroupModel;
pub use entities::snap_device_lora_gate::ActiveModel as DeviceLoraGateActiveModel;
pub use entities::snap_device_lora_gate::Column as DeviceLoraGateColumn;
pub use entities::snap_device_lora_gate::Entity as DeviceLoraGateEntity;
pub use entities::snap_device_lora_gate::Model as DeviceLoraGateModel;
pub use entities::snap_device_lora_node::ActiveModel as DeviceLoraNodeActiveModel;
pub use entities::snap_device_lora_node::Column as DeviceLoraNodeColumn;
pub use entities::snap_device_lora_node::Entity as DeviceLoraNodeEntity;
pub use entities::snap_device_lora_node::Model as DeviceLoraNodeModel;
pub use entities::snap_device_map_group::ActiveModel as DeviceMapGroupActiveModel;
pub use entities::snap_device_map_group::Column as DeviceMapGroupColumn;
pub use entities::snap_device_map_group::Entity as DeviceMapGroupEntity;
pub use entities::snap_device_map_group::Model as DeviceMapGroupModel;
pub use entities::snap_device_mqtt::ActiveModel as DeviceMqttActiveModel;
pub use entities::snap_device_mqtt::Column as DeviceMqttColumn;
pub use entities::snap_device_mqtt::Entity as DeviceMqttEntity;
pub use entities::snap_device_mqtt::Model as DeviceMqttModel;
pub use entities::snap_devices::ActiveModel as DevicesActiveModel;
pub use entities::snap_devices::Column as DevicesColumn;
pub use entities::snap_devices::Entity as DevicesEntity;
pub use entities::snap_devices::Model as DevicesModel;
pub use entities::snap_integration_mqtt::ActiveModel as SnapIntegrationMqttActiveModel;
pub use entities::snap_integration_mqtt::Column as SnapIntegrationMqttColumn;
pub use entities::snap_integration_mqtt::Entity as SnapIntegrationMqttEntity;
pub use entities::snap_integration_mqtt::Model as SnapIntegrationMqttModel;
pub use entities::snap_snap_device::ActiveModel as SnapDeviceActiveModel;
pub use entities::snap_snap_device::Column as SnapDeviceColumn;
pub use entities::snap_snap_device::Entity as SnapDeviceEntity;
pub use entities::snap_snap_device::Model as SnapDeviceModel;
pub use entities::snap_user_token::ActiveModel as UserTokenActiveModel;
pub use entities::snap_user_token::Column as UserTokenColumn;
pub use entities::snap_user_token::Entity as UserTokenEntity;
pub use entities::snap_user_token::Model as UserTokenModel;
pub use entities::snap_users::ActiveModel as UsersActiveModel;
pub use entities::snap_users::Column as UsersColumn;
pub use entities::snap_users::Entity as UsersEntity;
pub use entities::snap_users::Model as UsersModel;
pub use entities::snap_users_tripartite::ActiveModel as UsersTripartiteActiveModel;
pub use entities::snap_users_tripartite::Column as UsersTripartiteColumn;
pub use entities::snap_users_tripartite::Entity as UsersTripartiteEntity;
pub use entities::snap_users_tripartite::Model as UsersTripartiteModel;
