use crate::{
    error::Error,
    net::data::{generate_optional, generate_string, generate_varint, parse_string},
};
use bitflags::bitflags;
use cookie_factory::{SerializeFn, combinator::string, gen_simple, multi::many_ref};
use nom::{IResult, bytes::streaming::take_until1};
use std::{io::Write, num::NonZero};

#[derive(Debug)]
pub struct PlayerBuilder {
    profile: Option<Profile>,
    skin_options: SkinOptions,
    main_hand: Hand,
    allow_server_listing: bool,
    locale: Option<String>,
    view_distance: Option<u8>,
    chat_options: ChatOptions,
    particle_options: ParticleOptions,
}

pub struct Player {
    profile: Profile,
    skin_options: SkinOptions,
    main_hand: Hand,
    allow_server_listing: bool,
    locale: String,
    view_distance: u8,
    chat_options: ChatOptions,
    particle_options: ParticleOptions,
}

#[derive(Debug, Clone)]
pub struct Profile {
    uuid: u128,
    username: String,
    properties: Vec<ProfileProperty>,
}

#[derive(Debug, Clone)]
pub struct ProfileProperty {
    name: String,
    value: String,
    signature: Option<String>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct DatapackVersion {
    identifier: Identifier,
    version: String,
}

bitflags! {
    #[derive(Debug, Clone)]
    pub struct SkinOptions: u8 {
        const Cape = 0x01;
        const Jacket = 0x02;
        const LeftSleeve = 0x04;
        const RightSleeve = 0x08;
        const LeftLeg = 0x10;
        const RightLeg = 0x20;
        const Hat = 0x40;
    }
}

#[derive(Debug, Clone)]
pub enum Hand {
    Left,
    Right,
}

bitflags! {
    #[derive(Debug, Clone)]
    pub struct ChatOptions: u8 {
        const Enabled = 0x01;
        const CommandsOnly = 0x02;
        const ColorsEnabled = 0x04;
        const FilteringEnabled = 0x08;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ParticleOptions {
    All,
    Decreased,
    Minimal,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Identifier {
    pub namespace: String,
    pub value: String,
}

impl PlayerBuilder {
    pub fn new() -> Self {
        Self {
            profile: None,
            skin_options: SkinOptions::all(),
            main_hand: Hand::Right,
            allow_server_listing: true,
            locale: None,
            view_distance: None,
            chat_options: ChatOptions::Enabled | ChatOptions::ColorsEnabled,
            particle_options: ParticleOptions::All,
        }
    }

    pub fn profile(mut self, profile: Profile) -> Self {
        self.profile = Some(profile);
        self
    }

    pub fn skin_options(mut self, skin_options: SkinOptions) -> Self {
        self.skin_options = skin_options;
        self
    }

    pub fn main_hand(mut self, main_hand: Hand) -> Self {
        self.main_hand = main_hand;
        self
    }

    pub fn build(self) -> Option<Player> {
        Some(Player {
            profile: self.profile?,
            skin_options: self.skin_options,
            main_hand: self.main_hand,
            allow_server_listing: self.allow_server_listing,
            locale: self.locale.unwrap_or(String::from("en_US")),
            view_distance: self.view_distance.unwrap_or(8),
            chat_options: self.chat_options,
            particle_options: self.particle_options,
        })
    }
}

impl Profile {
    pub fn new(uuid: u128, username: String, properties: Vec<ProfileProperty>) -> Self {
        Profile {
            uuid,
            username,
            properties,
        }
    }

    pub fn generate<W: Write>(&self) -> impl SerializeFn<W> {
        |mut w| {
            w.write_all(&self.uuid.to_be_bytes()[..])?;
            let w = generate_string(&self.username[..])(w)?;
            let w = gen_simple(generate_varint(self.properties.len() as i32), w)?;
            let w = gen_simple(many_ref(&self.properties, ProfileProperty::generate), w)?;
            Ok(w)
        }
    }
}

impl ProfileProperty {
    pub fn generate<W: Write>(&self) -> impl SerializeFn<W> {
        |w| {
            let w = gen_simple(generate_string(&self.name), w)?;
            let w = gen_simple(generate_string(&self.value), w)?;
            let signature = &self.signature.as_deref();
            let w = gen_simple(
                generate_optional(signature, |string| generate_string(*string)),
                w,
            )?;
            Ok(w)
        }
    }
}

impl Identifier {
    pub fn new<A: Into<String>, B: Into<String>>(namespace: A, value: B) -> Self {
        Self {
            namespace: namespace.into(),
            value: value.into(),
        }
    }

    pub(crate) fn parse(data: &[u8]) -> IResult<&[u8], Self, Error<&[u8]>> {
        let (data, string) = parse_string(data)?;
        if let Some((namespace, value)) = string.split_once(':') {
            return Ok((
                data,
                Self {
                    namespace: namespace.to_owned(),
                    value: value.to_owned(),
                },
            ));
        };
        Ok((
            data,
            Self {
                namespace: String::from("minecraft"),
                value: string.to_owned(),
            },
        ))
    }

    pub(crate) fn generate<W: Write>(&self) -> impl SerializeFn<W> {
        |w| {
            let mut temp = String::new();
            temp.push_str(&self.namespace);
            temp.push(':');
            temp.push_str(&self.value);
            gen_simple(generate_string(&*temp), w)
        }
    }
}

impl ToString for Identifier {
    fn to_string(&self) -> String {
        format!("{}:{}", self.namespace, self.value)
    }
}

impl DatapackVersion {
    pub fn new<V: Into<String>>(id: Identifier, ver: V) -> Self {
        Self {
            identifier: id,
            version: ver.into(),
        }
    }

    pub(crate) fn parse(data: &[u8]) -> IResult<&[u8], Self, Error<&[u8]>> {
        let (data, namespace) = parse_string(data)?;
        let (data, value) = parse_string(data)?;
        let (data, version) = parse_string(data)?;
        Ok((data, Self::new(Identifier::new(namespace, value), version)))
    }

    pub(crate) fn generate<W: Write>(&self) -> impl SerializeFn<W> {
        |w| {
            let w = gen_simple(generate_string(&self.identifier.namespace), w)?;
            let w = gen_simple(generate_string(&self.identifier.value), w)?;
            gen_simple(generate_string(&self.version), w)
        }
    }
}
