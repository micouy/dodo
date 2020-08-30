use serde::{
    de::{SeqAccess, Visitor},
    *,
};
use std::{
    collections::HashMap as Map,
    error::Error,
    fmt,
    path::{Path, PathBuf},
    str::FromStr,
};

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    // env vars?
    pub targets: Vec<Target>,
}

// waiting for https://github.com/serde-rs/serde/issues/939
// to add validation
#[derive(Deserialize, Serialize, Debug)]
pub struct Target {
    pub output: PathBuf, // handle multiple outputs?
    pub tasks: Vec<Task>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deps: Vec<PathBuf>,
}

impl Target {
    pub fn working_dir(&self) -> Option<&Path> {
        self.working_dir.as_ref().map(|d| d.as_ref())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Task {
    command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    working_dir: Option<PathBuf>,
}

impl Task {
    pub fn working_dir(&self) -> Option<&Path> {
        self.working_dir.as_ref().map(|d| d.as_ref())
    }

    pub fn command(&self) -> &str {
        &self.command
    }
}
