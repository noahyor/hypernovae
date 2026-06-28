use crate::data::Identifier;
use crate::data::datapack::DatapackVersion;
use crate::error::Error;
use crate::error::asciify;
use crate::error::error_to_owned;
use crate::error::map_nom_err;
use crate::game::{ChatOptions, Hand, ParticleOptions, Profile, SkinOptions};
use crate::net::data::generate_array;
use crate::net::data::length_prefixed;
use crate::net::data::parse_array;
use crate::net::data::{generate_string, generate_varint, parse_bool, parse_string, parse_varint};
use crate::net::proto::ProtocolState;
use cookie_factory::{SerializeFn, gen_simple};
use nom::AsBytes;
use nom::IResult;
use std::convert::identity;
use std::io::Write;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub struct MCStream {
    io: TcpStream,
    state: ProtocolState,
    compression: Option<u32>,
    buffer: Vec<u8>,
}

impl MCStream {
    pub async fn next(&mut self) -> Result<Option<ServerboundPacket>, Error<Vec<u8>>> {
        let mut atleast = 0;
        let (data, packet) = loop {
            let mut accum_len = 0;
            loop {
                let mut buf = [0; 1000];
                let len = self.io.read(&mut buf).await?;
                if len == 0 {
                    return Ok(None);
                }
                accum_len += len;
                self.buffer.extend(buf[..len].iter());
                if accum_len < atleast { continue } else { break }
            }
            println!("{}", hex::encode_upper(&self.buffer));
            println!("{}", asciify(&self.buffer));
            let result = parse_varint(&self.buffer[..])
                .map_err(nom::Err::convert)
                .map_err(|e| map_nom_err(e, error_to_owned));
            let (data, packet_length) = match result {
                Ok(val) => val,
                Err(nom::Err::Incomplete(needed)) => {
                    match needed {
                        nom::Needed::Unknown => atleast = 0,
                        nom::Needed::Size(n) => atleast = n.into(),
                    };
                    continue;
                }
                Err(nom::Err::Error(e)) => break Err(e),
                Err(nom::Err::Failure(e)) => break Err(e),
            };
            if packet_length as usize > data.len() {
                atleast = packet_length as usize;
                continue;
            };
            let result = ServerboundPacket::parse(self.state)(data)
                .map_err(|e| map_nom_err(e, error_to_owned));
            match result {
                Ok(val) => break Ok(val),
                Err(nom::Err::Incomplete(needed)) => {
                    match needed {
                        nom::Needed::Unknown => atleast = 0,
                        nom::Needed::Size(n) => atleast = n.into(),
                    };
                    continue;
                }
                Err(nom::Err::Error(e)) => break Err(e),
                Err(nom::Err::Failure(e)) => break Err(e),
            }
        }?;
        let len_parsed = self.buffer.len() - data.len();
        let unparsed = self.buffer[len_parsed..].to_owned();
        self.buffer = unparsed;
        self.do_state_transition(&packet);
        Ok(Some(packet))
    }

    pub async fn next_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<ServerboundPacket>, Error<Vec<u8>>> {
        tokio::time::timeout(timeout, self.next())
            .await
            .map_or_else(|_| Err(Error::Timeout), identity)
    }

    pub async fn send<E>(&mut self, packet: &ClientboundPacket) -> Result<(), Error<E>> {
        let data = gen_simple(length_prefixed(packet.generate()), Vec::new())?;
        println!("{:?}", packet);
        println!("{}", hex::encode_upper(&data));
        println!("{}", asciify(&data));
        self.io.write_all(data.as_bytes()).await?;
        Ok(())
    }

    pub async fn send_plugin<E, G>(
        &mut self,
        channel: Identifier,
        data: Vec<u8>,
    ) -> Result<(), Error<E>> {
        self.send(&ClientboundPacket::ConfigPluginMessage(
            PluginMessagePacket { channel, data },
        ))
        .await
    }

    pub fn set_compression(&mut self, threshold: u32) {
        self.compression = Some(threshold);
    }

    fn do_state_transition(&mut self, packet: &ServerboundPacket) {
        match packet {
            ServerboundPacket::Handshake(handshake) => self.state = handshake.intent().into(),
            ServerboundPacket::LoginAcknowledged => self.state = ProtocolState::Configuration,
            _ => (),
        }
    }

    pub fn addr(&self) -> Result<SocketAddr, std::io::Error> {
        self.io.peer_addr()
    }

    pub fn from_tcp(io: TcpStream) -> MCStream {
        MCStream {
            io,
            compression: None,
            state: ProtocolState::Handshaking,
            buffer: Vec::new(),
        }
    }

    pub(crate) fn state(&self) -> ProtocolState {
        self.state
    }
}

pub struct MCListener {
    io: TcpListener,
}

impl MCListener {
    pub fn new(listener: TcpListener) -> Self {
        MCListener { io: listener }
    }

    pub async fn accept(&self) -> Result<MCStream, std::io::Error> {
        let (io, _) = self.io.accept().await?;
        Ok(MCStream::from_tcp(io))
    }
}

#[derive(Debug)]
pub enum ServerboundPacket {
    Handshake(HandshakePacket),
    StatusRequest,
    StatusPing(u64),

    LoginStart(LoginStartPacket),
    LoginAcknowledged,
    ConfigPong(u32),
    PluginMessage(PluginMessagePacket),
    ClientInformation(ClientInformationPacket),
    KnownPacks(Vec<DatapackVersion>),
    FinishConfig,
}

#[derive(Debug)]
pub enum ClientboundPacket {
    SetCompression(u32),
    LoginSuccess(LoginSuccessPacket),
    StatusResponse(StatusResponsePacket),
    StatusPong(u64),
    ConfigPing(u32),
    KnownPacks(Vec<DatapackVersion>),
    ConfigPluginMessage(PluginMessagePacket),
    FinishConfig,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ServerboundPacketType {
    Handshake,
    StatusRequest,
    StatusPing,
    LoginStart,
    LoginAcknowledged,
    PluginMessage,
    ClientInformation,
    ConfigPong,
    KnownPacks,
    FinishConfig,
}

#[derive(Debug)]
pub struct HandshakePacket {
    proto_ver: i32,
    address: String,
    port: u16,
    intent: HandshakeIntent,
}

#[derive(Debug, Clone, Copy)]
pub enum HandshakeIntent {
    Status,
    Login,
    Transfer,
}

#[derive(Debug)]
pub struct LoginStartPacket {
    name: String,
    uuid: u128,
}

#[derive(Debug)]
pub struct LoginSuccessPacket {
    profile: Profile,
}

#[derive(Debug, Clone)]
pub struct PluginMessagePacket {
    pub channel: Identifier,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ClientInformationPacket {
    locale: String,
    view_distance: u8,
    chat_options: ChatOptions,
    displayed_skin: SkinOptions,
    main_hand: Hand,
    allow_listings: bool,
    particle_options: ParticleOptions,
}

#[derive(Debug)]
pub struct StatusResponsePacket {
    response: serde_json::Value,
}

impl ServerboundPacket {
    pub fn parse(state: ProtocolState) -> impl Fn(&[u8]) -> IResult<&[u8], Self, Error<&[u8]>> {
        move |input| {
            if input.len() == 0 {
                return Err(nom::Err::Incomplete(nom::Needed::Unknown));
            }

            match state {
                ProtocolState::Handshaking => Self::parse_handshaking(input),
                ProtocolState::Login => Self::parse_login(input),
                ProtocolState::Configuration => Self::parse_configuration(input),
                ProtocolState::Status => Self::parse_status(input),
                ProtocolState::Play => todo!("play protocol state"),
            }
        }
    }

    pub fn packet_type(&self) -> ServerboundPacketType {
        use ServerboundPacketType::*;
        match self {
            Self::Handshake(_) => Handshake,
            Self::StatusRequest => StatusRequest,
            Self::StatusPing(_) => StatusPing,
            Self::LoginStart(_) => LoginStart,
            Self::LoginAcknowledged => LoginAcknowledged,
            Self::PluginMessage(_) => PluginMessage,
            Self::ClientInformation(_) => ClientInformation,
            Self::ConfigPong(_) => ConfigPong,
            Self::KnownPacks(_) => KnownPacks,
            Self::FinishConfig => FinishConfig,
        }
    }

    fn parse_handshaking(data: &[u8]) -> IResult<&[u8], Self, Error<&[u8]>> {
        let (data, packet_type) = parse_varint(data).map_err(nom::Err::convert)?;
        Ok(match packet_type {
            0x00 => {
                let (data, packet) = HandshakePacket::parse(data)?;
                (data, Self::Handshake(packet))
            }
            0xFE => todo!("Legacy Server List Ping"),
            ptype => {
                return Err(nom::Err::Failure(Error::UnknownPacket(
                    ProtocolState::Handshaking,
                    ptype,
                )));
            }
        })
    }

    fn parse_status(data: &[u8]) -> IResult<&[u8], Self, Error<&[u8]>> {
        let (data, packet_type) = parse_varint(data).map_err(nom::Err::convert)?;
        Ok(match packet_type {
            0x00 => (data, Self::StatusRequest),
            0x01 => {
                let (data, n) = nom::number::streaming::be_u64(data)
                    .map_err(nom::Err::convert::<nom::error::Error<&[u8]>>)?;
                (data, Self::StatusPing(n))
            }
            ptype => {
                return Err(nom::Err::Failure(Error::UnknownPacket(
                    ProtocolState::Status,
                    ptype,
                )));
            }
        })
    }

    fn parse_login(data: &[u8]) -> IResult<&[u8], ServerboundPacket, Error<&[u8]>> {
        let (data, packet_type) = parse_varint(data).map_err(nom::Err::convert)?;
        Ok(match packet_type {
            0x00 => {
                let (data, packet) = LoginStartPacket::parse(data)?;
                (data, Self::LoginStart(packet))
            }
            0x03 => (data, Self::LoginAcknowledged),
            ptype => {
                return Err(nom::Err::Failure(Error::UnknownPacket(
                    ProtocolState::Login,
                    ptype,
                )));
            }
        })
    }

    fn parse_configuration(data: &[u8]) -> IResult<&[u8], ServerboundPacket, Error<&[u8]>> {
        let (data, packet_type) = parse_varint(data).map_err(nom::Err::convert)?;
        Ok(match packet_type {
            0x00 => {
                let (data, packet) = ClientInformationPacket::parse(data)?;
                (data, Self::ClientInformation(packet))
            }
            0x02 => {
                let (data, packet) = PluginMessagePacket::parse(data)?;
                (data, Self::PluginMessage(packet))
            }
            0x03 => (data, Self::FinishConfig),
            0x07 => {
                let (data, datapacks) = parse_array(DatapackVersion::parse)(data)?;
                (data, Self::KnownPacks(datapacks))
            }
            ptype => {
                return Err(nom::Err::Failure(Error::UnknownPacket(
                    ProtocolState::Configuration,
                    ptype,
                )));
            }
        })
    }
}

impl ClientboundPacket {
    pub(crate) fn generate<W: Write>(&self) -> impl SerializeFn<W> {
        move |w| match self {
            Self::SetCompression(n) => {
                let w = generate_varint(3)(w)?;
                gen_simple(cookie_factory::bytes::be_u32(n.clone()), w)
            }
            Self::LoginSuccess(packet) => {
                let w = generate_varint(2)(w)?;
                packet.generate()(w)
            }
            Self::StatusResponse(packet) => {
                let w = generate_varint(0)(w)?;
                packet.generate()(w)
            }
            Self::StatusPong(n) => {
                let w = generate_varint(1)(w)?;
                gen_simple(cookie_factory::bytes::be_u64(n.clone()), w)
            }
            Self::ConfigPing(n) => {
                let w = generate_varint(5)(w)?;
                gen_simple(cookie_factory::bytes::be_u32(n.clone()), w)
            }
            Self::KnownPacks(packs) => {
                let w = generate_varint(14)(w)?;
                generate_array(packs, DatapackVersion::generate)(w)
            }
            Self::ConfigPluginMessage(packet) => {
                let w = generate_varint(1)(w)?;
                packet.generate()(w)
            }
            Self::FinishConfig => generate_varint(3)(w),
        }
    }
}

impl HandshakePacket {
    pub(crate) fn parse(input: &[u8]) -> IResult<&[u8], Self, Error<&[u8]>> {
        let (data, proto_ver) = parse_varint(input).map_err(nom::Err::convert)?;
        let (data, address) = parse_string(data)?;
        let (data, port) = nom::number::streaming::be_u16(data)
            .map_err(nom::Err::convert::<nom::error::Error<&[u8]>>)?;
        let (data, intent) = parse_varint(data).map_err(nom::Err::convert)?;
        let intent = match intent {
            1 => HandshakeIntent::Status,
            2 => HandshakeIntent::Login,
            3 => HandshakeIntent::Transfer,
            intent => return Err(nom::Err::Failure(Error::UnknownIntent(intent))),
        };
        Ok((
            data,
            HandshakePacket {
                address: address.to_string(),
                intent,
                port,
                proto_ver,
            },
        ))
    }

    pub fn intent(&self) -> HandshakeIntent {
        self.intent
    }

    pub fn proto_ver(&self) -> i32 {
        self.proto_ver
    }
}

impl Into<ProtocolState> for HandshakeIntent {
    fn into(self) -> ProtocolState {
        match self {
            Self::Status => ProtocolState::Status,
            Self::Login => ProtocolState::Login,
            Self::Transfer => ProtocolState::Login,
        }
    }
}

impl LoginStartPacket {
    fn parse(data: &[u8]) -> IResult<&[u8], Self, Error<&[u8]>> {
        let (data, name) = parse_string(data)?;
        let (data, uuid) = nom::number::streaming::be_u128(data)
            .map_err(nom::Err::convert::<nom::error::Error<&[u8]>>)?;
        Ok((
            data,
            LoginStartPacket {
                name: name.to_owned(),
                uuid,
            },
        ))
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn uuid(&self) -> u128 {
        self.uuid
    }
}

impl LoginSuccessPacket {
    pub fn new(profile: Profile) -> Self {
        LoginSuccessPacket { profile }
    }

    pub fn generate<W: Write>(&self) -> impl SerializeFn<W> {
        |w| self.profile.generate()(w)
    }
}

impl PluginMessagePacket {
    fn parse(data: &[u8]) -> IResult<&[u8], Self, Error<&[u8]>> {
        let (payload, channel) = Identifier::parse(data)?;
        Ok((
            &data[data.len()..],
            PluginMessagePacket {
                channel,
                data: payload.to_owned(),
            },
        ))
    }

    fn generate<W: Write>(&self) -> impl SerializeFn<W> {
        |w| {
            let mut w = gen_simple(generate_string(&*self.channel.to_string()), w)?;
            w.write_all(&self.data)?;
            Ok(w)
        }
    }
}

impl ClientInformationPacket {
    fn parse(data: &[u8]) -> IResult<&[u8], Self, Error<&[u8]>> {
        let (data, locale) = parse_string(data)?;
        let (data, view_distance) = nom::number::streaming::be_u8(data)
            .map_err(nom::Err::convert::<nom::error::Error<&[u8]>>)?;
        let (data, chat_mode) = parse_varint(data).map_err(nom::Err::convert)?;
        let (data, chat_colors) = parse_bool(data).map_err(nom::Err::convert)?;
        let (data, displayed_skin) = nom::number::streaming::be_u8(data)
            .map_err(nom::Err::convert::<nom::error::Error<&[u8]>>)?;
        let (data, main_hand) = parse_varint(data).map_err(nom::Err::convert)?;
        let (data, text_filtering) = parse_bool(data).map_err(nom::Err::convert)?;
        let (data, allow_listings) = parse_bool(data).map_err(nom::Err::convert)?;
        let (data, particle_options) = parse_varint(data).map_err(nom::Err::convert)?;

        let mut chat_options = ChatOptions::empty();
        match chat_mode {
            0 => chat_options |= ChatOptions::Enabled,
            1 => chat_options |= ChatOptions::Enabled.union(ChatOptions::CommandsOnly),
            2 => (),
            chat_mode => return Err(nom::Err::Failure(Error::ChatMode(chat_mode))),
        }
        if chat_colors {
            chat_options |= ChatOptions::ColorsEnabled
        }
        if text_filtering {
            chat_options |= ChatOptions::FilteringEnabled
        }
        let particle_options = match particle_options {
            0 => ParticleOptions::All,
            1 => ParticleOptions::Decreased,
            2 => ParticleOptions::Minimal,
            _ => return Err(nom::Err::Failure(Error::ParticleOptions(particle_options))),
        };
        let displayed_skin =
            SkinOptions::from_bits(displayed_skin).ok_or(nom::Err::Failure(Error::SkinOptions))?;
        let main_hand = match main_hand {
            0 => Hand::Left,
            1 => Hand::Right,
            _ => return Err(nom::Err::Failure(Error::UnknownHand(main_hand))),
        };
        Ok((
            data,
            Self {
                locale: locale.to_owned(),
                view_distance,
                allow_listings,
                chat_options,
                displayed_skin,
                main_hand,
                particle_options,
            },
        ))
    }
}

impl StatusResponsePacket {
    pub fn new(data: serde_json::Value) -> Self {
        Self { response: data }
    }

    fn generate<W: Write>(&self) -> impl SerializeFn<W> {
        |w| gen_simple(generate_string(&self.response.to_string()[..]), w)
    }
}
