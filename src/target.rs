use std::{
    convert::AsRef,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

use crate::{
    error::{Error, Result, UserError},
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
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Target {
    #[serde(rename = "target")]
    pub identifier: PathBuf, // handle multiple outputs?
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

#[derive(Debug, Deserialize, Serialize, Clone, Eq, PartialEq)]
pub struct Task {
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<PathBuf>,
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

    pub fn format_command(
        &self,
        context: impl FormatArgs,
    ) -> Result<(String, Vec<String>)> {
        let mut parts = self.command.split(' ').filter(|s| !s.is_empty());
        let command = parts.nth(0).ok_or(UserError::EmptyCommand)?.to_string();
        let args = parts
            .map(|arg| format_arg(arg, &context).map_err(Error::from))
            .collect::<Result<_>>()?;

        Ok((command, args))
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

        let (command, args) = self.format_command(context)?;

        println!("executing: {} {}", command, args.join(" "));

        Command::new(command)
            .current_dir(working_dir)
            .args(&args)
            .spawn()
            .map(|mut child| child.wait())
            .flatten()
            .map_err(Error::IO)
    }
}
