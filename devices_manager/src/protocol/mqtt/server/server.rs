use crate::protocol::mqtt::server::broker::{ConnectionSettings, ListenConfig, MqttVersion, IO};
use crate::protocol::mqtt::server::link::Link;
use crate::protocol::mqtt::server::network::Network;
use crate::protocol::mqtt::server::tls::TLSAcceptor;
use crate::protocol::mqtt::server::{
    link, network, tls, AuthHandler, Event, MqttAuth, MqttAuthRequest,
};
use crate::protocol::mqtt::version::{
    ConnAck, ConnectReturnCode, Login, Packet, Protocol, V3, V4, V5,
};
use crate::protocol::mqtt::{version, ConnectionId};
use async_tungstenite::tungstenite;
use async_tungstenite::tungstenite::http::HeaderValue;
use bytes::BytesMut;
use flume::{RecvError, SendError, Sender};
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::time;
use tokio::time::error::Elapsed;
use tracing::log::debug;
use tracing::{error, field, info, Instrument, Span};
use uuid::Uuid;
use ws_stream_tungstenite::WsStream;

struct WSCallback;
impl tungstenite::handshake::server::Callback for WSCallback {
    fn on_request(
        self,
        _request: &tungstenite::handshake::server::Request,
        mut response: tungstenite::handshake::server::Response,
    ) -> Result<
        tungstenite::handshake::server::Response,
        tungstenite::handshake::server::ErrorResponse,
    > {
        response.headers_mut().insert("sec-websocket-protocol", HeaderValue::from_static("mqtt"));
        Ok(response)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Acceptor error")]
pub enum Error {
    #[error("I/O {0}")]
    Io(#[from] io::Error),
    #[error("Timeout")]
    Timeout(#[from] Elapsed),
    #[error("Channel recv error")]
    Recv(#[from] RecvError),
    #[error("Channel send error")]
    Send(#[from] SendError<(ConnectionId, Event)>),
    #[error("Certs error = {0}")]
    Certs(#[from] tls::Error),
    #[error("Accept error = {0}")]
    Accept(String),
    #[error("Remote error = {0}")]
    Remote(#[from] link::Error),
    #[error("Invalid configuration")]
    Config(String),
}

pub struct Server {
    auth_handler: AuthHandler,
    config: ListenConfig,
    router_tx: Sender<(ConnectionId, Event)>,
    acceptor: Option<TLSAcceptor>,
}

impl Server {
    pub fn new(
        config: ListenConfig,
        router_tx: Sender<(ConnectionId, Event)>,
        auth_handler: AuthHandler,
    ) -> Result<Self, Error> {
        let acceptor = config.tls.as_ref().map(|tls| TLSAcceptor::new(tls)).transpose()?;
        Ok(Server { config, router_tx, acceptor, auth_handler })
    }

    async fn tls_accept(&self, stream: TcpStream) -> Result<(Box<dyn IO>, Option<String>), Error> {
        match self.acceptor {
            Some(ref acceptor) => {
                let (tenant_id, network) = acceptor.accept(stream).await?;
                Ok((network, tenant_id))
            }
            None => Ok((Box::new(stream), None)),
        }
    }

    pub(crate) async fn start(&self) -> Result<(), Error> {
        let listener = {
            let bind = format!("{}:{}", self.config.host, self.config.port);
            info!(listen_addr = bind, "Listening started",);
            TcpListener::bind(bind).await?
        };
        let delay = Duration::from_millis(self.config.next_connection_delay_ms);
        let mut count: usize = 0;
        let config = Arc::new(self.config.connections.clone());

        loop {
            let (stream, addr) = match listener.accept().await {
                Ok((s, r)) => (s, r),
                Err(e) => {
                    error!(error=?e, "Unable to accept socket.");
                    continue;
                }
            };

            let (mut stream, tenant_id) = match self.tls_accept(stream).await {
                Ok(o) => o,
                Err(e) => {
                    error!(error=?e, "Tls accept error");
                    continue;
                }
            };

            if self.config.ws {
                stream = match async_tungstenite::tokio::accept_hdr_async(stream, WSCallback).await
                {
                    Ok(s) => Box::new(WsStream::new(s)),
                    Err(e) => {
                        error!(error=?e, "Websocket failed handshake");
                        continue;
                    }
                };
            }

            let router_tx = self.router_tx.clone();
            let config = config.clone();
            count += 1;

            tokio::task::spawn(
                remote(config, tenant_id.clone(), router_tx, stream, self.auth_handler.clone())
                    .instrument(tracing::info_span!(
                        "link",
                        client_id = field::Empty,
                        connection_id = field::Empty
                    )),
            );

            time::sleep(delay).await;
        }
    }
}

pub async fn mqtt_connect<P: Protocol>(
    config: Arc<ConnectionSettings>,
    network: &mut Network<P>,
    auth_handler: AuthHandler,
) -> Result<Packet, link::Error> {
    let connection_timeout_ms = config.connection_timeout_ms.into();
    let packet = time::timeout(Duration::from_millis(connection_timeout_ms), async {
        let packet = network.read().await?;
        Ok::<_, network::Error>(packet)
    })
    .await??;

    let (connect, _props, login) = match packet {
        Packet::Connect(ref connect, ref props, _, _, ref login) => (connect, props, login),
        packet => return Err(link::Error::NotConnectPacket(packet)),
    };

    Span::current().record("client_id", &connect.client_id);

    let mqtt_auth =
        handle_auth(login.as_ref(), connect.client_id.clone(), None, auth_handler).await?;
    network.set_mqtt_auth(mqtt_auth);
    if connect.keep_alive == 0 {
        return Err(link::Error::ZeroKeepAlive);
    }

    let empty_client_id = connect.client_id.is_empty();
    let clean_session = connect.clean_session;

    if empty_client_id && !clean_session {
        let ack =
            ConnAck { session_present: false, code: ConnectReturnCode::ClientIdentifierNotValid };

        let packet = Packet::ConnAck(ack, None);
        network.write(packet).await?;

        return Err(link::Error::InvalidClientId);
    }

    Ok(packet)
}

async fn handle_auth(
    login: Option<&Login>,
    client_id: String,
    common_name: Option<String>,
    auth_handler: AuthHandler,
) -> Result<MqttAuth, link::Error> {
    let Some(login) = login else {
        return Err(link::Error::InvalidAuth);
    };

    let username = login.username.clone();
    let password = login.password.clone();

    let mqtt_auth =
        auth_handler(MqttAuthRequest::new(common_name, username, password, client_id)).await?;

    Ok(mqtt_auth)
}

async fn remote(
    config: Arc<ConnectionSettings>,
    tenant_id: Option<String>,
    router_tx: Sender<(ConnectionId, Event)>,
    stream: Box<dyn IO>,
    auth_handler: AuthHandler,
) {
    let mut network = Network::new(
        stream,
        config.max_payload_size,
        config.max_inflight_count,
        V4,
        MqttAuth::default(),
    );
    let version = match network.try_connect().await {
        Ok(v) => v,
        Err(e) => {
            error!(error=?e, "Error while handling MQTT connect");
            return;
        }
    };

    match version {
        MqttVersion::V3 => {
            let network = network.new_protocol(V3);
            link_remote(config, tenant_id, router_tx, network, auth_handler).await;
        }
        MqttVersion::V4 => {
            let network = network.new_protocol(V4);
            link_remote(config, tenant_id, router_tx, network, auth_handler).await;
        }
        MqttVersion::V5 => {
            let network = network.new_protocol(V5);
            link_remote(config, tenant_id, router_tx, network, auth_handler).await;
        }
    }
}

async fn link_remote<P: Protocol>(
    config: Arc<ConnectionSettings>,
    tenant_id: Option<String>,
    router_tx: Sender<(ConnectionId, Event)>,
    mut network: Network<P>,
    auth_handler: AuthHandler,
) {
    let dynamic_filters = config.dynamic_filters;
    let (mut connect_packet) = match mqtt_connect(config, &mut network, auth_handler).await {
        Ok(p) => p,
        Err(e) => {
            error!(error=?e, "Error while handling MQTT connect packet");
            return;
        }
    };

    let (mut client_id, clean_session) = match &mut connect_packet {
        Packet::Connect(ref mut connect, _, _, _, _) => {
            connect.client_id = format!("{}.{}", network.mqtt_auth.id.0, connect.client_id);
            (connect.client_id.clone(), connect.clean_session)
        }
        _ => return,
    };

    let mut assigned_client_id = None;
    if client_id.is_empty() {
        let uuid = Uuid::nil().simple();
        client_id = format!("rumqtt-{uuid}");
        assigned_client_id = Some(client_id.clone());
    }

    // Start the link
    let mut link = match Link::new(
        router_tx.clone(),
        tenant_id.clone(),
        network,
        connect_packet,
        dynamic_filters,
        assigned_client_id,
    )
    .await
    {
        Ok(l) => l,
        Err(e) => {
            error!(error=?e, "Remote link error");
            return;
        }
    };

    let connection_id = link.connection_id;
    let mut send_disconnect = true;

    match link.start().await {
        // Connection got closed. This shouldn't usually happen.
        Ok(_) => error!("connection-stop"),
        // No need to send a disconnect message when disconnection
        // originated internally in the router.
        Err(link::Error::Link(e)) => {
            error!(error=?e, "router-drop");
            send_disconnect = false;
        }
        // Connection was closed by peer
        Err(link::Error::Network(network::Error::Io(err)) | link::Error::Io(err))
            if err.kind() == io::ErrorKind::ConnectionAborted =>
        {
            info!(error=?err, "disconnected");
        }
        // Any other error
        Err(e) => {
            error!(error=?e, "disconnected");
        }
    };

    if send_disconnect {
        let disconnect = Event::Disconnect;
        let message = (connection_id, disconnect);
        router_tx.send(message).ok();
    }
}

