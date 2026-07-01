use std::io::Write;

use cookie_factory::SerializeFn;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct BlockPosition {
    x: i32,
    z: i16,
    y: i32,
}

// #[derive(Serialize, Deserialize)]
// pub struct Advancement {
//     display: AdvancementDisplay,
//     criteria: Vec<AdvancementCriteria>,
//     requirements: Option<Vec<Vec<String>>>,
//     rewards: Option<AdvancementRewards>,
//     #[serde(default)]
//     sends_telemetry_event: bool,
// }
//
// #[derive(Serialize, Deserialize)]
// pub struct AdvancementDisplay {
//     icon: AdvancementIcon,
//     title: JsonTextComponent,
//     description: JsonTextComponent,
//     frame: AdvancementFrame,
//     #[serde(default = "always_true")]
//     show_toast: bool,
//     #[serde(default = "always_true")]
//     announce_to_chat: bool,
//     #[serde(default)]
//     hidden: bool,
// }
//
// #[derive(Serialize, Deserialize)]
// pub struct AdvancementIcon {
//     id: String,
//     #[serde(default = "one")]
//     count: i32,
//     components: serde_json::Map<String, serde_json::Value>,
// }

const fn always_true() -> bool {
    true
}

const fn one() -> i32 {
    1
}

impl BlockPosition {
    pub(crate) fn generate<W: Write>(&self) -> impl SerializeFn<W> {
        |w| {
            cookie_factory::bytes::be_u64(
                (self.x as u64) << 38
                    | ((self.z as u64) & 0x3FFFFFF) << 12
                    | (self.y as u64) & 0xFFF,
            )(w)
        }
    }
}
