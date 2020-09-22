use std::{io, path::PathBuf};

pub type Result<T> = std::result::Result<T, Error>;

// --- ERROR ---

#[derive(Debug)]
pub enum Error {
    UserError(UserError),
    TOML(toml::de::Error),
    IO(io::Error),
    Formatting(FmtError),
    Internal { line: u32, file: &'static str },
    Other,
}

impl Error {
    pub fn internal(line: u32, file: &'static str) -> Self {
        Error::Internal { line, file }
    }
}

// --- USER ERROR ---

#[derive(Debug)]
pub enum UserError {
    EmptyCommand,
    EmptyTargetIdentifier,
    DependencyCycle,
    DuplicateTarget,
    ConfigNotFound,
    NoSuchTarget(PathBuf),
}

impl From<UserError> for Error {
    fn from(inner: UserError) -> Self {
        Error::UserError(inner)
    }
}

// --- FORMATTING ERROR ---

#[derive(Debug)]
pub enum FmtError {
    EmptyBrackets,
    InvalidVar(String),
}

impl From<dynfmt::Error<'_>> for Error {
    fn from(err: dynfmt::Error) -> Self {
        use dynfmt::{Error::*, Position::*};
        use FmtError::*;
        // TODO check which errors can't occur (`Internal`)
        // and which can only be caused by the user (`UserError`)

        match err {
            ListRequired => Error::Formatting(EmptyBrackets),
            MissingArg(Key(s)) => Error::Formatting(InvalidVar(s.to_string())),
            Io(err) => Error::IO(err),
            _ => Error::Other,
        }
    }
}
