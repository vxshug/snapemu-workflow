use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::thread;
use flume::Sender;
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::error;
use crate::protocol::mqtt::ConnectionId;
use crate::protocol::mqtt::server::{local, AuthHandler, Event};
use crate::protocol::mqtt::server::local::{LinkBuilder, LinkRx, LinkTx};
use crate::protocol::mqtt::server::router::{Router, RouterConfig};
use crate::protocol::mqtt::server::tls::TlsConfig;
use crate::protocol::mqtt::server::server::{Error, Server};

pub trait IO: AsyncRead + AsyncWrite + Send + Unpin {}
impl<T> IO for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

#[derive(Deserialize, Clone)]
pub struct MqttConfig {
    pub id: usize,
    pub listen: Option<HashMap<String, ListenConfig>>,
    pub router: RouterConfig,
    #[serde(skip)]
    pub external_auth: Option<AuthHandler>,
}

impl Debug for MqttConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MqttConfig")
            .field("id", &self.id)
            .field("listen", &self.listen)
            .field("router", &self.router)
            .finish()
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct ListenConfig {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub ws: bool,
    pub tls: Option<TlsConfig>,
    pub next_connection_delay_ms: u64,
    pub tls_cert: Option<String>,
    pub tls_key: Option<String>,
    pub tls_cert_file: Option<String>,
    pub tls_key_file: Option<String>,
    pub connections: ConnectionSettings
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectionSettings {
    pub connection_timeout_ms: u16,
    pub max_payload_size: usize,
    pub max_inflight_count: usize,
    #[serde(default)]
    pub dynamic_filters: bool,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum MqttVersion {
    V3,
    V4,
    V5,
}

pub struct Broker {
    config: Arc<MqttConfig>,
    router_tx: Sender<(ConnectionId, Event)>,
}

impl Broker {
    pub fn new(config: MqttConfig) -> Broker {
        let config = Arc::new(config);
        let router_config = config.router.clone();
        let router: Router = Router::new(config.id, router_config);

        let router_tx = router.spawn();
        Broker { config, router_tx }
    }

    pub fn link(&self, client_id: &str) -> Result<(LinkTx, LinkRx), local::LinkError> {
        let (link_tx, link_rx, _ack) =
            LinkBuilder::new(client_id, self.router_tx.clone()).build()?;
        Ok((link_tx, link_rx))
    }

    #[tracing::instrument(skip(self))]
    pub fn start(&mut self) -> Result<(), Error> {
        if self.config.listen.is_none() {
            return Err(Error::Config("MQTT server not listen config".into()));
        }

        let auth_handler = self.config.external_auth.clone().unwrap();
        let mut server_thread_handles = Vec::new();
        let listen_config = self.config.listen.as_ref().unwrap();
        for (name, listen_config) in listen_config {
            let server_thread = thread::Builder::new().name(name.clone());
            let mut server = Server::new(listen_config.clone(), self.router_tx.clone(), auth_handler.clone())?;
            let handle = server_thread.spawn(move || {
                let mut runtime = tokio::runtime::Builder::new_current_thread();
                let runtime = runtime.enable_all().build().unwrap();

                runtime.block_on(async {
                    if let Err(e) = server.start().await {
                        error!(error=?e, "MQTT Server error");
                    }
                });
            })?;
            server_thread_handles.push(handle)
        }
        server_thread_handles.into_iter().for_each(|handle| {
            let _ = handle.join();
        });
        Ok(())
    }
}
