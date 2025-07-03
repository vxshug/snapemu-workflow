use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        user::up(manager).await?;
        device::up(manager).await?;
        data::up(manager).await?;
        admin::up(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        user::down(manager).await?;
        device::down(manager).await?;
        data::down(manager).await?;
        admin::down(manager).await
    }
}

pub fn big_key_auto<T: IntoIden>(name: T) -> ColumnDef {
    big_integer(name).auto_increment().primary_key().take()
}

mod user {
    use crate::m20240904_020441_create_table::big_key_auto;
    use sea_orm_migration::{prelude::*, schema::*};

    pub async fn up(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SnapUsers::Table)
                    .if_not_exists()
                    .col(big_integer(SnapUsers::Id).auto_increment().primary_key().take())
                    .col(uuid(SnapUsers::UId).unique_key())
                    .col(text(SnapUsers::UserLogin).unique_key())
                    .col(text(SnapUsers::UserNick))
                    .col(text(SnapUsers::Password))
                    .col(text_null(SnapUsers::Email))
                    .index(Index::create().unique().name("user-email-idx").col(SnapUsers::Email))
                    .col(boolean(SnapUsers::Active).default(false))
                    .col(text(SnapUsers::ActiveToken).default(""))
                    .col(text(SnapUsers::Picture).default(""))
                    .col(
                        timestamp_with_time_zone(SnapUsers::CreateTime)
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(SnapUsersTripartite::Table)
                    .if_not_exists()
                    .col(big_key_auto(SnapUsersTripartite::Id))
                    .col(big_integer(SnapUsersTripartite::UserId))
                    .col(text(SnapUsersTripartite::UniqueId))
                    .col(text(SnapUsersTripartite::Platform))
                    .col(
                        timestamp_with_time_zone(SnapUsersTripartite::CreateTime)
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(SnapUserToken::Table)
                    .if_not_exists()
                    .col(big_key_auto(SnapUserToken::Id))
                    .col(big_integer(SnapUserToken::UserId))
                    .col(text(SnapUserToken::Token))
                    .col(text(SnapUserToken::TokenType))
                    .col(boolean(SnapUserToken::Enable))
                    .col(timestamp_with_time_zone(SnapUserToken::ExpiresTime))
                    .col(
                        timestamp_with_time_zone(SnapUserToken::CreateTime)
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }
    pub async fn down(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(SnapUsers::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SnapUsersTripartite::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SnapUserToken::Table).to_owned()).await
    }

    #[derive(DeriveIden)]
    enum SnapUsers {
        Table,
        Id,
        UId,
        UserLogin,
        UserNick,
        Password,
        Email,
        Active,
        ActiveToken,
        Picture,
        CreateTime,
    }

    #[derive(DeriveIden)]
    enum SnapUsersTripartite {
        Table,
        Id,
        UserId,
        UniqueId,
        Platform,
        CreateTime,
    }

    #[derive(DeriveIden)]
    enum SnapUserToken {
        Table,
        Id,
        UserId,
        Token,
        TokenType,
        Enable,
        ExpiresTime,
        CreateTime,
    }
}

mod device {
    use crate::m20240904_020441_create_table::big_key_auto;
    use sea_orm_migration::{prelude::*, schema::*};

    pub async fn up(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SnapDeviceGroup::Table)
                    .if_not_exists()
                    .col(big_key_auto(SnapDeviceGroup::Id))
                    .col(text(SnapDeviceGroup::Name))
                    .col(text(SnapDeviceGroup::Description))
                    .col(boolean(SnapDeviceGroup::DefaultGroup))
                    .col(big_integer(SnapDeviceGroup::Owner))
                    .col(
                        timestamp_with_time_zone(SnapDeviceGroup::CreateTime)
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(SnapDeviceFunction::Table)
                    .if_not_exists()
                    .col(big_key_auto(SnapDeviceFunction::Id))
                    .col(integer(SnapDeviceFunction::Device))
                    .col(text(SnapDeviceFunction::FuncName))
                    .col(text(SnapDeviceFunction::FuncValue))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("func-device-idx")
                    .table(SnapDeviceFunction::Table)
                    .col(SnapDeviceFunction::Device)
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(SnapProductInfo::Table)
                    .if_not_exists()
                    .col(big_key_auto(SnapProductInfo::Id))
                    .col(text(SnapProductInfo::Sku))
                    .col(text(SnapProductInfo::Name))
                    .col(text(SnapProductInfo::Description))
                    .col(text(SnapProductInfo::Image))
                    .col(
                        timestamp_with_time_zone(SnapProductInfo::CreateTime)
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(SnapDevices::Table)
                    .if_not_exists()
                    .col(big_key_auto(SnapDevices::Id))
                    .col(text(SnapDevices::Eui).unique_key())
                    .col(text(SnapDevices::Name))
                    .col(text(SnapDevices::Description))
                    .col(big_integer(SnapDevices::Creator))
                    .col(boolean(SnapDevices::Enable))
                    .col(integer(SnapDevices::Period))
                    .col(big_integer_null(SnapDevices::Script))
                    .col(big_integer_null(SnapDevices::DataId))
                    .col(big_integer_null(SnapDevices::ProductId))
                    .col(text(SnapDevices::DeviceType))
                    .col(timestamp_with_time_zone_null(SnapDevices::ActiveTime))
                    .col(
                        timestamp_with_time_zone(SnapDevices::CreateTime)
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(SnapIntegrationMqtt::Table)
                    .if_not_exists()
                    .col(big_key_auto(SnapIntegrationMqtt::Id))
                    .col(big_integer(SnapIntegrationMqtt::UserId))
                    .col(text(SnapIntegrationMqtt::MqttType))
                    .col(text(SnapIntegrationMqtt::Name))
                    .col(text(SnapIntegrationMqtt::Username))
                    .col(text(SnapIntegrationMqtt::Password))
                    .col(boolean(SnapIntegrationMqtt::Enable))
                    .col(
                        timestamp_with_time_zone(SnapIntegrationMqtt::CreateTime)
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(SnapDeviceMapGroup::Table)
                    .if_not_exists()
                    .col(big_key_auto(SnapDeviceMapGroup::Id))
                    .col(big_integer(SnapDeviceMapGroup::UserId))
                    .col(big_integer(SnapDeviceMapGroup::DeviceId))
                    .col(big_integer(SnapDeviceMapGroup::GroupId))
                    .col(integer(SnapDeviceMapGroup::DevOrder))
                    .col(
                        timestamp_with_time_zone(SnapDeviceMapGroup::CreateTime)
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("device-user-map-group-idx")
                    .table(SnapDeviceMapGroup::Table)
                    .col(SnapDeviceMapGroup::UserId)
                    .col(SnapDeviceMapGroup::DeviceId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("group-user-map-device-idx")
                    .table(SnapDeviceMapGroup::Table)
                    .col(SnapDeviceMapGroup::UserId)
                    .col(SnapDeviceMapGroup::GroupId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("device-map-group-idx")
                    .table(SnapDeviceMapGroup::Table)
                    .col(SnapDeviceMapGroup::DeviceId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("group-map-device-idx")
                    .table(SnapDeviceMapGroup::Table)
                    .col(SnapDeviceMapGroup::GroupId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(SnapDeviceLoraGate::Table)
                    .if_not_exists()
                    .col(big_key_auto(SnapDeviceLoraGate::Id))
                    .col(big_integer(SnapDeviceLoraGate::DeviceId).unique_key())
                    .col(text(SnapDeviceLoraGate::Region))
                    .col(text(SnapDeviceLoraGate::Eui))
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(SnapDeviceMQTT::Table)
                    .if_not_exists()
                    .col(big_key_auto(SnapDeviceMQTT::Id))
                    .col(big_integer(SnapDeviceMQTT::DeviceId).unique_key())
                    .col(text(SnapDeviceMQTT::Eui))
                    .col(text(SnapDeviceMQTT::Username))
                    .col(text(SnapDeviceMQTT::Password))
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(SnapDeviceLoraNode::Table)
                    .if_not_exists()
                    .col(big_key_auto(SnapDeviceLoraNode::Id))
                    .col(big_integer(SnapDeviceLoraNode::DeviceId).unique_key())
                    .col(text(SnapDeviceLoraNode::Region))
                    .col(text(SnapDeviceLoraNode::JoinType))
                    .col(text(SnapDeviceLoraNode::AppEui))
                    .col(text(SnapDeviceLoraNode::DevEui).unique_key())
                    .col(text(SnapDeviceLoraNode::AppKey))
                    .col(text(SnapDeviceLoraNode::DevAddr).unique_key())
                    .col(text(SnapDeviceLoraNode::NwkSkey))
                    .col(text(SnapDeviceLoraNode::AppSkey))
                    .col(boolean(SnapDeviceLoraNode::ClassB))
                    .col(boolean(SnapDeviceLoraNode::ClassC))
                    .col(boolean(SnapDeviceLoraNode::Adr))
                    .col(small_integer(SnapDeviceLoraNode::Rx1Delay))
                    .col(small_integer(SnapDeviceLoraNode::DesRx1Delay))
                    .col(small_integer(SnapDeviceLoraNode::Rx1Dro))
                    .col(small_integer(SnapDeviceLoraNode::DesRx1Dro))
                    .col(small_integer(SnapDeviceLoraNode::Rx2Dr))
                    .col(small_integer(SnapDeviceLoraNode::DesRx2Dr))
                    .col(integer(SnapDeviceLoraNode::Rx2Freq))
                    .col(integer(SnapDeviceLoraNode::DesRx2Freq))
                    .col(small_integer(SnapDeviceLoraNode::DRetry))
                    .col(small_integer(SnapDeviceLoraNode::CRetry))
                    .col(text(SnapDeviceLoraNode::ProductType))
                    .col(integer(SnapDeviceLoraNode::Dutycyle))
                    .col(boolean(SnapDeviceLoraNode::UpConfirm))
                    .col(small_integer(SnapDeviceLoraNode::UpDr))
                    .col(small_integer(SnapDeviceLoraNode::Power))
                    .col(small_integer_null(SnapDeviceLoraNode::Battery))
                    .col(boolean(SnapDeviceLoraNode::Charge))
                    .col(integer(SnapDeviceLoraNode::TimeZone))
                    .col(integer(SnapDeviceLoraNode::Firmware))
                    .col(integer(SnapDeviceLoraNode::DevNon))
                    .col(integer(SnapDeviceLoraNode::AppNon))
                    .col(integer(SnapDeviceLoraNode::NetId))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(SnapSnapDevice::Table)
                    .if_not_exists()
                    .col(big_key_auto(SnapSnapDevice::Id))
                    .col(big_integer(SnapSnapDevice::DeviceId).unique_key())
                    .col(text(SnapSnapDevice::Eui))
                    .col(text(SnapSnapDevice::Key))
                    .col(text(SnapSnapDevice::ProductType))
                    .col(small_integer_null(SnapSnapDevice::Battery))
                    .col(boolean(SnapSnapDevice::Charge))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(SnapDeviceAuthority::Table)
                    .if_not_exists()
                    .col(big_key_auto(SnapDeviceAuthority::Id))
                    .col(big_integer(SnapDeviceAuthority::AuthCreator))
                    .col(big_integer(SnapDeviceAuthority::DeviceId))
                    .col(text(SnapDeviceAuthority::ShareType))
                    .col(big_integer(SnapDeviceAuthority::ShareId))
                    .col(boolean(SnapDeviceAuthority::Owner))
                    .col(boolean(SnapDeviceAuthority::Manager))
                    .col(boolean(SnapDeviceAuthority::Modify))
                    .col(boolean(SnapDeviceAuthority::Delete))
                    .col(boolean(SnapDeviceAuthority::Share))
                    .col(
                        timestamp_with_time_zone(SnapDeviceAuthority::CreateTime)
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("share-map-idx")
                    .table(SnapDeviceAuthority::Table)
                    .col(SnapDeviceAuthority::ShareType)
                    .col(SnapDeviceAuthority::ShareId)
                    .to_owned(),
            )
            .await;
        manager
            .create_table(
                Table::create()
                    .table(SnapDownlink::Table)
                    .if_not_exists()
                    .col(big_key_auto(SnapDownlink::Id))
                    .col(big_integer(SnapDownlink::DeviceId))
                    .col(big_integer(SnapDownlink::UserId))
                    .col(text(SnapDownlink::Name))
                    .col(text(SnapDownlink::Data))
                    .col(integer(SnapDownlink::Order))
                    .col(integer(SnapDownlink::Port))
                    .col(
                        timestamp_with_time_zone(SnapDownlink::CreateTime)
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }
    pub async fn down(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(SnapDeviceGroup::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SnapDeviceFunction::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SnapDevices::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SnapIntegrationMqtt::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SnapDeviceMapGroup::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SnapDeviceLoraGate::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SnapDeviceMQTT::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SnapDeviceLoraNode::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SnapSnapDevice::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SnapDeviceAuthority::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SnapDownlink::Table).to_owned()).await
    }

    #[derive(DeriveIden)]
    enum SnapDeviceGroup {
        Table,
        Id,
        Name,
        Description,
        DefaultGroup,
        Owner,
        CreateTime,
    }

    #[derive(DeriveIden)]
    enum SnapDeviceFunction {
        Table,
        Id,
        Device,
        FuncName,
        FuncValue,
    }

    #[derive(DeriveIden)]
    enum SnapProductInfo {
        Table,
        Id,
        Sku,
        Name,
        Description,
        Image,
        CreateTime,
    }
    #[derive(DeriveIden)]
    enum SnapDevices {
        Table,
        Id,
        Eui,
        Name,
        Description,
        Creator,
        Enable,
        Period,
        Script,
        DataId,
        ProductId,
        DeviceType,
        ActiveTime,
        CreateTime,
    }

    #[derive(DeriveIden)]
    enum SnapIntegrationMqtt {
        Table,
        Id,
        UserId,
        MqttType,
        Name,
        Username,
        Password,
        Enable,
        CreateTime,
    }

    #[derive(DeriveIden)]
    enum SnapDeviceMapGroup {
        Table,
        Id,
        UserId,
        DeviceId,
        GroupId,
        DevOrder,
        CreateTime,
    }

    #[derive(DeriveIden)]
    enum SnapDeviceLoraGate {
        Table,
        Id,
        DeviceId,
        Region,
        Eui,
        Product,
        Config,
    }

    #[derive(DeriveIden)]
    enum SnapDeviceMQTT {
        Table,
        Id,
        DeviceId,
        Eui,
        Username,
        Password,
    }

    #[derive(DeriveIden)]
    enum SnapDeviceLoraNode {
        Table,
        Id,
        DeviceId,
        Region,
        JoinType,
        AppEui,
        DevEui,
        AppKey,
        DevAddr,
        NwkSkey,
        AppSkey,
        ClassB,
        ClassC,
        Adr,
        Rx1Delay,
        DesRx1Delay,
        Rx1Dro,
        DesRx1Dro,
        Rx2Dr,
        DesRx2Dr,
        Rx2Freq,
        DesRx2Freq,
        DRetry,
        CRetry,
        ProductType,
        Dutycyle,
        UpConfirm,
        UpDr,
        Power,
        Battery,
        Charge,
        TimeZone,
        Firmware,
        DevNon,
        AppNon,
        NetId,
    }

    #[derive(DeriveIden)]
    enum SnapSnapDevice {
        Table,
        Id,
        DeviceId,
        Eui,
        Key,
        ProductType,
        Battery,
        Charge,
    }

    #[derive(DeriveIden)]
    enum SnapDeviceAuthority {
        Table,
        Id,
        AuthCreator,
        DeviceId,
        ShareType,
        ShareId,
        Owner,
        Manager,
        Modify,
        Delete,
        Share,
        CreateTime,
    }

    #[derive(DeriveIden)]
    enum SnapDownlink {
        Table,
        Id,
        DeviceId,
        UserId,
        Name,
        Data,
        Order,
        Port,
        CreateTime,
    }
}

mod data {
    use crate::m20240904_020441_create_table::big_key_auto;
    use sea_orm_migration::{prelude::*, schema::*};

    pub async fn up(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SnapDeviceData::Table)
                    .if_not_exists()
                    .col(big_key_auto(SnapDeviceData::Id))
                    .col(big_integer(SnapDeviceData::DeviceId))
                    .col(json(SnapDeviceData::Data))
                    .col(text(SnapDeviceData::Bytes))
                    .col(
                        timestamp_with_time_zone(SnapDeviceData::CreateTime)
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(SnapDeviceDataName::Table)
                    .if_not_exists()
                    .col(big_key_auto(SnapDeviceDataName::Id))
                    .col(big_integer(SnapDeviceDataName::DeviceId))
                    .col(big_integer(SnapDeviceDataName::Owner))
                    .col(text(SnapDeviceDataName::Name))
                    .col(json(SnapDeviceDataName::Map))
                    .col(
                        timestamp_with_time_zone(SnapDeviceDataName::CreateTime)
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(SnapDecodeScript::Table)
                    .if_not_exists()
                    .col(big_key_auto(SnapDecodeScript::Id))
                    .col(text(SnapDecodeScript::Script))
                    .col(text(SnapDecodeScript::Lang))
                    .col(big_integer(SnapDecodeScript::Owner))
                    .col(text(SnapDecodeScript::Name))
                    .col(json(SnapDecodeScript::Map))
                    .col(
                        timestamp_with_time_zone(SnapDecodeScript::CreateTime)
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        timestamp_with_time_zone(SnapDecodeScript::ModifyTime)
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    pub async fn down(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(SnapDeviceData::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SnapDeviceDataName::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(SnapDecodeScript::Table).to_owned()).await
    }

    #[derive(DeriveIden)]
    enum SnapDeviceData {
        Table,
        Id,
        DeviceId,
        Data,
        Bytes,
        CreateTime,
    }

    #[derive(DeriveIden)]
    enum SnapDeviceDataName {
        Table,
        Id,
        DeviceId,
        Owner,
        Name,
        Map,
        CreateTime,
    }

    #[derive(DeriveIden)]
    enum SnapDecodeScript {
        Table,
        Id,
        Script,
        Lang,
        Owner,
        Name,
        Map,
        CreateTime,
        ModifyTime,
    }
}

mod admin {
    use crate::m20240904_020441_create_table::big_key_auto;
    use sea_orm_migration::{prelude::*, schema::*};

    pub async fn up(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SnapAdmin::Table)
                    .if_not_exists()
                    .col(big_key_auto(SnapAdmin::Id))
                    .col(uuid(SnapAdmin::UId))
                    .col(
                        timestamp_with_time_zone(SnapAdmin::CreateTime)
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(SnapConfig::Table)
                    .if_not_exists()
                    .col(big_key_auto(SnapConfig::Id))
                    .col(text_uniq(SnapConfig::Name))
                    .col(text_uniq(SnapConfig::Value))
                    .col(
                        timestamp_with_time_zone(SnapConfig::CreateTime)
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    pub async fn down(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(SnapAdmin::Table).to_owned()).await
    }

    #[derive(DeriveIden)]
    enum SnapAdmin {
        Table,
        Id,
        UId,
        CreateTime,
    }
    #[derive(DeriveIden)]
    enum SnapConfig {
        Table,
        Id,
        Name,
        Value,
        CreateTime,
    }
}
