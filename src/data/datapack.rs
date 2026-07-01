use crate::{
    data::Identifier,
    error::Error,
    // game::data::Advancement,
    net::data::{generate_string, parse_string},
};
use cookie_factory::{SerializeFn, gen_simple};
use nom::IResult;
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap},
    fs::FileType,
    io::Write,
    path::PathBuf,
};

pub struct Datapack {
    identifier: Identifier,
    version: String,
    namespaces: BTreeMap<Identifier, DatapackNamespace>,
}

pub struct DatapackNamespace {
    // advancement: BTreeMap<String, indextree::Arena<Advancement>>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct DatapackVersion {
    identifier: Identifier,
    version: String,
}

impl Datapack {
    pub fn from_dir(
        path: PathBuf,
        identifier: Identifier,
        version: String,
    ) -> Result<Self, Error<Vec<u8>>> {
        assert!(path.join("pack.mcmeta").exists());
        let root_path = path.join("data/");
        let mut namespaces = BTreeMap::new();
        for namespace in std::fs::read_dir(root_path)?
            .collect::<Result<Vec<std::fs::DirEntry>, std::io::Error>>()?
            .iter()
        {
            if namespace.file_type()?.is_file() {
                continue;
            }
        }
        Ok(Self {
            identifier,
            version,
            namespaces,
        })
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
