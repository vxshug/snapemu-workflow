use prost::Message;

pub mod manager {
    tonic::include_proto!("manager");
}
pub mod event;

