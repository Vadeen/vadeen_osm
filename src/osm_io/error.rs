use std::fmt::{Display, Formatter};
use std::io;
use crate::osm_io::error::Repr::Simple;
use crate::osm_io::error::ErrorKind::{IO, ParseError};

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
    ParseError,
    IO(io::Error),
}

impl Error {
    pub fn new(kind: ErrorKind, message: Option<String>) -> Self {
        Error {
            repr: Simple(kind),
            message
        }
    }

    /// Returns reference to error kind.
    pub fn kind(&self) -> &ErrorKind {
        match &self.repr {
            Simple(e) => &e,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        /*
        if self.line.is_some() {
            writeln!(f, "Parse error at line {}", self.line.unwrap())?;
        } else if self.position.is_some() {
            writeln!(f, "Parse error at position {}", self.position.unwrap())?;
        }
        ErrorKind::fmt(&self.kind, f)?;
        // TODO
        */
        Ok(())
    }
}

/*
impl ErrorKind {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ErrorKind::QuickXml(e) => Some(e),
            ErrorKind::IO(e) => Some(e),
            ErrorKind::InvalidData(_) => None,
        }
    }
}*/

/*
impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            ErrorKind::QuickXml(e) => write!(f, "XML error: {}", e),
            ErrorKind::IO(e) => write!(f, "IO error: {}", e),
            ErrorKind::InvalidData(s) => write!(f, "Invalid data: {}", s),
        }
    }
}*/

/*
impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.kind.source()
    }
}
*/

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error {
            repr: Simple(IO(e)),
            message: None
        }
    }
}

impl From<quick_xml::Error> for Error {
    fn from(e: quick_xml::Error) -> Self {
        Error {
            repr: Simple(ParseError),
            message: Some("".to_owned()) // TODO xml error message
        }
//        ErrorKind::QuickXml(e)
    }
}
