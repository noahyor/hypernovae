use std::collections::HashMap;

use nom::{
    AsChar, IResult, Parser, branch::alt, bytes::streaming::*, character::streaming::multispace0,
    error::ParseError, sequence::terminated,
};

pub enum NBT {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    String(String),
    Array(Vec<NBT>, Option<NBTType>),
    Compound(HashMap<String, NBT>),
}

pub enum NBTType {}

impl NBT {
    pub fn parse(input: &[u8]) -> IResult<&[u8], NBT> {
        alt((
            compound, string, array, byte, short, long, float, double, int,
        ))
        .parse(input)
    }
}

fn compound(input: &[u8]) -> IResult<&[u8], NBT> {
    todo!()
}

// fn compound(input: &[u8]) -> IResult<&[u8], NBT> {
//     delimited(
//         ws(tag("{")),
//         ws(separated_list0(tag(","), key_val_pair)),
//         ws(tag("}"))
//     ).parse(input).map(|(bytes, val)| (bytes, NBT::Compound(val.into_iter().collect())))
// }

// fn key_val_pair(input: &[u8]) -> IResult<&[u8], (String, NBT)> {
//     (map_parser(take_until1(":"), alt((delimited(quote, , quote), many(1.., alt(()))))), NBT::parse).parse(input)
// }
//
// fn limited_string(input: &[u8]) -> IResult<&[u8], String> {
//     if input.len() < 1 {return Err(nom::Err::Incomplete(nom::Needed::Unknown))}
//     let mut accum = Vec::new();
//     let first = input[0];
//     let fold_first = first | (1 << 5);
//     if ((fold_first >= b'a') & (fold_first <= b'z'))
//     | (first == 43) | (first == 45) | (first == 46) | (first == 95) {
//         accum.push(first)
//     } else {
//         return Err(nom::Err::Error(()))
//     }
//     for byte in input[1..] {
//
//     }
// }

fn string(input: &[u8]) -> IResult<&[u8], NBT> {
    todo!()
}

fn array(input: &[u8]) -> IResult<&[u8], NBT> {
    todo!()
}

fn byte(input: &[u8]) -> IResult<&[u8], NBT> {
    todo!()
}

fn boolean(input: &[u8]) -> IResult<&[u8], NBT> {
    todo!()
}

fn short(input: &[u8]) -> IResult<&[u8], NBT> {
    todo!()
}

fn long(input: &[u8]) -> IResult<&[u8], NBT> {
    todo!()
}

fn float(input: &[u8]) -> IResult<&[u8], NBT> {
    todo!()
}

fn double(input: &[u8]) -> IResult<&[u8], NBT> {
    todo!()
}

fn int(input: &[u8]) -> IResult<&[u8], NBT> {
    todo!()
}

fn quote(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((tag("\""), tag("'"))).parse(input)
}

fn ws<I: AsChar + nom::Input, O, E: ParseError<I>, F>(
    inner: F,
) -> impl Parser<I, Output = O, Error = E>
where
    F: Parser<I, Output = O, Error = E>,
    <I as nom::Input>::Item: AsChar,
{
    terminated(inner, multispace0)
}
