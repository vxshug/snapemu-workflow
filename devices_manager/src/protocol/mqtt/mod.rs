mod server;
mod version;

pub use server::broker::Broker;
pub use server::broker::MqttConfig;
pub use server::broker::ListenConfig;
pub use server::broker::ConnectionSettings;
pub use server::router::RouterConfig;
pub use server::tls::TlsConfig;
pub use server::Notification;
pub use server::MqttAuth;
pub use server::MqttAuthRequest;
pub use server::link::Error as LinkError;
pub use server::local::LinkRx;
pub use server::local::LinkTx;
pub type ConnectionId = usize;
pub type RouterId = usize;
pub type NodeId = usize;
pub type Topic = String;
pub type Filter = String;
pub type TopicId = usize;
pub type Offset = (u64, u64);
pub type Cursor = (u64, u64);

pub type ClientId = String;
pub type AuthUser = String;
pub type AuthPass = String;