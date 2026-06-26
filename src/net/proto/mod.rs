use std::{collections::HashSet, convert::identity, time::Duration};

use either::Either::{self, Left};

use crate::{
    error::{
        Error::{self, Protocol},
        ProtocolError,
    },
    game::{DatapackVersion, Identifier, Player, PlayerBuilder, Profile},
    net::{
        data::generate_string,
        packet::{
            ClientInformationPacket, ClientboundPacket, HandshakeIntent, LoginSuccessPacket,
            MCStream, PluginMessagePacket, ServerboundPacket, ServerboundPacketType,
            StatusResponsePacket,
        },
    },
};

pub struct Connection {
    io: MCStream,
}

impl Connection {
    pub fn from_stream(io: MCStream) -> Self {
        Self { io }
    }

    pub async fn next(&mut self) -> Result<ProtocolEvent, Error<Vec<u8>>> {
        loop {
            let packet = self.io.next().await?;
            println!("{:?}", packet);
            if let None = packet {
                return Ok(ProtocolEvent::ConnectionClosed);
            }
            let packet = packet.unwrap();
            match packet {
                ServerboundPacket::Handshake(packet) => match packet.intent() {
                    HandshakeIntent::Status => {
                        let packet = self.io.next().await?;
                        if let None = packet {
                            return Ok(ProtocolEvent::ConnectionClosed);
                        }
                        let packet = packet.unwrap();
                        match packet {
                            ServerboundPacket::StatusRequest => {
                                return Ok(ProtocolEvent::StatusRequest);
                            }
                            ServerboundPacket::StatusPing(n) => {
                                self.io.send(&ClientboundPacket::StatusPong(n)).await?;
                                return Ok(ProtocolEvent::ConnectionClosed);
                            }
                            any => {
                                return Err(Error::Protocol(ProtocolError::NotReply(
                                    any.packet_type(),
                                    vec![
                                        ServerboundPacketType::StatusRequest,
                                        ServerboundPacketType::StatusPing,
                                    ],
                                )));
                            }
                        };
                    }
                    HandshakeIntent::Transfer => return Ok(ProtocolEvent::Transfer),
                    HandshakeIntent::Login => {
                        let packet = self.io.next().await?;
                        if let None = packet {
                            return Ok(ProtocolEvent::ConnectionClosed);
                        }
                        let packet = packet.unwrap();
                        match packet {
                            ServerboundPacket::LoginStart(packet) => {
                                return Ok(ProtocolEvent::Login(
                                    packet.name().to_owned(),
                                    packet.uuid(),
                                ));
                            }
                            any => {
                                return Err(Error::Protocol(ProtocolError::NotReply(
                                    any.packet_type(),
                                    vec![ServerboundPacketType::LoginStart],
                                )));
                            }
                        }
                    }
                },
                ServerboundPacket::StatusPing(n) => {
                    self.io.send(&ClientboundPacket::StatusPong(n)).await?;
                    return Ok(ProtocolEvent::ConnectionClosed);
                }
                ServerboundPacket::LoginStart(_) => (),
                ServerboundPacket::LoginAcknowledged => {
                    return Err(Error::Protocol(ProtocolError::OnlyReply(
                        ServerboundPacketType::LoginAcknowledged,
                    )));
                }
                ServerboundPacket::PluginMessage(_) => (),
                ServerboundPacket::ClientInformation(_) => (),
                ServerboundPacket::StatusRequest => {
                    return Ok(ProtocolEvent::StatusRequest);
                }
                ServerboundPacket::ConfigPong(_) => (),
                ServerboundPacket::KnownPacks(_) => (),
            }
        }
    }

    pub async fn next_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<ProtocolEvent, Error<Vec<u8>>> {
        tokio::time::timeout(timeout, self.next())
            .await
            .map_or_else(|_| Err(Error::Protocol(ProtocolError::Timeout)), identity)
    }

    pub async fn login(
        &mut self,
        compression: Option<u32>,
        profile: Profile,
    ) -> Result<(), Error<Vec<u8>>> {
        if self.io.state() != ProtocolState::Login {
            return Err(Error::Protocol(ProtocolError::InvalidState(
                self.io.state(),
                ProtocolState::Login,
            )));
        }
        println!("logging in");
        if let Some(n) = compression {
            self.io.send(&ClientboundPacket::SetCompression(n)).await?;
            self.io.set_compression(n);
        }
        let packet = LoginSuccessPacket::new(profile);
        self.io
            .send(&ClientboundPacket::LoginSuccess(packet))
            .await?;
        self.ensure_response_immediate(ServerboundPacketType::LoginAcknowledged)
            .await?;
        Ok(())
    }

    pub async fn configure<B, C, P, M>(
        &mut self,
        datapacks: Vec<DatapackVersion>,
        mut brand_handler: B,
        mut client_info_handler: C,
        mut plugin_message_handler: P,
        mut missing_packs_handler: M,
    ) -> Result<(), Error<Vec<u8>>>
    where
        B: FnMut(String) -> String,
        C: FnMut(&ClientInformationPacket),
        P: FnMut(&mut Self, &PluginMessagePacket),
        M: FnMut(&mut Self, Vec<DatapackVersion>),
    {
        if self.io.state() != ProtocolState::Configuration {
            return Err(Error::Protocol(ProtocolError::InvalidState(
                self.io.state(),
                ProtocolState::Configuration,
            )));
        }
        println!("configurating");
        self.io
            .send(&ClientboundPacket::KnownPacks(datapacks.clone()))
            .await?;
        loop {
            let result = self.io.next_timeout(Duration::from_millis(200)).await;
            if let Err(Error::Timeout) = result {
                break;
            } else {
                let packet = result?;
                if let None = packet {
                    return Err(Error::Protocol(ProtocolError::Disconnected));
                }
                let packet = packet.unwrap();
                match &packet {
                    ServerboundPacket::ClientInformation(packet) => client_info_handler(packet),
                    ServerboundPacket::PluginMessage(packet) => {
                        if packet.channel == Identifier::new("minecraft", "brand") {
                            self.io
                                .send_plugin::<Vec<u8>, Vec<u8>>(
                                    Identifier::new("minecraft", "brand"),
                                    cookie_factory::gen_simple(
                                        generate_string(&*brand_handler(
                                            String::from_utf8_lossy(&packet.data[1..]).into_owned(),
                                        )),
                                        Vec::new(),
                                    )?,
                                )
                                .await?;
                        }
                        plugin_message_handler(self, packet)
                    }
                    ServerboundPacket::KnownPacks(returned_packs) => {
                        let hashset: HashSet<DatapackVersion> =
                            returned_packs.clone().into_iter().collect();
                        let mut vec = Vec::new();
                        datapacks.clone().into_iter().for_each(|val| {
                            if !hashset.contains(&val) {
                                vec.push(val)
                            }
                        });
                        if vec.len() != 0 {
                            missing_packs_handler(self, vec)
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }
        Ok(())
    }

    pub async fn ping(&mut self, value: u32) -> Result<(), Error<Vec<u8>>> {
        if !((self.io.state() == ProtocolState::Configuration)
            | (self.io.state() == ProtocolState::Play))
        {
            return Err(Error::Protocol(ProtocolError::InvalidState(
                self.io.state(),
                ProtocolState::Configuration,
            )));
        }
        if self.io.state() == ProtocolState::Configuration {
            self.io.send(&ClientboundPacket::ConfigPing(value)).await?;
        }
        let packet = self
            .ensure_response_immediate(ServerboundPacketType::ConfigPong)
            .await?;
        if let ServerboundPacket::ConfigPong(n) = packet {
            if n == value {
                Ok(())
            } else {
                Err(Error::Protocol(ProtocolError::InvalidPingResponse(
                    value, n,
                )))
            }
        } else {
            unreachable!()
        }
    }

    pub async fn status_response(
        &mut self,
        status: serde_json::Value,
    ) -> Result<(), Error<Vec<u8>>> {
        if self.io.state() != ProtocolState::Status {
            return Err(Error::Protocol(ProtocolError::InvalidState(
                self.io.state(),
                ProtocolState::Status,
            )));
        }
        let packet = StatusResponsePacket::new(status);
        self.io
            .send(&ClientboundPacket::StatusResponse(packet))
            .await
    }

    // fn process(&mut self, packet: ServerboundPacket) -> Result<Option<()

    async fn ensure_response_immediate(
        &mut self,
        packet_type: ServerboundPacketType,
    ) -> Result<ServerboundPacket, Error<Vec<u8>>> {
        let packet = self.io.next().await?;
        if let None = packet {
            return Err(Error::Protocol(ProtocolError::Disconnected));
        }
        let packet = packet.unwrap();
        if packet.packet_type() != packet_type {
            Err(Error::Protocol(ProtocolError::NotReply(
                packet.packet_type(),
                vec![packet_type],
            )))
        } else {
            Ok(packet)
        }
    }

    async fn ensure_response_buffered(
        &mut self,
        packet_type: ServerboundPacketType,
        timeout: usize,
    ) -> Result<Vec<ServerboundPacket>, Error<Vec<u8>>> {
        let mut vec = Vec::new();
        for _ in 0..timeout {
            let packet = self.io.next().await?;
            if let None = packet {
                return Err(Error::Protocol(ProtocolError::Disconnected));
            }
            let packet = packet.unwrap();
            if packet.packet_type() != packet_type {
                vec.push(packet);
                continue;
            } else {
                vec.push(packet);
                return Ok(vec);
            }
        }
        Err(Protocol(ProtocolError::Timeout))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolState {
    Handshaking,
    Status,
    Login,
    Configuration,
    Play,
}

#[derive(Debug)]
pub enum ProtocolEvent {
    StatusRequest,
    ConnectionClosed,
    Transfer,
    Login(String, u128),
}

pub enum ConfigurationEvent {
    PluginMessage(PluginMessagePacket),
    Brand(String),
    ClientInformation(ClientInformationPacket),
    MissingPacks(Vec<DatapackVersion>),
}
