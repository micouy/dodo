use std::{
    collections::HashMap,
    convert::AsRef,
    env,
    fs,
    hash::{Hash, Hasher},
    io,
    path::{Path, PathBuf},
    string::ToString,
};

use ansi_term::Colour::*;
use serde::*;
use toml;

mod error;
mod target;
mod util;

use error::{Error, Result};
use target::{Config, Target};
use util::{cwd, get_file_hash, parse_command, read_config};

fn main() -> Result<()> {
    let args = std::env::args().skip(1).next();
    let dodo = read_config(args)?;
    let dodo = toml::from_str::<Config>(&dodo).map_err(Error::TOML)?;

    print_targets(&dodo.targets)?;

    Ok(())
}

fn print_targets(targets: &[Target]) -> Result<()> {
    for target in targets {
        println!("{}: {:?}", Green.paint("OUTPUT"), target.output);

        println!(
            "{}: {}",
            Green.paint("WORKING DIR"),
            target
                .working_dir()
                .unwrap_or(".".as_ref())
                .to_string_lossy()
        );

        let hash = get_file_hash(&target.output)
            .map(|h| format!("{:x}", h))
            .unwrap_or("file not present".into());
        println!("{}: {}", Green.paint("HASH"), hash);

        let mut vars = HashMap::new();

        let target_filename = target
            .output
            .file_name()
            .ok_or_else(|| Error::InvalidFilePath)?
            .to_str()
            .ok_or(Error::Unreachable(
                "Conversion of OsStr to &str failed".to_string(),
            ))?; // TOML uses UTF-8 so the conversion won't fail
        let target_filename: ansi_term::ANSIGenericString<str> =
            Fixed(14).paint(target_filename);
        let target_filename = target_filename.to_string();
        vars.insert("target_filename", target_filename);
        let vars = vars; // demut

        println!("{}:", Green.paint("COMMANDS"));
        target
            .tasks
            .iter()
            .map(|task| {
                let cmd = parse_command(task.command(), &vars).unwrap();
                let dir = task
                    .working_dir()
                    .map(|dir| dir.to_string_lossy())
                    .map(|dir| Fixed(242).paint(format!("# in {}", dir)));

                (cmd, dir)
            })
            .for_each(|(cmd, dir)| match dir {
                Some(dir) => println!("$ {} {}", cmd, dir),
                None => println!("$ {}", cmd),
            });

        println!("");
    }

    Ok(())
}
