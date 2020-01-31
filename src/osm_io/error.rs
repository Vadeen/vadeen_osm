use crate::osm_io::error::ErrorKind::IO;
use crate::osm_io::error::Repr::Simple;
use std::fmt::{Display, Formatter};
use std::io;

pub type Result<T> = std::result::Result<T, Error>;

/// Represents errors that may occur when reading or writing osm.
#[derive(Debug)]
pub struct Error {
    repr: Repr,
    message: Option<String>,
}

/// It will make it possible to change internals without breaking change.
#[derive(Debug)]
enum Repr {
    Simple(ErrorKind),
}

#[derive(Debug)]
pub enum ErrorKind {
    /// File format error. E.g. unsupported or unrecognized osm format.
    FileFormat,

    /// Parse error. E.g. invalid data in file.
    Parse,

    /// IO error. E.g. file not found, permission denied.
    IO(io::Error),
}

impl Error {
    pub fn new(kind: ErrorKind, message: Option<String>) -> Self {
        Error {
            repr: Simple(kind),
            message,
        }
    }

    pub fn message(&self) -> Option<&String> {
        self.message.as_ref()
    }

    pub fn set_message(&mut self, message: String) {
        self.message = Some(message);
    }

    /// Returns reference to error kind.
    pub fn kind(&self) -> &ErrorKind {
        match &self.repr {
            Simple(e) => &e,
        }
    }
}

impl ErrorKind {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            IO(e) => Some(e),
            _ => None,
        }
    }
}

impl Repr {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Simple(kind) => kind.source(),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.repr.source()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match &self.message {
            Some(message) => write!(f, "{}", message),
            None => write!(f, "Unknown error occurred."),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error {
            repr: Simple(IO(e)),
            message: None,
        }
    }
}
