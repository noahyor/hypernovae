use crate::net::{packet::ServerboundPacketType, proto::ProtocolState};
use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error<I> {
    #[error("error while parsing: {0:?}")]
    Nom(#[from] nom::error::Error<I>),
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("error while generating: {0:?}")]
    CookieFactory(#[from] cookie_factory::GenError),
    #[error("{0}")]
    Protocol(ProtocolError),
    #[error("error while parsing a string \"{0:?}\"")]
    String(I),
    #[error("unknown intent in a handshaking packet, expected 1-3, got {0}")]
    UnknownIntent(i32),
    #[error("unknown packet ID {1} in state {0:?}")]
    UnknownPacket(ProtocolState, i32),
    #[error("unknown chat mode while configuring {0}")]
    ChatMode(i32),
    #[error("reserved bit set in skin options")]
    SkinOptions,
    #[error("invalid hand id {0}")]
    UnknownHand(i32),
    #[error("invalid particle option {0}")]
    ParticleOptions(i32),
    #[error("timed out")]
    Timeout,
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("recieved {0:?} when it should only be a response")]
    OnlyReply(ServerboundPacketType),
    #[error("recieved {0:?} expecting any of {1:?}")]
    NotReply(ServerboundPacketType, Vec<ServerboundPacketType>),
    #[error("invalid state, got {0:?} expecting {1:?}")]
    InvalidState(ProtocolState, ProtocolState),
    #[error("timed out")]
    Timeout,
    #[error("invalid ping response, expected {0} but got {1}")]
    InvalidPingResponse(u32, u32),
    #[error("error while configuring connection")]
    InvalidConfig,
    #[error("client disconnected")]
    Disconnected,
}

// pub(crate) type AnyError = Box<dyn std::error::Error>;

pub(crate) fn error_to_owned(error: Error<&[u8]>) -> Error<Vec<u8>> {
    use crate::error::Error;
    match error {
        Error::Nom(err) => match err {
            _ => todo!(),
        },
        Error::String(bytes) => crate::error::Error::String(bytes.to_vec()),
        Error::UnknownPacket(state, id) => Error::UnknownPacket(state, id),
        Error::UnknownIntent(id) => Error::UnknownIntent(id),
        Error::IO(e) => Error::IO(e),
        Error::CookieFactory(e) => Error::CookieFactory(e),
        Error::ChatMode(e) => Error::ChatMode(e),
        Error::SkinOptions => Error::SkinOptions,
        Error::UnknownHand(e) => Error::UnknownHand(e),
        Error::ParticleOptions(e) => Error::ParticleOptions(e),
        Error::Protocol(e) => Error::Protocol(e),
        Error::Timeout => Error::Timeout,
    }
}

pub(crate) fn map_nom_err<F, A, B>(error: nom::Err<A>, f: F) -> nom::Err<B>
where
    F: Fn(A) -> B,
{
    match error {
        nom::Err::Incomplete(needed) => nom::Err::Incomplete(needed),
        nom::Err::Error(err) => nom::Err::Error(f(err)),
        nom::Err::Failure(err) => nom::Err::Failure(f(err)),
    }
}

pub(crate) fn asciify(bytes: &[u8]) -> String {
    let mut ret = String::new();
    for &b in bytes {
        ret.push(if b.is_ascii() && !b.is_ascii_control() {
            char::from(b)
        } else {
            '.'
        });
    }
    ret
}
