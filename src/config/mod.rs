use serde::{Deserialize, Serialize};
use std::{error::Error, fs::File, io::Read};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub address: [u8; 4],
    pub port: u16,
}

impl Config {
    pub fn read(path: String) -> Result<Config, Box<dyn Error>> {
        let mut file = File::options().read(true).open(path)?;
        let mut string = String::new();
        file.read_to_string(&mut string)?;
        Ok(toml::from_str(&string)?)
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            address: [127, 0, 0, 1],
            port: 25565_u16,
        }
    }
}
