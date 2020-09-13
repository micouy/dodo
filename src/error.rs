use std::io;

pub type Result<T> = std::result::Result<T, Error>;

// --- ERROR ---

#[derive(Debug)]
pub enum Error {
    TOML(toml::de::Error),
    IO(io::Error),
    Formatting(FmtError),
    Unreachable(Unreachable),
    ConfigNotFound,
    InvalidFilePath,
    EmptyCommand,
    DependencyCycle,
    DuplicateOutput,
}

// --- UNREACHABLE ERROR ---

#[derive(Debug)]
pub enum Unreachable {
    FmtBadFormat,
    FmtBadArg,
    FmtBadData,
    FmtParse,
    FmtMapRequired,
    FmtMissingArg,
    OsStrConversion,
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

        match err {
            ListRequired => Error::Formatting(EmptyBrackets),
            MissingArg(Key(s)) => Error::Formatting(InvalidVar(s.to_string())),
            Io(err) => Error::IO(err),

            BadFormat(..) => Error::Unreachable(Unreachable::FmtBadFormat),
            BadData(..) => Error::Unreachable(Unreachable::FmtBadData),
            BadArg(..) => Error::Unreachable(Unreachable::FmtBadArg),
            Parse(..) => Error::Unreachable(Unreachable::FmtParse),
            MapRequired => Error::Unreachable(Unreachable::FmtMapRequired),
            MissingArg(..) => Error::Unreachable(Unreachable::FmtMissingArg),
        }
    }
}
