use std::{
    collections::HashMap,
    convert::AsRef,
    env,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    string::ToString,
};

use crate::{
    error::{Error, Result},
    target::Target,
};

use ansi_term::Colour::*;
use dynfmt::{Format, SimpleCurlyFormat as Formatter};
use rustc_hash::FxHasher;
use serde::*;

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

pub fn print_targets(targets: &[Target]) -> Result<()> {
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
