use std::{
    collections::HashMap,
    convert::{identity, AsRef},
    env,
    hash::Hash,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

use crate::{
    error::{Error, Result},
    util::format_arg,
};

use dynfmt::FormatArgs;
use serde::*;

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
    // reject commands containing spaces with `and_then` when the new serde is
    // out?
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    working_dir: Option<PathBuf>,
}

#[derive(Clone)]
pub struct TaskContext {
    pub target_filename: String,
}

impl FormatArgs for TaskContext {
    fn get_key(
        &self,
        key: &str,
    ) -> std::result::Result<Option<dynfmt::Argument>, ()> {
        match key {
            "target_filename" => Ok(Some(&self.target_filename)),
            _ => Ok(None),
        }
    }
}

impl Task {
    pub fn working_dir(&self) -> Option<&Path> {
        self.working_dir.as_ref().map(|d| d.as_ref())
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn format_args(&self, context: impl FormatArgs) -> Result<Vec<String>> {
        self.args
            .iter()
            .map(|arg| format_arg(arg, &context).map_err(Error::from))
            .collect()
    }

    pub fn run(
        &self,
        target_working_dir: PathBuf,
        context: impl FormatArgs,
    ) -> Result<ExitStatus> {
        let working_dir = self
            .working_dir()
            .map(|subdir| target_working_dir.join(subdir))
            .unwrap_or(target_working_dir);

        let args = self.format_args(context)?;

        println!("executing: {} {}", self.command, args.clone().join(" "));

        Command::new(self.command.clone())
            .current_dir(working_dir)
            .args(&args)
            .spawn()
            .map(|mut child| child.wait())
            .and_then(identity) // smart flatten ;))))
            .map_err(Error::IO)
    }
}
