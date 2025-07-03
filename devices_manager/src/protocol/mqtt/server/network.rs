use std::collections::VecDeque;
use std::io;
use std::io::ErrorKind;
use std::time::Duration;
use bytes::{BufMut, Bytes, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::error::Elapsed;
use tokio::time::timeout;
use x509_parser::nom::AsBytes;
use crate::protocol::mqtt::server::broker::{MqttVersion, IO};
use crate::protocol::mqtt::server::{MqttAuth, Packet};
use crate::protocol::mqtt::version;
use crate::protocol::mqtt::version::Protocol;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O = {0}")]
    Io(#[from] io::Error),
    #[error("Invalid data = {0}")]
    Protocol(#[from] version::Error),
    #[error["Keep alive timeout"]]
    KeepAlive(#[from] Elapsed),
}

pub struct Network<P> {
    /// Socket for IO
    socket: Box<dyn IO>,
    /// Buffered reads
    read: BytesMut,
    /// Buffered writes
    write: BytesMut,
    /// Maximum packet size
    max_incoming_size: usize,
    /// Maximum connection buffer count.
    max_connection_buffer_len: usize,
    /// Keep alive timeout
    keepalive: Duration,
    protocol: P,
    pub mqtt_auth: MqttAuth,
    topic_prefix: String
}

fn remaining_length(length_start: &[u8]) -> usize {
    let mut len = 0;
    for l in length_start {
        if len == 4 {
            return len;
        }
        len += 1;
        if (l & 0x80) == 0 {
            break
        }
    }
    len
}

impl<P: Protocol> Network<P> {
    pub fn new(
        socket: Box<dyn IO>,
        max_incoming_size: usize,
        max_connection_buffer_len: usize,
        protocol: P,
        mqtt_auth: MqttAuth
    ) -> Network<P> {
        let topic_prefix = format!("user/{}", mqtt_auth.id);
        Network {
            socket,
            read: BytesMut::with_capacity(10 * 1024),
            write: BytesMut::with_capacity(10 * 1024),
            max_incoming_size,
            max_connection_buffer_len,
            keepalive: Duration::MAX,
            protocol,
            mqtt_auth,
            topic_prefix
        }
    }

    pub fn new_protocol<N>(self, protocol: N) -> Network<N> {
        Network::<N> {
            protocol,
            socket: self.socket,
            read: self.read,
            write: self.write,
            max_incoming_size: self.max_incoming_size,
            max_connection_buffer_len: self.max_connection_buffer_len,
            keepalive: self.keepalive,
            mqtt_auth: self.mqtt_auth,
            topic_prefix: self.topic_prefix
        }
    }

    pub fn set_mqtt_auth(&mut self, mqtt_auth: MqttAuth) {
        let topic_prefix = format!("user/{}", mqtt_auth.id);
        self.topic_prefix = topic_prefix;
        self.mqtt_auth = mqtt_auth;
    }


    pub fn set_keepalive(&mut self, keepalive: u16) {
        let keepalive = Duration::from_secs(keepalive as u64);
        self.keepalive = keepalive + keepalive.mul_f32(0.5);
    }

    pub async fn try_connect(&mut self) -> Result<MqttVersion, Error> {
        if self.read.is_empty() {
            self.read_bytes(12).await?;
        }
        let len = remaining_length(&self.read[1..]);
        let start = len + 3;

        let protocol_name = &self.read[start..start + 4];
        if protocol_name == b"MQTT" {
            let protocol_level = self.read[start + 4];
            if protocol_level == 4 {
                return Ok(MqttVersion::V4)
            }
            if protocol_level == 5 {
                return Ok(MqttVersion::V5)
            }
            return Err(Error::Protocol(version::Error::InvalidProtocol))
        }

        let protocol_name = self.read.get(start..start + 6).ok_or(Error::Protocol(version::Error::InvalidProtocol))?;
        let protocol_level = *self.read.get(start + 6).ok_or(Error::Protocol(version::Error::InvalidProtocol))?;
        if protocol_name == b"MQIsdp" && protocol_level == 3 {
            return Ok(MqttVersion::V3)
        }
        Err(Error::Protocol(version::Error::InvalidProtocolLevel(protocol_level)))
    }

    async fn read_bytes(&mut self, required: usize) -> io::Result<usize> {
        let mut total_read = 0;
        loop {
            let read = self.socket.read_buf(&mut self.read).await?;
            if 0 == read {
                let error = if self.read.is_empty() {
                    io::Error::new(ErrorKind::ConnectionAborted, "connection closed by peer")
                } else {
                    io::Error::new(ErrorKind::ConnectionReset, "connection reset by peer")
                };

                return Err(error);
            }

            total_read += read;
            if total_read >= required {
                return Ok(total_read);
            }
        }
    }
    fn pack_topic(&mut self, mut packet: Packet ) -> Result<Packet, Error> {
        match packet {
            Packet::Publish(mut publish, o) => {
                let mut user_topic = BytesMut::from(self.topic_prefix.as_bytes());
                user_topic.put_slice(publish.topic.as_bytes());
                publish.topic = user_topic.freeze();
                Ok(Packet::Publish(publish, o))
            }
            Packet::Subscribe(mut subscribe, o) => {
                for filter in subscribe.filters.iter_mut() {
                    if !filter.path.starts_with("/") {
                        return Err(Error::Protocol(version::Error::TopicStart))
                    }
                    filter.path = format!("{}{}", self.topic_prefix, filter.path);
                }
                Ok(Packet::Subscribe(subscribe, o))
            }
            Packet::Unsubscribe(mut unsubscribe, o) => {
                for filter in unsubscribe.filters.iter_mut() {
                    *filter = format!("{}{}", self.topic_prefix, filter);
                }
                Ok(Packet::Unsubscribe(unsubscribe, o))
            }
            packet => Ok(packet)
        }
    }
    fn unpack_topic(&mut self, mut packet: Packet ) -> Result<Packet, Error> {
        match packet {
            Packet::Publish(mut publish, o) => {
                let topic_len = self.topic_prefix.len();
                if topic_len < publish.topic.len() {
                    publish.topic = publish.topic.slice(topic_len..);
                    return Ok(Packet::Publish(publish, o))
                }
                Ok(Packet::Publish(publish, o))
            }
            Packet::Subscribe(mut subscribe, o) => {
                let topic_len = self.topic_prefix.len();
                for filter in subscribe.filters.iter_mut() {
                    if topic_len < filter.path.len() {
                        filter.path = filter.path[topic_len..].to_owned();
                    }
                }
                Ok(Packet::Subscribe(subscribe, o))
            }
            Packet::Unsubscribe(mut unsubscribe, o) => {
                let topic_len = self.topic_prefix.len();
                for filter in unsubscribe.filters.iter_mut() {
                    if topic_len < filter.len() {
                        *filter = filter[topic_len..].to_owned();
                    }
                }
                Ok(Packet::Unsubscribe(unsubscribe, o))
            }
            packet => Ok(packet)
        }
    }
    pub async fn read(&mut self) -> Result<Packet, Error> {
        loop {
            let required = match Protocol::read_mut(
                &mut self.protocol,
                &mut self.read,
                self.max_incoming_size,
            ) {
                Ok(packet) => return self.pack_topic(packet),
                Err(version::Error::InsufficientBytes(required)) => required,
                Err(e) => return Err(e.into()),
            };

            // read more packets until a frame can be created. This function
            // blocks until a frame can be created. Use this in a select! branch
            timeout(self.keepalive, self.read_bytes(required)).await??;
        }
    }


    pub fn readv(&mut self, packets: &mut VecDeque<Packet>) -> Result<usize, Error> {
        loop {
            match self
                .protocol
                .read_mut(&mut self.read, self.max_incoming_size)
            {
                Ok(packet) => {
                    packets.push_back(self.pack_topic(packet)?);
                    let connection_buffer_length = packets.len();
                    if connection_buffer_length >= self.max_connection_buffer_len {
                        return Ok(connection_buffer_length);
                    }
                }
                Err(version::Error::InsufficientBytes(_)) => return Ok(packets.len()),
                Err(e) => return Err(io::Error::new(ErrorKind::InvalidData, e.to_string()).into()),
            }
        }
    }

    pub async fn write(&mut self, packet: Packet) -> Result<(), Error> {
        let packet = self.unpack_topic(packet)?;
        Protocol::write(&self.protocol, packet, &mut self.write)?;
        self.socket.write_all(&self.write).await?;
        self.write.clear();
        Ok(())
    }

    pub async fn writev(&mut self, packets: VecDeque<Packet>) -> Result<(), Error> {
        for packet in packets {
            let packet = self.unpack_topic(packet)?;
            Protocol::write(&self.protocol, packet, &mut self.write)?;
        }
        self.socket.write_all(&self.write).await?;
        self.write.clear();
        Ok(())
    }
}