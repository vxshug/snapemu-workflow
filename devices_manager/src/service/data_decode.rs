#[derive(Default)]
pub struct DeviceStatus {
    pub battery: Option<u8>,
    pub charge: bool,
}
// async fn decode(payload: &[u8]) -> DeviceResult<(Vec<DecodeData>, DeviceStatus)> {
//
//     let data = decode::up_data_decode(payload)?;
//
//     let mut db_data = Vec::new();
//     let mut status: Option<DeviceBattery> = None;
//
//
//     for datum in data {
//         match datum {
//             db::data::PKDataValue::IO(_io) => {
//             }
//             db::data::PKDataValue::State(b) => {
//                 status = Some(b);
//             }
//             db::data::PKDataValue::Data(data) => {
//                 db_data.push(data);
//             }
//         }
//     }
//
//     Ok((db_data, status))
// }
