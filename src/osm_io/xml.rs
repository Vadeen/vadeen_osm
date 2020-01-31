//! Base module for reading and writing osm xml data.
//! See: https://wiki.openstreetmap.org/wiki/OSM_XML

extern crate quick_xml;

mod reader;
mod writer;

pub use self::reader::*;
pub use self::writer::*;
use crate::osm_io::error::Error;
use crate::osm_io::error::ErrorKind::ParseError;

impl From<quick_xml::Error> for Error {
    fn from(e: quick_xml::Error) -> Self {
        Error::new(ParseError, Some(e.to_string()))
    }
}

#[cfg(test)]
mod test {
    use crate::osm_io::{create_reader, FileFormat};

    #[test]
    fn quick_xml_error() {
        let xml = r#"
            <osm>
            </wrong-element>
        "#;
        let error = create_reader(xml.as_bytes(), FileFormat::Xml)
            .read()
            .unwrap_err();
        assert_eq!(
            "Line 3: Expecting </osm> found </wrong-element>",
            error.to_string()
        );
    }
}
