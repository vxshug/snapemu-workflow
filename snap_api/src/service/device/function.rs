use crate::error::ApiResult;
use crate::service::device::DeviceService;
use common_define::db::{DeviceFunctionActiveModel, DeviceFunctionColumn, DeviceFunctionEntity};
use common_define::Id;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};

impl DeviceService {
    pub const FUNC_BLUETOOTH: &'static str = "blue";
    pub(crate) async fn new_func_blue<C: ConnectionTrait>(
        device_id: Id,
        blue_name: &str,
        conn: &C,
    ) -> ApiResult {
        let func = DeviceFunctionEntity::find()
            .filter(
                DeviceFunctionColumn::Device
                    .eq(device_id)
                    .and(DeviceFunctionColumn::FuncName.eq(Self::FUNC_BLUETOOTH)),
            )
            .one(conn)
            .await?;
        match func {
            Some(device_function) => {
                if device_function.func_value == blue_name {
                    return Ok(());
                }
                let mut func: DeviceFunctionActiveModel = device_function.into();
                func.func_value = ActiveValue::Set(blue_name.to_string());
                func.update(conn).await?;
            }
            None => {
                let func = DeviceFunctionActiveModel {
                    id: Default::default(),
                    device: ActiveValue::Set(device_id),
                    func_name: ActiveValue::Set(Self::FUNC_BLUETOOTH.to_string()),
                    func_value: ActiveValue::Set(blue_name.to_string()),
                };
                func.insert(conn).await?;
            }
        }
        Ok(())
    }
}
