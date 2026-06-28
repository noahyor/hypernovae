use std::io::Write;

use cookie_factory::{SerializeFn, gen_simple};
use nom::IResult;

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
