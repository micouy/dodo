use std::io;

pub type Result<T> = std::result::Result<T, Error>;

// --- ERROR ---

#[derive(Debug)]
pub enum Error {
    TOML(toml::de::Error),
    IO(io::Error),
    Formatting(FmtError),
    Unreachable(String),
    ConfigNotFound,
    InvalidFilePath,
    EmptyCommand,
    DependencyCycle,
    DuplicateOutput,
}

// --- UNREACHABLE ERROR ---

#[derive(Debug)]
pub enum Unreachable {}

// --- FORMATTING ERROR ---

#[derive(Debug)]
pub enum FmtError {
    EmptyBrackets,
    InvalidVar(String),
}

impl From<dynfmt::Error<'_>> for Error {
    fn from(err: dynfmt::Error) -> Self {
        use dynfmt::{Error::*, Position::*};
        use Error::*;
        use FmtError::*;

        match err {
            BadFormat(..) => Unreachable(
                "`BadFormat` thrown by curly formatter.".to_string(),
            ),
            BadData(..) =>
                Unreachable("`BadData` thrown by curly formatter.".to_string()),
            BadArg(..) =>
                Unreachable("`BadArg` thrown by curly formatter.".to_string()),
            Parse(..) =>
                Unreachable("`Parse` thrown by curly formatter.".to_string()),
            MapRequired => Unreachable(
                "`MapRequired` thrown while parsing from named access."
                    .to_string(),
            ),
            ListRequired => Formatting(EmptyBrackets),
            MissingArg(Key(s)) => Formatting(InvalidVar(s.to_string())),
            MissingArg(..) => Unreachable(
                "Missing indexed arg while parsing from named access."
                    .to_string(),
            ),
            Io(err) => IO(err),
        }
    }
}
