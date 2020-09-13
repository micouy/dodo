use std::{
    convert::AsRef,
    env,
    fs,
    hash::{Hash, Hasher as _},
    path::Path,
    string::ToString,
};

use crate::{
    error::{Error, Result, Unreachable},
    target::{Target, TaskContext},
};

use ansi_term::Colour::*;
use dynfmt::{Format, FormatArgs, SimpleCurlyFormat as Formatter};
use twox_hash::XxHash64 as Hasher;

pub fn format_arg(arg: &str, context: impl FormatArgs) -> Result<String> {
    Formatter
        .format(arg, context)
        .map_err(Into::into)
        .map(|cow| cow.to_string())
}

pub fn get_file_hash(path: impl AsRef<Path>) -> Result<u64> {
    let mut hasher = Hasher::default();

    fs::read(path.as_ref()).map_err(Error::IO).map(|content| {
        content.hash(&mut hasher);

        hasher.finish()
    })
}

pub fn read_config<P>(file: Option<P>) -> Result<String>
where
    P: AsRef<Path>,
{
    let file = file
        .as_ref()
        .map(|f| f.as_ref())
        .unwrap_or_else(|| "dodo.toml".as_ref());
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
                .unwrap_or_else(|| ".".as_ref())
                .to_string_lossy()
        );

        let hash = get_file_hash(&target.output)
            .map(|h| format!("{:x}", h))
            .unwrap_or_else(|_| "file not present".into());
        println!("{}: {}", Green.paint("HASH"), hash);

        let target_filename = target
            .output
            .file_name()
            .ok_or(Error::InvalidFilePath)?
            .to_str()
            .ok_or_else(|| Error::Unreachable(Unreachable::OsStrConversion))?; // TOML uses UTF-8 so the conversion won't fail
        let target_filename = Fixed(14).paint(target_filename).to_string();
        let context = TaskContext { target_filename };

        println!("{}:", Green.paint("COMMANDS"));
        target
            .tasks
            .iter()
            .map(|task| {
                let (command, mut args) =
                    task.format_command(&context).unwrap();
                let command = Fixed(3).paint(&command).to_string();
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

        println!();
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
            .ok_or(Error::InvalidFilePath)?
            .to_str()
            .ok_or_else(|| Error::Unreachable(Unreachable::OsStrConversion))?
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
