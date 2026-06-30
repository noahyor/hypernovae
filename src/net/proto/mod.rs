use std::{collections::HashSet, convert::identity, time::Duration};

use either::Either::{self, Left};

use crate::{
    data::{Identifier, datapack::DatapackVersion},
    error::{
        Error::{self, Protocol},
        ProtocolError,
    },
    game::{Player, PlayerBuilder, Profile, data::BlockPosition},
    net::{
        data::generate_string,
        packet::{
            ClientInformationPacket, ClientboundPacket, FinalizeLoginPacket, HandshakeIntent,
            LoginSuccessPacket, MCStream, PluginMessagePacket, ServerboundPacket,
            ServerboundPacketType, StatusResponsePacket,
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
                ServerboundPacket::LoginStart(_) => todo!(),
                ServerboundPacket::LoginAcknowledged => {
                    return Err(Error::Protocol(ProtocolError::OnlyReply(
                        ServerboundPacketType::LoginAcknowledged,
                    )));
                }
                ServerboundPacket::PluginMessage(_) => todo!(),
                ServerboundPacket::ClientInformation(_) => todo!(),
                ServerboundPacket::StatusRequest => {
                    return Ok(ProtocolEvent::StatusRequest);
                }
                ServerboundPacket::ConfigPong(_) => todo!(),
                ServerboundPacket::KnownPacks(_) => todo!(),
                ServerboundPacket::FinishConfig => {
                    return Err(Error::Protocol(ProtocolError::OnlyReply(
                        ServerboundPacketType::FinishConfig,
                    )));
                }
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
    ) -> Result<(), Either<Error<Vec<u8>>, ()>>
    where
        B: FnMut(String) -> Result<String, ()>,
        C: FnMut(&ClientInformationPacket) -> Result<(), ()>,
        P: FnMut(&mut Self, &PluginMessagePacket) -> Result<(), ()>,
        M: FnMut(&mut Self, Vec<DatapackVersion>) -> Result<(), ()>,
    {
        if self.io.state() != ProtocolState::Configuration {
            return Err(Left(Error::Protocol(ProtocolError::InvalidState(
                self.io.state(),
                ProtocolState::Configuration,
            ))));
        }
        println!("configurating");
        self.io
            .send(&ClientboundPacket::KnownPacks(datapacks.clone()))
            .await
            .map_err(Either::Left)?;
        loop {
            let result = self.io.next_timeout(Duration::from_millis(200)).await;
            if let Err(Error::Timeout) = result {
                break;
            } else {
                let packet = result.map_err(Either::Left)?;
                if let None = packet {
                    return Err(Left(Error::Protocol(ProtocolError::Disconnected)));
                }
                let packet = packet.unwrap();
                match &packet {
                    ServerboundPacket::ClientInformation(packet) => {
                        client_info_handler(packet).map_err(Either::Right)?
                    }
                    ServerboundPacket::PluginMessage(packet) => {
                        if packet.channel == Identifier::new("minecraft", "brand") {
                            self.io
                                .send_plugin::<Vec<u8>, Vec<u8>>(
                                    Identifier::new("minecraft", "brand"),
                                    cookie_factory::gen_simple(
                                        generate_string(
                                            &*brand_handler(
                                                String::from_utf8_lossy(&packet.data[1..])
                                                    .into_owned(),
                                            )
                                            .map_err(Either::Right)?,
                                        ),
                                        Vec::new(),
                                    )
                                    .map_err(Into::into)
                                    .map_err(Either::Left)?,
                                )
                                .await
                                .map_err(Either::Left)?;
                        }
                        plugin_message_handler(self, packet).map_err(Either::Right)?
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
                            missing_packs_handler(self, vec).map_err(Either::Right)?
                        }
                        break;
                    }
                    _ => unreachable!(),
                }
            }
        }
        self.io
            .send(&ClientboundPacket::FinishConfig)
            .await
            .map_err(Either::Left)?;
        self.ensure_response_immediate(ServerboundPacketType::FinishConfig)
            .await
            .map_err(Either::Left)?;
        Ok(())
    }

    pub async fn finalize_login(
        &mut self,
        entity_id: i32,
        hardcore: bool,
        dimensions: Vec<Identifier>,
        max_players: i32,
        render_distance: i32,
        simulation_distance: i32,
        reduced_debug_info: bool,
        respawn_screen_enabled: bool,
        limited_crafting: bool,
        dimension_type: i32,
        dimension_name: Identifier,
        hashed_seed: i64,
        game_mode: u8,
        previous_game_mode: i8,
        is_debug_world: bool,
        is_flat_world: bool,
        death_location: Option<(Identifier, BlockPosition)>,
        portal_cooldown: i32,
        sea_level: i32,
        enforces_secure_chat: bool,
    ) -> Result<(), Error<Vec<u8>>> {
        self.io
            .send(&ClientboundPacket::FinalizeLogin(FinalizeLoginPacket {
                entity_id,
                hardcore,
                dimensions,
                max_players,
                render_distance,
                simulation_distance,
                reduced_debug_info,
                respawn_screen_enabled,
                limited_crafting,
                dimension_type,
                dimension_name,
                hashed_seed,
                game_mode,
                previous_game_mode,
                is_debug_world,
                is_flat_world,
                death_location,
                portal_cooldown,
                sea_level,
                enforces_secure_chat,
            }))
            .await?;
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
