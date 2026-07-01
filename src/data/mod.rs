use std::io::Write;

use cookie_factory::{SerializeFn, gen_simple};
use nom::IResult;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    error::Error,
    net::data::{generate_string, parse_string},
};

pub mod datapack;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Identifier {
    pub namespace: String,
    pub value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum TextComponent {
    String(String),
    List(Vec<TextComponent>),
    Object(TextComponentObject),
}

#[derive(Serialize, Deserialize)]
pub struct TextComponentObject {
    #[serde(flatten)]
    content: TextComponentContent,
    #[serde(default)]
    extra: Vec<TextComponent>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    font: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    bold: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    italic: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    underlined: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    strikethrough: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    obfuscated: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    shadow_color: Option<i32>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    insertion: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    click_event: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    hover_event: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Serialize, Deserialize)]
pub enum TextComponentContent {
    Plain {
        text: String,
    },
    Translatable {
        translate: String,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        fallback: Option<String>,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        with: Option<serde_json::Map<String, serde_json::Value>>,
    },
    Scoreboard {
        score: TextComponentScoreboardValue,
    },
    EntitySelector {
        selector: String,
        separator: Box<TextComponent>,
    },
    Keybind {
        keybind: String,
    },
    Nbt {
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        source: Option<TextComponentNbtSource>,
        nbt: String,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        interpret: Option<bool>,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        plain: Option<bool>,
        separator: Box<TextComponent>,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        entity: Option<String>,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        block: Option<String>,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        storage: Option<String>,
    },
    Atlas {
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        atlas: Option<String>,
        sprite: String,
    },
    Player {
        #[serde(deserialize_with = "check_is_player_object")]
        #[serde(serialize_with = "make_player")]
        object: (),
        #[serde(skip)]
        player: (),
    },
}

#[derive(Serialize, Deserialize)]
pub enum TextComponentNbtSource {
    #[serde(rename = "block")]
    Block,
    #[serde(rename = "entity")]
    Entity,
    #[serde(rename = "storage")]
    Storage,
}

fn check_is_player_object<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    let string = String::deserialize(deserializer)?;
    if string != "player" {
        Err(serde::de::Error::custom(format!(
            "\"object\" != \"player\""
        )))
    } else {
        Ok(())
    }
}

fn make_player<'ser, S>(_value: &(), serilalizer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serilalizer.serialize_str("player")
}

#[derive(Serialize, Deserialize)]
pub struct TextComponentScoreboardValue {
    name: String,
    objective: String,
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
