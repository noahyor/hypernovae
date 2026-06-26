use serde_json::{Map, Value};

use crate::game::Identifier;

pub struct Datapack {
    identifier: Identifier,
    version: String,
    data: Map<String, Value>,
}
