use std::{
    collections::HashMap,
    convert::AsRef,
    env,
    fs,
    hash::{Hash, Hasher},
    iter,
    path::{Path, PathBuf},
    string::ToString,
};

use crate::{
    error::{Error, Result},
    target::{Target, TaskContext},
};

use ansi_term::Colour::*;
use dynfmt::{Format, FormatArgs, SimpleCurlyFormat as Formatter};
use rustc_hash::FxHasher;
use serde::*;

pub fn format_arg(arg: &str, context: impl FormatArgs) -> Result<String> {
    Formatter
        .format(arg, context)
        .map_err(Into::into)
        .map(|cow| cow.to_string())
}

pub fn get_file_hash(path: impl AsRef<Path>) -> Result<u64> {
    let mut hasher: FxHasher = FxHasher::default();

    fs::read(path.as_ref()).map_err(Error::IO).map(|content| {
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
        println!(
            "{}: {}",
            Green.paint("OUTPUT"),
            target.output.to_string_lossy()
        );

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

        let target_filename = target
            .output
            .file_name()
            .ok_or_else(|| Error::InvalidFilePath)?
            .to_str()
            .ok_or(Error::Unreachable(
                "Conversion of OsStr to &str failed".to_string(),
            ))?; // TOML uses UTF-8 so the conversion won't fail
        let target_filename = Fixed(14).paint(target_filename).to_string();
        let context = TaskContext { target_filename };

        println!("{}:", Green.paint("COMMANDS"));
        target
            .tasks
            .iter()
            .map(|task| {
                let command = Fixed(3).paint(&task.command).to_string();
                let mut args = task.format_args(&context).unwrap();
                args.insert(0, command);
                let line = args.join(" ");
                let mb_dir = task
                    .working_dir()
                    .map(|dir| dir.to_string_lossy())
                    .map(|dir| Fixed(242).paint(format!("# in {}", dir)));

                (line, mb_dir)
            })
            .for_each(|(line, mb_dir)| match mb_dir {
                Some(dir) => println!("$ {} {}", line, dir),
                None => println!("$ {}", line),
            });

        println!("");
    }

    Ok(())
}

pub fn run_targets(targets: &[Target]) -> Result<()> {
    for target in targets {
        let current_dir = env::current_dir().map_err(Error::IO)?;
        let working_dir = target.working_dir.clone().unwrap_or(current_dir);

        let target_filename = target
            .output
            .file_name()
            .ok_or_else(|| Error::InvalidFilePath)?
            .to_str()
            .ok_or(Error::Unreachable(
                "Conversion of OsStr to &str failed".to_string(), // TOML uses UTF-8 so the conversion won't fail
            ))?
            .to_string();
        let context = TaskContext { target_filename };

        target
            .tasks
            .iter()
            .map(|task| task.run(working_dir.clone(), &context).map(|_| ()))
            .collect::<Result<()>>()?;
    }

    Ok(())
}
