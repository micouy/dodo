use std::{
    collections::HashMap,
    convert::AsRef,
    env,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use dynfmt::{Format, SimpleCurlyFormat as Formatter};
use rustc_hash::FxHasher;
use serde::*;

use crate::error::{Error, Result};

pub fn parse_command<K, V>(
    command: &str,
    vars: &HashMap<K, V>,
) -> Result<String>
where
    K: std::borrow::Borrow<str> + Hash + Eq,
    V: Serialize,
{
    Formatter
        .format(command, vars)
        .map_err(Into::into)
        .map(|cow| cow.to_string())
}

pub fn get_file_hash(path: impl AsRef<Path>) -> Option<u64> {
    let mut hasher: FxHasher = FxHasher::default();

    fs::read(path.as_ref()).ok().map(|content| {
        content.hash(&mut hasher);

        hasher.finish()
    })
}

pub fn cwd() -> Result<PathBuf> {
    env::current_dir().map_err(Error::IO)
}

pub fn read_config<P>(file: Option<P>) -> Result<String>
where
    P: AsRef<Path>,
{
    let file = file
        .as_ref()
        .map(|f| f.as_ref())
        .unwrap_or("dodo.toml".as_ref());
    fs::read_to_string(file).map_err(|e| {
        use std::io::ErrorKind::*;

        match e.kind() {
            NotFound => Error::ConfigNotFound,
            _ => Error::IO(e),
        }
    })
}
