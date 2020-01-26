//! Base module for reading and writing osm xml data.
//! See: https://wiki.openstreetmap.org/wiki/OSM_XML

extern crate quick_xml;

mod reader;
mod writer;

pub use self::reader::*;
pub use self::writer::*;
