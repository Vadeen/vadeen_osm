use std::error;
use std::fmt::{Display, Formatter};
use std::io;

pub type Result<T> = std::result::Result<T, ErrorKind>;

/// Represents errors that may occur when reading or writing osm.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    position: Option<u32>, // Byte position for binary files.
    line: Option<u32>,     // Line for text files.
}

/// TODO don't expose internal error in this way.
/// It will make it possible to change internals without breaking change.
#[derive(Debug)]
pub enum ErrorKind {
    QuickXml(quick_xml::Error),
    IO(io::Error),
    InvalidData(String),
}

impl Error {
    pub fn new(kind: ErrorKind, position: Option<u32>, line: Option<u32>) -> Self {
        Error {
            kind,
            position,
            line,
        }
    }

    /// Returns byte position where error occurred if available.
    pub fn position(&self) -> Option<u32> {
        self.position
    }

    /// Returns on which line error occurred if available.
    pub fn line(&self) -> Option<u32> {
        self.line
    }

    /// Returns reference to error kind.
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        if self.line.is_some() {
            writeln!(f, "Parse error at line {}", self.line.unwrap())?;
        } else if self.position.is_some() {
            writeln!(f, "Parse error at position {}", self.position.unwrap())?;
        }
        ErrorKind::fmt(&self.kind, f)?;
        Ok(())
    }
}

impl ErrorKind {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ErrorKind::QuickXml(e) => Some(e),
            ErrorKind::IO(e) => Some(e),
            ErrorKind::InvalidData(_) => None,
        }
    }
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            ErrorKind::QuickXml(e) => write!(f, "XML error: {}", e),
            ErrorKind::IO(e) => write!(f, "IO error: {}", e),
            ErrorKind::InvalidData(s) => write!(f, "Invalid data: {}", s),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.kind.source()
    }
}

impl From<io::Error> for ErrorKind {
    fn from(e: io::Error) -> Self {
        ErrorKind::IO(e)
    }
}

impl From<quick_xml::Error> for ErrorKind {
    fn from(e: quick_xml::Error) -> Self {
        ErrorKind::QuickXml(e)
    }
}
