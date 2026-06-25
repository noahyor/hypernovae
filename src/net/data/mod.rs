use std::{collections::VecDeque, io::Write};

use cookie_factory::{SerializeFn, gen_simple};
use nom::{
    Err, IResult, Parser, branch::alt, bytes::streaming::tag, combinator::value, error::ParseError,
    multi::many,
};

pub mod nbt;

use crate::error::Error;

pub fn parse_bool(input: &[u8]) -> IResult<&[u8], bool> {
    alt((value(false, tag(&[0][..])), value(true, tag(&[1][..])))).parse(input)
}

pub fn parse_varint(input: &[u8]) -> IResult<&[u8], i32> {
    let mut value = 0_i32;
    for index in 0..5 {
        if input.len() <= index {
            return Err(nom::Err::Incomplete(nom::Needed::Unknown));
        }
        value |= (input[index] as i32 & 0x7F) << (index * 7);
        if (input[index] & 0x80) == 0 {
            return Ok((&input[index + 1..], value));
        }
    }
    Err(nom::Err::Error(nom::error::Error::new(
        input,
        nom::error::ErrorKind::TooLarge,
    )))
}

pub fn parse_varlong(input: &[u8]) -> IResult<&[u8], i64> {
    let mut value = 0_i64;
    for index in 0..10 {
        value |= (input[index] as i64 & 0x7F) << (index * 7);
        if (input[index] & 0x80) == 0 {
            return Ok((&input[index + 1..], value));
        }
    }
    Err(nom::Err::Error(nom::error::Error::new(
        input,
        nom::error::ErrorKind::TooLarge,
    )))
}

pub fn parse_string(input: &[u8]) -> IResult<&[u8], &str, Error<&[u8]>> {
    let (data, length) = parse_varint(input).map_err(nom::Err::convert)?;
    let length = length as usize;
    match std::str::from_utf8(&data[..length]) {
        Err(_) => Err(nom::Err::Error(Error::String(input))),
        Ok(val) => Ok((&data[length..], val)),
    }
}

pub fn parse_optional<'a, P: Parser<&'a [u8]>>(
    inner_parse: &mut P,
) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], Option<P::Output>, P::Error>
where
    Err<P::Error>: From<Err<nom::error::Error<&'a [u8]>>>,
{
    move |input: &'a [u8]| {
        let (remaining, present) = parse_bool(input).map_err(|err| match err {
            Err::Error(error) => Err::Error(error.into()),
            Err::Failure(error) => Err::Error(error.into()),
            any => any,
        })?;
        if !present {
            return Ok((remaining, None));
        }
        inner_parse
            .parse(remaining)
            .map(|(bytes, val)| (bytes, Some(val)))
    }
}

pub fn parse_array<'a, O, E, F: Clone + Fn(&'a [u8]) -> Result<(&[u8], O), nom::Err<E>>>(
    inner_parse: F,
) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], Vec<O>, E>
where
    E: From<nom::error::Error<&'a [u8]>>,
{
    move |input: &'a [u8]| {
        let (remaining, length) = parse_varint(input).map_err(nom::Err::convert)?;
        let mut vec = Vec::new();
        let mut remaining = remaining;
        for _ in 0..length as usize {
            let (data, value) = inner_parse.clone()(remaining)?;
            remaining = data;
            vec.push(value);
        }
        Ok((remaining, vec))
    }
}

pub fn generate_varint<W: Write>(n: i32) -> impl SerializeFn<W> {
    move |mut w| {
        let mut value = n as u32;
        loop {
            if (value & !0x7F) == 0 {
                w.write_all(&[value as u8][..])?;
                return Ok(w);
            }
            let byte = (value & 0x7F) as u8 | 0x80;
            w.write_all(&[byte][..])?;
            value >>= 7;
        }
    }
}

pub fn length_prefixed<W: Write, F: SerializeFn<Vec<u8>>>(f: F) -> impl SerializeFn<W> {
    move |w| {
        let wrapped_data = gen_simple(&f, Vec::new())?;
        let mut w = gen_simple(generate_varint(wrapped_data.len() as i32), w)?;
        w.write_all(&wrapped_data)?;
        Ok(w)
    }
}

pub fn generate_string<W: Write>(string: &str) -> impl SerializeFn<W> {
    move |w| {
        let mut w = generate_varint(string.len() as i32)(w)?;
        w.write_all(string.as_bytes())?;
        Ok(w)
    }
}

pub fn generate_optional<W: Write, T, F: Fn(&T) -> S, S: SerializeFn<W>>(
    value: &Option<T>,
    f: F,
) -> impl SerializeFn<W> {
    move |mut w| {
        let w = match value {
            None => {
                w.write_all(&[0][..])?;
                w
            }
            Some(v) => {
                w.write_all(&[1][..])?;
                f(v)(w)?
            }
        };
        Ok(w)
    }
}

pub fn generate_array<'a, W: Write, T, F: Fn(&'a T) -> S, S: SerializeFn<W>>(
    values: &'a [T],
    f: F,
) -> impl SerializeFn<W> {
    move |w| {
        let mut w = generate_varint(values.len() as i32)(w)?;
        for item in values {
            w = f(item)(w)?
        }
        Ok(w)
    }
}
