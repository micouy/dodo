use std::{
    convert::AsRef,
    env,
    fs,
    hash::{Hash, Hasher as _},
    iter::once,
    path::Path,
    result::Result as StdResult,
    string::ToString,
};

use crate::{
    error::{Error, Result, UserError},
    target::{Target, TaskContext},
};

use ansi_term::Colour::*;
use dynfmt::{Format, FormatArgs, SimpleCurlyFormat as Formatter};
use twox_hash::XxHash64 as Hasher;

pub fn format_arg(arg: &str, context: impl FormatArgs) -> Result<String> {
    Formatter
        .format(arg, context)
        .map_err(|_| Error::internal(line!(), file!()))
        .map(|cow| cow.to_string())
}

pub fn get_file_hash(path: impl AsRef<Path>) -> Result<u64> {
    let mut hasher = Hasher::default();

    fs::read(path.as_ref()).map_err(Error::IO).map(|content| {
        content.hash(&mut hasher);

        hasher.finish()
    })
}

pub fn read_config<P>(file: P) -> Result<String>
where
    P: AsRef<Path>,
{
    fs::read_to_string(file).map_err(|e| {
        use std::io::ErrorKind::*;

        match e.kind() {
            NotFound => UserError::ConfigNotFound.into(),
            _ => Error::IO(e),
        }
    })
}

pub fn print_targets(targets: &[Target]) -> Result<()> {
    for target in targets {
        println!(
            "{}: {}",
            Green.paint("OUTPUT"),
            target.identifier.to_string_lossy()
        );

        println!(
            "{}: {}",
            Green.paint("WORKING DIR"),
            target
                .working_dir()
                .unwrap_or_else(|| ".".as_ref())
                .to_string_lossy()
        );

        let hash = get_file_hash(&target.identifier)
            .map(|h| format!("{:x}", h))
            .unwrap_or_else(|_| "file not present".into());
        println!("{}: {}", Green.paint("HASH"), hash);

        let target_filename = target
            .identifier
            .file_name()
            .ok_or(UserError::EmptyTargetIdentifier)?
            .to_str()
            .ok_or_else(|| Error::internal(line!(), file!()))?; // TOML uses UTF-8 so the conversion won't fail
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

pub fn run_targets(targets: Vec<Target>) -> Result<()> {
    let targets_with_contexts = targets
        .into_iter()
        .map(|target| {
            let current_dir = env::current_dir().map_err(Error::IO)?;
            let working_dir = target.working_dir.clone().unwrap_or(current_dir);

            let target_filename = target
                .identifier
                .file_name()
                .ok_or(UserError::EmptyTargetIdentifier)?
                .to_str()
                .ok_or_else(|| Error::internal(line!(), file!()))?
                .to_string();
            let context = TaskContext { target_filename };

            Ok((target, context, working_dir))
        })
        .collect::<Result<Vec<_>>>()?;

    targets_with_contexts
        .clone()
        .into_iter()
        .map(|(target, context, _)| {
            target
                .tasks
                .into_iter()
                .map(move |task| task.format_command(&context))
        })
        .flatten()
        .map_item(|(command, args)| {
            let text = once(command).chain(args).collect::<Vec<_>>().join(" ");
            println!("{}", text);
        })
        .collect::<Result<()>>()?;

    targets_with_contexts
        .into_iter()
        .map(|(target, context, working_dir)| {
            target
                .tasks
                .into_iter()
                .map(move |task| task.run(working_dir.clone(), &context))
                .map_item(|_| ())
        })
        .flatten()
        .collect::<Result<_>>()?;

    Ok(())
}

pub struct MapOk<I, O> {
    inner: I,
    op: O,
}

impl<I, T, E, O, U> Iterator for MapOk<I, O>
where
    I: Iterator<Item = StdResult<T, E>>,
    O: FnMut(T) -> U,
{
    type Item = StdResult<U, E>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|result| result.map(&mut self.op))
    }
}

pub struct MapErr<I, O> {
    inner: I,
    op: O,
}

impl<I, T, E, O, F> Iterator for MapErr<I, O>
where
    I: Iterator<Item = StdResult<T, E>>,
    O: FnMut(E) -> F,
{
    type Item = StdResult<T, F>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|result| result.map_err(&mut self.op))
    }
}

// TODO rename the methods
pub trait ResultIterator<T, E>:
    Iterator<Item = StdResult<T, E>> + Sized
{
    fn map_item<O, U>(self, op: O) -> MapOk<Self, O>
    where
        O: FnMut(T) -> U,
    {
        MapOk { inner: self, op }
    }

    fn map_item_err<O, F>(self, op: O) -> MapErr<Self, O>
    where
        O: FnMut(E) -> F,
    {
        MapErr { inner: self, op }
    }
}

impl<I, T, E> ResultIterator<T, E> for I where
    I: Iterator<Item = StdResult<T, E>>
{
}
