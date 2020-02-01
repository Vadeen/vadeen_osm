//! IO functionality for OSM maps.
//!
//! Reading to and from files are easiest done with the [`read`] and [`write`] functions.
//!
//! Readers and writers are easies created with the [`create_reader`] and [`create_writer`]
//! functions.
//!
//! Error handling is defined in the [`error`] module.
//!
//! # Examples
//! Read from .osm file and write to .o5m file:
//! ```rust,no_run
//! use vadeen_osm::osm_io::{read, write};
//! # use vadeen_osm::osm_io::error::Result;
//! # fn main() -> Result<()> {
//! let osm = read("map.osm")?;
//! write("map.o5m", &osm)?;
//! # Ok(())
//! # }
//! ```
//!
//! Read from arbitrary reader and write to arbitrary writer:
//! ```rust,no_run
//! # use std::path::Path;
//! # use std::convert::TryInto;
//! # use std::fs::File;
//! # use vadeen_osm::osm_io::{create_reader, create_writer, FileFormat};
//! # use vadeen_osm::osm_io::error::Result;
//! # use std::io::BufReader;
//! # fn main() -> Result<()> {
//! // Read from file in this example, you can read from anything that implements the Read trait.
//! let path = Path::new("map.osm");
//! let input = File::open(path)?;
//!
//! // Get format from path. This can also be specified as FileFormat::Xml or any other FileFormat.
//! let format = path.try_into()?;
//!
//! // Create reader and read.
//! let mut reader = create_reader(BufReader::new(input), format);
//! let osm = reader.read()?;
//!
//! // ... do som stuff with the map.
//!
//! // Write to a Vec in this example, you can write to anything that implements the Write trait.
//! let output = Vec::new();
//!
//! // Create writer and write.
//! let mut writer = create_writer(output, FileFormat::O5m);
//! writer.write(&osm);
//! # Ok(())
//! # }
//! ```
//!
//! [`create_reader`]: fn.create_reader.html
//! [`create_writer`]: fn.create_writer.html
//! [`read`]: fn.read.html
//! [`write`]: fn.write.html
//! [`FileFormat`]: enum.FileFormat.html
//! [`error`]: error/index.html
extern crate chrono;

pub mod error;
mod o5m;
mod xml;

use self::error::*;
use self::o5m::O5mWriter;
use self::xml::XmlWriter;
use crate::osm_io::o5m::O5mReader;
use crate::osm_io::xml::XmlReader;
use crate::Osm;
use std::convert::{TryFrom, TryInto};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

/// Represent a osm file format.
///
/// See OSM documentation over [`file formats`].
///
/// # Examples
/// ```
/// # use vadeen_osm::osm_io::FileFormat;
/// # use std::path::Path;
/// # use std::convert::TryInto;
/// assert_eq!("osm".try_into(), Ok(FileFormat::Xml));
/// assert_eq!(Path::new("./path/file.o5m").try_into(), Ok(FileFormat::O5m));
/// assert_eq!(FileFormat::from("o5m"), Some(FileFormat::O5m));
/// ```
/// [`file formats`]: https://wiki.openstreetmap.org/wiki/OSM_file_formats
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum FileFormat {
    Xml,
    O5m,
}

/// Writer for the osm formats.
pub trait OsmWriter<W: Write> {
    fn write(&mut self, osm: &Osm) -> std::result::Result<(), Error>;

    fn into_inner(self: Box<Self>) -> W;
}

/// Reader for the osm formats.
pub trait OsmReader {
    fn read(&mut self) -> std::result::Result<Osm, Error>;
}

/// Convenience function for easily reading osm files.
/// Format is determined from file ending.
///
/// # Example
/// ```rust,no_run
/// # use vadeen_osm::osm_io::error::Result;
/// # use vadeen_osm::osm_io::read;
/// # fn main() -> Result<()> {
/// // Read xml map.
/// let osm = read("map.osm")?;
///
/// // Read o5m map.
/// let osm = read("map.o5m")?;
/// # Ok(())
/// # }
/// ```
pub fn read<P: AsRef<Path>>(path: P) -> Result<Osm> {
    let format = path.as_ref().try_into()?;
    let file = File::open(path)?;
    let mut reader = create_reader(BufReader::new(file), format);
    reader.read()
}

/// Convenience function for easily writing osm files.
/// Format is determined from file ending.
///
/// # Example
/// ```rust,no_run
/// # use vadeen_osm::OsmBuilder;
/// # use vadeen_osm::osm_io::error::Result;
/// # use vadeen_osm::osm_io::write;
/// # fn main() -> Result<()> {
/// let osm = OsmBuilder::default().build();
///
/// // Write xml map.
/// write("map.osm", &osm)?;
///
/// // Write o5m map.
/// write("map.o5m", &osm)?;
/// # Ok(())
/// # }
/// ```
pub fn write<P: AsRef<Path>>(path: P, osm: &Osm) -> Result<()> {
    let format = path.as_ref().try_into()?;
    let file = File::create(path)?;
    let mut writer = create_writer(file, format);
    writer.write(&osm)
}

/// Creates an `OsmReader` appropriate to the provided `FileFormat`.
///
/// # Example
/// Read map from map.osm
/// ```rust,no_run
/// # use std::path::Path;
/// # use std::convert::TryInto;
/// # use std::fs::File;
/// # use vadeen_osm::osm_io::create_reader;
/// # use vadeen_osm::osm_io::error::Result;
/// # use std::io::BufReader;
/// # fn main() -> Result<()> {
/// let path = Path::new("map.osm");
/// let file = File::open(path)?;
///
/// // Get format from path. This can also be specified as FileFormat::Xml.
/// let format = path.try_into()?;
///
/// // Read from file.
/// let mut reader = create_reader(BufReader::new(file), format);
/// let osm = reader.read()?;
/// # Ok(())
/// # }
/// ```
pub fn create_reader<'a, R: BufRead + 'a>(
    reader: R,
    format: FileFormat,
) -> Box<dyn OsmReader + 'a> {
    match format {
        FileFormat::Xml => Box::new(XmlReader::new(reader)),
        FileFormat::O5m => Box::new(O5mReader::new(reader)),
    }
}

/// Creates an `OsmWriter` appropriate to the provided `FileFormat`.
///
/// # Example
/// Write map to map.o5m
/// ```rust,no_run
/// # use vadeen_osm::{Osm, OsmBuilder};
/// # use vadeen_osm::osm_io::{create_writer, FileFormat, create_reader};
/// # use vadeen_osm::osm_io::error::Result;
/// # use std::fs::File;
/// # use std::path::Path;
/// # use std::convert::TryInto;
/// # use std::io::BufReader;
/// # fn main() -> Result<()> {
/// let builder = OsmBuilder::default();
/// // builder.add_polygon(..); etc...
/// let osm = builder.build();
///
/// // Write to file.
/// let output = File::create("map.o5m")?;
/// let mut writer = create_writer(output, FileFormat::O5m);
/// writer.write(&osm);
/// # Ok(())
/// # }
/// ```
pub fn create_writer<'a, W: Write + 'a>(
    writer: W,
    format: FileFormat,
) -> Box<dyn OsmWriter<W> + 'a> {
    match format {
        FileFormat::O5m => Box::new(O5mWriter::new(writer)),
        FileFormat::Xml => Box::new(XmlWriter::new(writer)),
    }
}

impl FileFormat {
    pub fn from(s: &str) -> Option<Self> {
        match s {
            "osm" => Some(FileFormat::Xml),
            "o5m" => Some(FileFormat::O5m),
            _ => None,
        }
    }
}

impl TryFrom<&str> for FileFormat {
    type Error = Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        if let Some(format) = FileFormat::from(&value) {
            Ok(format)
        } else {
            Err(Error::new(
                ErrorKind::InvalidFileFormat,
                Some(format!("'{}' is not a valid osm file format.", value)),
            ))
        }
    }
}

impl TryFrom<&String> for FileFormat {
    type Error = Error;

    fn try_from(value: &String) -> std::result::Result<Self, Self::Error> {
        (value[..]).try_into()
    }
}

impl TryFrom<&Path> for FileFormat {
    type Error = Error;

    fn try_from(path: &Path) -> std::result::Result<Self, Self::Error> {
        if let Some(ext) = path.extension() {
            if let Some(str) = ext.to_str() {
                return str.try_into();
            }
        }
        Err(Error::new(
            ErrorKind::InvalidFileFormat,
            Some(format!(
                "Could not determine format of '{}'.",
                path.to_str().unwrap()
            )),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::osm_io::{read, FileFormat};
    use std::convert::TryInto;
    use std::path::Path;

    #[test]
    fn file_format_from_path() {
        let path = Path::new("test.o5m");
        let format = path.try_into();
        assert_eq!(format, Ok(FileFormat::O5m));

        let path = Path::new("test.osm");
        let format = path.try_into();
        assert_eq!(format, Ok(FileFormat::Xml));
    }

    #[test]
    fn file_format_from_str() {
        let format = "o5m".try_into();
        assert_eq!(format, Ok(FileFormat::O5m));

        let format = "osm".try_into();
        assert_eq!(format, Ok(FileFormat::Xml));
    }

    #[test]
    fn file_format_from_string() {
        let format = (&"o5m".to_owned()).try_into();
        assert_eq!(format, Ok(FileFormat::O5m));

        let format = (&"osm".to_owned()).try_into();
        assert_eq!(format, Ok(FileFormat::Xml));
    }

    #[test]
    fn read_invalid_format() {
        let err = read("osm.invalid").unwrap_err();
        assert_eq!(err.to_string(), "'invalid' is not a valid osm file format.");
    }
}
