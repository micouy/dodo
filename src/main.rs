use std::{
    collections::HashMap,
    convert::AsRef,
    env,
    fs,
    hash::{Hash, Hasher},
    io,
    path::{Path, PathBuf},
};

use ansi_term::Colour::*;
use serde::*;
use toml;

mod error;
mod target;
mod util;

use error::{Error, Result};
use target::{Command, TargetOpts};
use util::{cwd, get_file_hash, parse_command, read_config};

fn main() -> Result<()> {
    let dodo = read_config()?;
    let mut dodo = toml::from_str::<HashMap<PathBuf, TargetOpts>>(&dodo)
        .map_err(Error::TOML)?;

    print_targets(&dodo);

    Ok(())
}

fn print_targets(targets: &HashMap<PathBuf, TargetOpts>) -> Result<()> {
    let cwd = cwd()?;

    for (target, opts) in targets {
        println!("{}: {}", Green.paint("TARGET"), target.to_string_lossy());

        let working_dir: &Path = opts
            .working_dir
            .as_ref()
            .map(|p| p.as_ref())
            .unwrap_or(".".as_ref());
        println!(
            "{}: {}",
            Green.paint("WORKING DIR"),
            working_dir.to_string_lossy()
        );

        let hash = get_file_hash(&target)
            .map(|h| format!("{:x}", h))
            .unwrap_or("no file".to_string());
        println!("{}: {}", Green.paint("HASH"), hash);

        let mut vars = HashMap::new();
        let target_filename = target
            .file_name()
            .ok_or_else(|| Error::InvalidFilePath)?
            .to_str()
            .ok_or(Error::Unreachable(
                "Conversion of OsStr to &str failed".to_string(),
            ))?; // TOML uses UTF-8 so the conversion won't fail
        let target_filename = Fixed(14).paint(target_filename).to_string();
        vars.insert("target_filename", target_filename);
        let vars = vars; // demut

        println!("{}:", Green.paint("COMMANDS"));
        let commands = opts
            .commands
            .iter()
            .filter_map(|cmd| match cmd {
                Command::Plain(cmd) => parse_command(cmd, &vars).ok(),
                Command::Struct {
                    command: cmd,
                    working_dir: None,
                } => parse_command(cmd, &vars).ok(),
                Command::Struct {
                    command: cmd,
                    working_dir: Some(dir),
                } => parse_command(cmd, &vars).ok().map(|cmd| {
                    format!(
                        "{} {}",
                        cmd,
                        Fixed(242)
                            .paint(format!("# in {}", dir.to_string_lossy()))
                    )
                }),
            })
            .for_each(|cmd| println!("$ {}", cmd));

        println!("");
    }

    Ok(())
}
