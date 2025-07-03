fn main() {
    tonic_build::configure()
        .type_attribute("manager.GwConfig", "#[derive(serde::Serialize, serde::Deserialize)]")
        .field_attribute("manager.GwConfig.port_up", r#"#[serde(skip_serializing_if = "Option::is_none")]"#)
        .field_attribute("manager.GwConfig.port_up1", r#"#[serde(skip_serializing_if = "Option::is_none")]"#)
        .field_attribute("manager.GwConfig.port_down", r#"#[serde(skip_serializing_if = "Option::is_none")]"#)
        .field_attribute("manager.GwConfig.port_down1", r#"#[serde(skip_serializing_if = "Option::is_none")]"#)
        .field_attribute("manager.GwConfig.server_addr", r#"#[serde(skip_serializing_if = "Option::is_none")]"#)
        .field_attribute("manager.GwConfig.server_addr1", r#"#[serde(skip_serializing_if = "Option::is_none")]"#)
        .field_attribute("manager.GwConfig.extra_server", r#"#[serde(skip_serializing_if = "Option::is_none")]"#)
        .field_attribute("manager.GwConfig.region", r#"#[serde(skip_serializing_if = "Option::is_none")]"#)
        .field_attribute("manager.GwConfig.subband", r#"#[serde(skip_serializing_if = "Option::is_none")]"#)
        .field_attribute("manager.GwConfig.mode", r#"#[serde(skip_serializing_if = "Option::is_none")]"#)
        .field_attribute("manager.GwConfig.basicstation_type", r#"#[serde(skip_serializing_if = "Option::is_none")]"#)
        .field_attribute("manager.GwConfig.basicstation_url", r#"#[serde(skip_serializing_if = "Option::is_none")]"#)
        .field_attribute("manager.GwConfig.basicstation_port", r#"#[serde(skip_serializing_if = "Option::is_none")]"#)
        .field_attribute("manager.GwConfig.trust_mode", r#"#[serde(skip_serializing_if = "Option::is_none")]"#)
        .field_attribute("manager.GwConfig.api_token", r#"#[serde(skip_serializing_if = "Option::is_none")]"#)
        .field_attribute("manager.GwConfig.cert", r#"#[serde(skip_serializing_if = "Option::is_none")]"#)
        .field_attribute("manager.GwConfig.client_cert", r#"#[serde(skip_serializing_if = "Option::is_none")]"#)
        .compile_protos(&["proto/manager.proto"], &["proto"])
        .unwrap();
}


