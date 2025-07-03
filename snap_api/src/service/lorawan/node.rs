use crate::error::{ApiError, ApiResult};
use crate::man::DeviceQueryClient;
use crate::service::device::define::{DeviceParameter, LoRaNode};
use crate::service::device::device::ExtraParm;
use crate::service::device::DeviceService;
use crate::utils::Checker;
use crate::{get_current_user, tt, CurrentUser};
use common_define::db::{
    DeviceLoraNodeActiveModel, DeviceLoraNodeColumn, DeviceLoraNodeEntity, DeviceLoraNodeModel,
    Eui, Key, LoRaAddr,
};
use common_define::lora::{LoRaJoinType, LoRaRegion};
use common_define::product::{DeviceType, ProductType};
use common_define::Id;
use device_info::lorawan::NodeInfo;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel,
    QueryFilter,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use tracing::warn;

pub(crate) struct LoRaNodeService;

#[derive(Serialize, Deserialize)]
pub(crate) struct JoinParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) app_skey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) nwk_skey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) dev_addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) app_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) app_eui: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) dev_eui: Option<String>,
}

pub(crate) struct LoraNodeDeviceDefault {
    pub(crate) name: String,
    pub(crate) eui: Eui,
    pub(crate) description: String,
    pub(crate) region: LoRaRegion,
    pub(crate) join_type: LoRaJoinType,
    pub(crate) app_eui: Eui,
    pub(crate) blue_name: Option<String>,
    pub(crate) dev_eui: Eui,
    pub(crate) app_key: Key,
    pub(crate) dev_addr: LoRaAddr,
    pub(crate) nwk_skey: Key,
    pub(crate) app_skey: Key,
    pub(crate) class_b: bool,
    pub(crate) class_c: bool,
    pub(crate) adr: bool,
    pub(crate) rx1_delay: i16,
    pub(crate) des_rx1_delay: i16,

    pub(crate) rx1_dro: i16,
    pub(crate) des_rx1_dro: i16,

    pub(crate) rx2_dr: i16,
    pub(crate) des_rx2_dr: i16,

    pub(crate) rx2_freq: i32,
    pub(crate) des_rx2_freq: i32,

    pub(crate) d_retry: i32,
    pub(crate) c_retry: i32,
    pub(crate) dutycyle: i32,
    pub(crate) product_type: ProductType,
}

impl LoraNodeDeviceDefault {
    async fn with_bluetooth<C: ConnectionTrait>(
        user: &CurrentUser,
        req: ReqLoraNode,
        conn: &C,
    ) -> ApiResult<Self> {
        let blue_param =
            req.blue_parm.ok_or(ApiError::User(tt!("messages.device.lora.blue_param_missing")))?;
        let eui = req.eui;
        let mut this = Self {
            eui,
            name: req.name,
            description: req.description,
            region: req.region,
            join_type: LoRaJoinType::OTAA,
            app_eui: blue_param.app_eui,
            blue_name: req.blue_name,
            dev_eui: blue_param.dev_eui,
            app_key: blue_param.app_key,
            dev_addr: blue_param.dev_addr,
            nwk_skey: blue_param.nwk_skey,
            app_skey: blue_param.app_skey,
            class_b: false,
            class_c: false,
            adr: blue_param.adr == 1,
            rx1_delay: blue_param.rx1_delay as i16,
            des_rx1_delay: blue_param.rx1_delay as i16,
            rx1_dro: 0,
            des_rx1_dro: 0,
            rx2_dr: blue_param.rx2_dr as i16,
            des_rx2_dr: blue_param.rx2_dr as i16,
            rx2_freq: blue_param.rx2_freq,
            des_rx2_freq: blue_param.rx2_freq,
            d_retry: blue_param.retry,
            c_retry: blue_param.retry,
            dutycyle: blue_param.duty_cycle,
            product_type: ProductType::Monitor,
        };
        let join_type = blue_param
            .join_type
            .or(blue_param.jion_type)
            .ok_or(ApiError::User(tt!("messages.device.lora.join_type_missing")))?;

        match join_type {
            0 => {
                this.join_type = LoRaJoinType::ABP;
            }
            1 => {
                this.join_type = LoRaJoinType::OTAA;
            }
            _ => {
                return Err(ApiError::User(tt!("messages.device.lora.join_type_missing")));
            }
        }

        Ok(this)
    }

    #[instrument(skip_all)]
    async fn with_default<C: ConnectionTrait>(req: ReqLoraNode, conn: &C) -> ApiResult<Self> {
        let (rx2_freq, rx2_dr) = match req.region {
            LoRaRegion::EU868 => (8695250, 0_i16),
            LoRaRegion::US915 => (9233000, 8),
            LoRaRegion::CN779 => (7860000, 0),
            LoRaRegion::EU433 => (4346650, 0),
            LoRaRegion::AU915 => (9233000, 8),
            LoRaRegion::CN470 => (5053000, 0),
            LoRaRegion::AS923_1 => (9232000, 2),
            LoRaRegion::AS923_2 => (9232000, 2),
            LoRaRegion::AS923_3 => (9232000, 2),
            LoRaRegion::KR920 => (9219000, 0),
            LoRaRegion::IN865 => (8665500, 2),
            LoRaRegion::RU864 => (8691000, 0),
        };
        let eui = req.eui;
        let (class_b, class_c) = req
            .extra_parm
            .map(|e| (e.class_b.unwrap_or(false), e.class_c.unwrap_or(false)))
            .unwrap_or((false, false));

        let mut this = Self {
            eui,
            name: req.name,
            description: req.description,
            region: req.region,
            join_type: req.join_type,
            app_eui: Eui::new(0),
            blue_name: req.blue_name,
            dev_eui: Eui::new(0),
            app_key: Key::nil(),
            dev_addr: LoRaAddr::new(0),
            nwk_skey: Key::nil(),
            app_skey: Key::nil(),
            class_b,
            class_c,
            adr: true,
            rx1_delay: 5,
            des_rx1_delay: 5,
            rx1_dro: 0,
            des_rx1_dro: 0,
            rx2_dr,
            des_rx2_dr: rx2_dr,
            rx2_freq,
            des_rx2_freq: rx2_freq,
            d_retry: 0,
            c_retry: 0,
            dutycyle: 30,
            product_type: ProductType::Monitor,
        };

        match req.join_type {
            LoRaJoinType::OTAA => {
                this.otaa(
                    req.join_parameter.app_key.as_ref(),
                    req.join_parameter.app_eui.as_ref(),
                    req.join_parameter.dev_eui.as_ref(),
                    conn,
                )
                .await?;
                this.dev_addr = LoRaNodeService::create_addr(conn).await?;
            }
            LoRaJoinType::ABP => {
                this.abp(
                    req.join_parameter.app_skey.as_ref(),
                    req.join_parameter.nwk_skey.as_ref(),
                    req.join_parameter.dev_addr.as_ref(),
                    conn,
                )
                .await?;
                this.dev_eui = eui;
            }
        }

        Ok(this)
    }

    async fn abp<C: ConnectionTrait>(
        &mut self,
        app_skey: Option<&String>,
        nwk_skey: Option<&String>,
        dev_addr: Option<&String>,
        conn: &C,
    ) -> ApiResult {
        let app_skey =
            app_skey.ok_or(ApiError::User(tt!("messages.device.lora.app_skey_missing")))?;
        let nwk_skey =
            nwk_skey.ok_or(ApiError::User(tt!("messages.device.lora.nwk_skey_missing")))?;
        let dev_addr =
            dev_addr.ok_or(ApiError::User(tt!("messages.device.lora.dev_addr_missing")))?;

        if app_skey.len() != 32 || !Checker::hex(app_skey) {
            return Err(ApiError::User(tt!("messages.device.lora.app_skey")));
        }
        if nwk_skey.len() != 32 || !Checker::hex(nwk_skey) {
            return Err(ApiError::User(tt!("messages.device.lora.nwk_skey")));
        }
        if dev_addr.len() != 8 || !Checker::hex(dev_addr) {
            return Err(ApiError::User(tt!("messages.device.lora.dev_addr")));
        }

        let app_skey = app_skey.to_uppercase();
        let nwk_skey = nwk_skey.to_uppercase();
        let dev_addr = dev_addr.to_uppercase();

        let predefine = DeviceQueryClient::query_eui(dev_addr.as_str()).await?;
        if let Some(predefine) = predefine {
            return match predefine.parameter {
                DeviceParameter::Device(predefine_node) => {
                    if predefine_node.join_parameter.app_skey.as_str() != app_skey.as_str()
                        || predefine_node.join_parameter.nwk_skey.as_str() != nwk_skey.as_str()
                    {
                        return Err(ApiError::User(tt!("messages.device.lora.device_already")));
                    }
                    self.fill_predefine(predefine_node);
                    Ok(())
                }
                DeviceParameter::Gate(_) => {
                    Err(ApiError::User(tt!("messages.device.lora.dev_eui_match")))
                }
            };
        }

        self.app_skey =
            app_skey.parse().map_err(|_| ApiError::User(tt!("messages.device.lora.app_key")))?;

        self.nwk_skey =
            nwk_skey.parse().map_err(|_| ApiError::User(tt!("messages.device.lora.nwk_skey")))?;
        self.dev_addr =
            dev_addr.parse().map_err(|_| ApiError::User(tt!("messages.device.lora.dev_addr")))?;
        Ok(())
    }
    fn fill_predefine(&mut self, define: LoRaNode) {
        self.join_type = define.join_type;
        self.blue_name = define.blue_name;

        match define.join_parameter.nwk_skey.as_str().parse() {
            Ok(o) => {
                self.nwk_skey = o;
            }
            Err(e) => {
                let user = get_current_user();
                warn!(user = user.id.to_string(), "nwk_skey {}", e);
            }
        }

        match define.join_parameter.app_skey.as_str().parse() {
            Ok(o) => {
                self.app_skey = o;
            }
            Err(e) => {
                let user = get_current_user();
                warn!(user = user.id.to_string(), "app_skey {}", e);
            }
        }

        match define.join_parameter.dev_addr.as_str().parse() {
            Ok(o) => {
                self.dev_addr = o;
            }
            Err(e) => {
                let user = get_current_user();
                warn!(user = user.id.to_string(), "dev_addr {}", e);
            }
        }

        match define.join_parameter.app_key.as_str().parse() {
            Ok(o) => {
                self.app_key = o;
            }
            Err(e) => {
                let user = get_current_user();
                warn!(user = user.id.to_string(), "app_key {}", e);
            }
        }
        match define.join_parameter.dev_eui.as_str().parse() {
            Ok(o) => {
                self.dev_eui = o;
            }
            Err(e) => {
                let user = get_current_user();
                warn!(user = user.id.to_string(), "dev_eui {}", e);
            }
        }

        match define.join_parameter.app_eui.as_str().parse() {
            Ok(o) => {
                self.app_eui = o;
            }
            Err(e) => {
                let user = get_current_user();
                warn!(user = user.id.to_string(), "app_eui {}", e);
            }
        }

        self.adr = define.join_parameter.adr;
        self.rx2_freq = define.join_parameter.rx2_freq;
        self.rx2_dr = define.join_parameter.rx2_dr;
        self.rx1_delay = define.join_parameter.rx1_delay;
        self.rx1_dro = define.join_parameter.rx1_dro;

        self.des_rx2_freq = define.join_parameter.rx2_freq;
        self.des_rx2_dr = define.join_parameter.rx2_dr;
        self.des_rx1_delay = define.join_parameter.rx1_delay;
        self.des_rx1_dro = define.join_parameter.rx1_dro;
    }

    async fn check_dev_eui(eui: &str, app_key: Option<&str>) -> ApiResult<Eui> {
        if eui.len() != 16 || !Checker::hex(eui) {
            return Err(ApiError::User(tt!("messages.device.lora.dev_eui")));
        }
        let dev_eui: Eui =
            eui.parse().map_err(|_| ApiError::User(tt!("messages.device.lora.dev_eui")))?;
        let predefine = DeviceQueryClient::query_eui(eui).await?;
        if let Some(predefine) = predefine {
            return match predefine.parameter {
                DeviceParameter::Device(predefine_node) => {
                    if let Some(app_key) = app_key {
                        if predefine_node.join_parameter.app_key.as_str() == app_key {
                            return Ok(dev_eui);
                        }
                    }
                    return Err(ApiError::User(tt!("messages.device.lora.device_already")));
                }
                DeviceParameter::Gate(_) => {
                    Err(ApiError::User(tt!("messages.device.lora.dev_eui_match")))
                }
            };
        }
        Ok(dev_eui)
    }

    async fn otaa<C: ConnectionTrait>(
        &mut self,
        app_key: Option<&String>,
        app_eui: Option<&String>,
        dev_eui: Option<&String>,
        conn: &C,
    ) -> ApiResult {
        let app_key = app_key.ok_or(ApiError::User(tt!("messages.device.lora.app_key_missing")))?;
        let app_eui = app_eui.ok_or(ApiError::User(tt!("messages.device.lora.app_eui_missing")))?;
        let dev_eui = dev_eui.ok_or(ApiError::User(tt!("messages.device.lora.dev_eui_missing")))?;

        if app_key.len() != 32 || !Checker::hex(app_key) {
            return Err(ApiError::User(tt!("messages.device.lora.app_key")));
        }

        if app_eui.len() != 16 || !Checker::hex(app_eui) {
            return Err(ApiError::User(tt!("messages.device.lora.app_eui")));
        }
        if dev_eui.len() != 16 || !Checker::hex(dev_eui) {
            return Err(ApiError::User(tt!("messages.device.lora.dev_eui")));
        }

        let app_key = app_key.to_uppercase();
        let app_eui = app_eui.to_uppercase();
        let dev_eui = dev_eui.to_uppercase();

        let predefine = DeviceQueryClient::query_eui(dev_eui.as_str()).await?;
        if let Some(predefine) = predefine {
            return match predefine.parameter {
                DeviceParameter::Device(predefine_node) => {
                    if predefine_node.join_parameter.app_key.as_str() != app_key.as_str()
                        || predefine_node.join_parameter.app_eui.as_str() != app_eui.as_str()
                    {
                        return Err(ApiError::User(tt!("messages.device.lora.device_already")));
                    }
                    self.fill_predefine(predefine_node);
                    Ok(())
                }
                DeviceParameter::Gate(_) => {
                    Err(ApiError::User(tt!("messages.device.lora.dev_eui_match")))
                }
            };
        }
        self.dev_eui =
            dev_eui.parse().map_err(|_| ApiError::User(tt!("messages.device.lora.dev_eui")))?;

        self.app_eui =
            app_eui.parse().map_err(|_| ApiError::User(tt!("messages.device.lora.app_eui")))?;
        self.app_key =
            app_key.parse().map_err(|_| ApiError::User(tt!("messages.device.lora.app_key")))?;
        Ok(())
    }
}
#[derive(Deserialize)]
pub(crate) struct ReqLoraNode {
    pub(crate) device: Option<Id>,
    pub(crate) name: String,
    pub(crate) eui: Eui,
    pub(crate) description: String,
    pub(crate) region: LoRaRegion,
    pub(crate) join_type: LoRaJoinType,
    pub(crate) join_parameter: JoinParam,
    pub(crate) scan_eui: Option<String>,
    pub(crate) blue_name: Option<String>,
    pub(crate) blue_parm: Option<crate::service::device::device::BluetoothNode>,
    pub(crate) extra_parm: Option<ExtraParm>,
}

#[derive(Deserialize)]
pub(crate) struct BluetoothNode {
    product: String,
    region: LoRaRegion,
    join_type: Option<i32>,
    jion_type: Option<i32>,
    class: String,
    #[serde(rename = "ADR")]
    adr: i32,
    #[serde(rename = "DR")]
    dr: i32,
    #[serde(rename = "Confirmed")]
    confirmed: i32,
    #[serde(rename = "DutyCycle")]
    duty_cycle: i32,
    #[serde(rename = "Power")]
    power: i32,
    #[serde(rename = "Retry")]
    retry: i32,
    #[serde(rename = "Channel")]
    channel: Option<String>,
    dev_eui: Option<String>,
    app_eui: Option<String>,
    app_key: Option<String>,
    dev_addr: Option<String>,
    nwk_skey: Option<String>,
    app_skey: Option<String>,
    #[serde(rename = "RX1DELAY")]
    rx1_delay: i32,
    #[serde(rename = "RX2DELAY")]
    rx2_delay: i32,
    #[serde(rename = "RX2DR_TYPE")]
    rx2_dr_type: i32,
    #[serde(rename = "RX2FREQ_TYPE")]
    rx2_freq_type: i32,
    #[serde(rename = "RX2DR")]
    rx2_dr: i32,
    #[serde(rename = "RX2FREQ")]
    rx2_freq: i32,
    #[serde(rename = "Firmware")]
    firmware: String,
}

impl LoRaNodeService {
    pub(crate) async fn create_addr<C: ConnectionTrait>(conn: &C) -> ApiResult<LoRaAddr> {
        loop {
            let addr = LoRaAddr::random();
            if !Self::valid_addr(addr, conn).await? {
                return Ok(addr);
            }
        }
    }

    pub(crate) async fn valid_addr<C: ConnectionTrait>(
        dev_addr: LoRaAddr,
        conn: &C,
    ) -> ApiResult<bool> {
        let addr = dev_addr.to_string();
        let addr = addr.as_str();
        let res = DeviceLoraNodeEntity::find()
            .filter(DeviceLoraNodeColumn::DevAddr.eq(dev_addr))
            .one(conn)
            .await?;

        if res.is_some() {
            return Err(ApiError::User(tt!("messages.device.lora.dev_addr_already", addr = addr)));
        }
        let r = DeviceQueryClient::query_eui(addr).await?;
        Ok(r.is_some())
    }

    pub(crate) async fn create_dev_eui<C: ConnectionTrait>(conn: &C) -> ApiResult<Eui> {
        loop {
            let eui = Eui::random();
            if !Self::valid_dev_eui(eui, conn).await? {
                return Ok(eui);
            }
        }
    }
    pub(crate) async fn valid_dev_eui<C: ConnectionTrait>(
        dev_eui: Eui,
        conn: &C,
    ) -> ApiResult<bool> {
        let eui = dev_eui.to_string();
        let eui = eui.as_str();
        let res = DeviceLoraNodeEntity::find()
            .filter(DeviceLoraNodeColumn::DevEui.eq(dev_eui))
            .one(conn)
            .await?;

        if res.is_some() {
            return Err(ApiError::User(tt!("messages.device.lora.dev_addr_already", addr = eui)));
        }
        let r = DeviceQueryClient::query_eui(eui).await?;
        Ok(r.is_some())
    }
    pub(crate) async fn update_blue<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        req: ReqLoraNode,
        user: &CurrentUser,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult<Id> {
        match req.device {
            None => {
                DeviceService::valid_eui(req.eui, conn).await?;
                let node = LoraNodeDeviceDefault::with_bluetooth(user, req, conn).await?;
                Self::insert_node(node, user, redis, conn).await
            }
            Some(device_id) => {
                let device = DeviceService::query_one_with_auth(user.id, device_id, conn).await?;
                let node = DeviceLoraNodeEntity::find()
                    .filter(DeviceLoraNodeColumn::DeviceId.eq(device_id))
                    .one(conn)
                    .await?;
                match node {
                    None => Err(ApiError::User(tt!(
                        "messages.device.common.not_found_device",
                        device_id = device_id
                    ))),
                    Some(mut node) => {
                        if let Some(blue) = req.blue_parm {
                            let otaa = blue.join_type.or(blue.jion_type).ok_or(ApiError::User(
                                tt!("messages.device.lora.join_type_missing"),
                            ))? == 1;

                            let join_type =
                                if otaa { LoRaJoinType::OTAA } else { LoRaJoinType::ABP };

                            let mut active_model = node.clone().into_active_model();
                            if node.dev_eui != blue.dev_eui {
                                let other = DeviceLoraNodeEntity::find()
                                    .filter(DeviceLoraNodeColumn::DevEui.eq(blue.dev_eui))
                                    .one(conn)
                                    .await?;
                                if other.is_some() {
                                    return Err(ApiError::User(tt!(
                                        "messages.device.lora.dev_eui_already",
                                        eui = blue.dev_eui
                                    )));
                                }
                                Self::valid_dev_eui(blue.dev_eui, conn).await?;
                                active_model.dev_eui = ActiveValue::Set(blue.dev_eui);
                            }

                            if node.app_eui != blue.app_eui {
                                active_model.app_eui = ActiveValue::Set(blue.app_eui);
                            }
                            if node.app_key != blue.app_key {
                                active_model.app_key = ActiveValue::Set(blue.app_key);
                            }

                            if node.dev_addr != blue.dev_addr {
                                let other = DeviceLoraNodeEntity::find()
                                    .filter(DeviceLoraNodeColumn::DevAddr.eq(blue.dev_addr))
                                    .one(conn)
                                    .await?;
                                if other.is_some() {
                                    return Err(ApiError::User(tt!(
                                        "messages.device.lora.dev_addr_already",
                                        addr = blue.dev_addr
                                    )));
                                }
                                Self::valid_addr(blue.dev_addr, conn).await?;
                                active_model.dev_addr = ActiveValue::Set(blue.dev_addr);
                                active_model.nwk_skey = ActiveValue::Set(blue.nwk_skey);
                                active_model.app_skey = ActiveValue::Set(blue.app_skey);
                                active_model.rx1_delay = ActiveValue::Set(blue.rx1_delay as i16);
                            }
                            if node.join_type != join_type {
                                active_model.join_type = ActiveValue::Set(join_type);
                            }
                            if active_model.is_changed() {
                                node = active_model.update(conn).await?;
                            }

                            NodeInfo::unregister(node.dev_eui, node.dev_addr, redis).await?;
                            NodeInfo::register_to_redis(node, device.device, redis).await?;
                        }
                        Ok(device_id)
                    }
                }
            }
        }
    }

    #[instrument(skip_all)]
    pub(crate) async fn create_node<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        req: ReqLoraNode,
        user: &CurrentUser,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult<Id> {
        if req.blue_parm.is_some() {
            return Self::update_blue(req, user, redis, conn).await;
        }
        let node = LoraNodeDeviceDefault::with_default(req, conn).await?;
        Self::insert_node(node, user, redis, conn).await
    }
    #[instrument(skip_all)]
    async fn insert_node<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        node: LoraNodeDeviceDefault,
        user: &CurrentUser,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult<Id> {
        match node.join_type {
            LoRaJoinType::OTAA => {
                if NodeInfo::check_eui(node.dev_eui, redis).await? {
                    return Err(ApiError::User(tt!(
                        "messages.device.lora.dev_eui_already",
                        eui = node.dev_eui
                    )));
                }
                let res = DeviceLoraNodeEntity::find()
                    .filter(DeviceLoraNodeColumn::DevEui.eq(node.dev_eui))
                    .one(conn)
                    .await?;
                if res.is_some() {
                    return Err(ApiError::User(tt!(
                        "messages.device.lora.dev_eui_already",
                        eui = node.dev_eui
                    )));
                }
            }
            LoRaJoinType::ABP => {
                if NodeInfo::check_addr(node.dev_addr, redis).await? {
                    return Err(ApiError::User(tt!(
                        "messages.device.lora.dev_addr_already",
                        addr = node.dev_addr
                    )));
                }
                let res = DeviceLoraNodeEntity::find()
                    .filter(DeviceLoraNodeColumn::DevAddr.eq(node.dev_addr))
                    .one(conn)
                    .await?;
                if res.is_some() {
                    return Err(ApiError::User(tt!(
                        "messages.device.lora.dev_addr_already",
                        addr = node.dev_addr
                    )));
                }
            }
        }

        let device = DeviceService::register_device(
            user,
            node.eui,
            node.name.as_str(),
            node.description.as_str(),
            DeviceType::LoRaNode,
            conn,
        )
        .await?;

        let model = DeviceLoraNodeActiveModel {
            id: Default::default(),
            device_id: ActiveValue::Set(device.id),
            region: ActiveValue::Set(node.region),
            join_type: ActiveValue::Set(node.join_type),
            app_eui: ActiveValue::Set(node.app_eui),
            dev_eui: ActiveValue::Set(node.dev_eui),
            app_key: ActiveValue::Set(node.app_key),
            dev_addr: ActiveValue::Set(node.dev_addr),
            nwk_skey: ActiveValue::Set(node.nwk_skey),
            app_skey: ActiveValue::Set(node.app_skey),
            class_b: ActiveValue::Set(node.class_b),
            class_c: ActiveValue::Set(node.class_c),
            adr: ActiveValue::Set(node.adr),
            rx1_delay: ActiveValue::Set(node.rx1_delay),
            des_rx1_delay: ActiveValue::Set(node.des_rx1_delay),
            rx1_dro: ActiveValue::Set(node.rx1_dro),
            des_rx1_dro: ActiveValue::Set(node.des_rx1_dro),
            rx2_dr: ActiveValue::Set(node.rx2_dr),
            des_rx2_dr: ActiveValue::Set(node.des_rx2_dr),
            rx2_freq: ActiveValue::Set(node.rx2_freq),
            des_rx2_freq: ActiveValue::Set(node.des_rx2_freq),
            d_retry: ActiveValue::Set(node.d_retry as i16),
            c_retry: ActiveValue::Set(node.c_retry as i16),
            product_type: ActiveValue::Set(node.product_type),
            dutycyle: ActiveValue::Set(node.dutycyle),
            up_confirm: ActiveValue::Set(false),
            up_dr: ActiveValue::Set(0),
            power: ActiveValue::Set(0),
            battery: Default::default(),
            charge: ActiveValue::Set(false),
            time_zone: ActiveValue::Set(0),
            firmware: ActiveValue::Set(0),
            dev_non: ActiveValue::Set(0),
            app_non: ActiveValue::Set(0),
            net_id: ActiveValue::Set(0),
        };
        let model = model.insert(conn).await?;
        if let Some(blue_name) = node.blue_name {
            DeviceService::new_func_blue(device.id, blue_name.as_str(), conn).await?;
        }
        let device_id = device.id;
        NodeInfo::register_to_redis(model, device, redis).await?;

        Ok(device_id)
    }

    #[instrument(skip_all)]
    pub(crate) async fn delete_node<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        device_id: Id,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult {
        let node = DeviceLoraNodeEntity::find()
            .filter(DeviceLoraNodeColumn::DeviceId.eq(device_id))
            .one(conn)
            .await?;

        match node {
            Some(node) => {
                NodeInfo::unregister(node.dev_eui, node.dev_addr, redis).await?;
                node.into_active_model().delete(conn).await?;
                Ok(())
            }
            None => {
                Err(ApiError::Device { device_id, msg: tt!("messages.device.lora.device_missing") })
            }
        }
    }

    #[instrument(skip_all)]
    pub(crate) async fn get_lora_node<C: ConnectionTrait>(
        device_id: Id,
        conn: &C,
    ) -> ApiResult<DeviceLoraNodeModel> {
        DeviceLoraNodeEntity::find()
            .filter(DeviceLoraNodeColumn::DeviceId.eq(device_id))
            .one(conn)
            .await?
            .ok_or(ApiError::Device {
                device_id,
                msg: tt!("messages.device.common.device_missing", device_id = device_id),
            })
    }

    #[instrument(skip(conn))]
    pub(crate) async fn get_all_lora_node<C: ConnectionTrait>(
        id: Vec<Id>,
        conn: &C,
    ) -> ApiResult<Vec<DeviceLoraNodeModel>> {
        if id.is_empty() {
            return Ok(Vec::new());
        }
        DeviceLoraNodeEntity::find()
            .filter(DeviceLoraNodeColumn::DeviceId.is_in(id))
            .all(conn)
            .await
            .map_err(Into::into)
    }
}
