//! Base module for reading and writing o5m data.
//! See: https://wiki.openstreetmap.org/wiki/O5m

mod reader;
mod varint;
mod writer;

use crate::osm_io::error::{Error, ErrorKind, Result};
use crate::osm_io::o5m::varint::VarInt;
pub use reader::*;
use std::collections::VecDeque;
use std::fmt::Debug;
pub use writer::*;

const MAX_STRING_TABLE_SIZE: usize = 15_000;
const MAX_STRING_REFERENCE_LENGTH: usize = 250;

const O5M_HEADER_DATA: &[u8] = &[0x04, 0x6f, 0x35, 0x6d, 0x32];
const O5M_HEADER: u8 = 0xE0;
const O5M_EOF: u8 = 0xFE;
const O5M_RESET: u8 = 0xFF;
const O5M_NODE: u8 = 0x10;
const O5M_WAY: u8 = 0x11;
const O5M_RELATION: u8 = 0x12;
const O5M_BOUNDING_BOX: u8 = 0xDB;

/// String reference table is used for decoding and encoding strings as references.
/// See: https://wiki.openstreetmap.org/wiki/O5m#Strings
#[derive(Debug)]
struct StringReferenceTable {
    table: VecDeque<Vec<u8>>,
}

/// Represents a delta value, i.e. a value that is relative to it's last value.
#[derive(Debug)]
struct DeltaValue {
    state: i64,
}

#[derive(Debug, Copy, Clone)]
enum Delta {
    Id,
    Time,
    Lat,
    Lon,
    ChangeSet,
    WayRef,
    RelNodeRef,
    RelWayRef,
    RelRelRef,
}

#[derive(Debug)]
struct DeltaState {
    time: DeltaValue,
    id: DeltaValue,
    lat: DeltaValue,
    lon: DeltaValue,
    change_set: DeltaValue,
    way_ref: DeltaValue,
    rel_node_ref: DeltaValue,
    rel_way_ref: DeltaValue,
    rel_rel_ref: DeltaValue,
}

impl StringReferenceTable {
    pub fn new() -> Self {
        StringReferenceTable {
            table: VecDeque::with_capacity(15000),
        }
    }

    pub fn clear(&mut self) {
        self.table.clear();
    }

    /// Get string from table. idx starts at 1.
    pub fn get(&mut self, idx: u64) -> Result<&Vec<u8>> {
        if let Some(value) = self.table.get((idx - 1) as usize) {
            Ok(value)
        } else {
            Err(Error::new(
                ErrorKind::Parse,
                Some(format!(
                    "String reference '{}' not found in table with size '{}'.",
                    idx,
                    self.table.len()
                )),
            ))
        }
    }

    /// Get string reference. If string is present bytes representing a string reference will be
    /// returned. If not the string will be pushed to the table and returned untouched.
    fn reference(&mut self, bytes: Vec<u8>) -> Vec<u8> {
        if bytes.len() > MAX_STRING_REFERENCE_LENGTH {
            return bytes;
        }

        if let Some(pos) = self.table.iter().position(|b| b == &bytes) {
            VarInt::create_bytes((pos + 1) as u64)
        } else {
            self.push(&bytes);
            bytes
        }
    }

    /// Push string to table. The string is only added if it do not exceed 250 bytes in length.
    pub fn push(&mut self, bytes: &[u8]) {
        if bytes.len() > MAX_STRING_REFERENCE_LENGTH {
            return;
        }

        // Pop the oldest one off if we are at the limit.
        if self.table.len() == MAX_STRING_TABLE_SIZE {
            self.table.pop_back();
        }

        self.table.push_front(bytes.to_vec());
    }
}

impl DeltaValue {
    pub fn new() -> Self {
        DeltaValue { state: 0 }
    }

    /// Calculate delta and update state.
    pub fn delta(&mut self, id: i64) -> i64 {
        let delta = id - self.state;
        self.state = id;
        delta
    }

    /// Calculate value from delta and update state.
    pub fn value(&mut self, delta: i64) -> i64 {
        self.state += delta;
        self.state
    }
}

impl DeltaState {
    pub fn new() -> Self {
        DeltaState {
            time: DeltaValue::new(),
            id: DeltaValue::new(),
            lat: DeltaValue::new(),
            lon: DeltaValue::new(),
            change_set: DeltaValue::new(),
            way_ref: DeltaValue::new(),
            rel_node_ref: DeltaValue::new(),
            rel_way_ref: DeltaValue::new(),
            rel_rel_ref: DeltaValue::new(),
        }
    }

    pub fn encode(&mut self, delta: Delta, value: i64) -> i64 {
        let delta = self.get(delta);
        delta.delta(value)
    }

    pub fn decode(&mut self, delta: Delta, value: i64) -> i64 {
        let delta = self.get(delta);
        delta.value(value)
    }

    fn get(&mut self, delta: Delta) -> &mut DeltaValue {
        match delta {
            Delta::Id => &mut self.id,
            Delta::Time => &mut self.time,
            Delta::Lat => &mut self.lat,
            Delta::Lon => &mut self.lon,
            Delta::ChangeSet => &mut self.change_set,
            Delta::WayRef => &mut self.way_ref,
            Delta::RelNodeRef => &mut self.rel_node_ref,
            Delta::RelWayRef => &mut self.rel_way_ref,
            Delta::RelRelRef => &mut self.rel_rel_ref,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::osm_io::o5m::StringReferenceTable;

    #[test]
    fn string_references() {
        let mut table = StringReferenceTable::new();
        assert_eq!(table.reference(vec![0x01, 0x01]), vec![0x01, 0x01]); // New
        assert_eq!(table.reference(vec![0x02, 0x02]), vec![0x02, 0x02]); // New
        assert_eq!(table.reference(vec![0x01, 0x01]), vec![0x02]); // Existing
        assert_eq!(table.reference(vec![0x03, 0x03]), vec![0x03, 0x03]); // New
        assert_eq!(table.reference(vec![0x01, 0x01]), vec![0x03]); // Existing
        assert_eq!(table.reference(vec![0x02, 0x02]), vec![0x02]); // Existing
        assert_eq!(table.reference(vec![0x03, 0x03]), vec![0x01]); // Existing

        table.clear();

        assert_eq!(table.reference(vec![0x01, 0x01]), vec![0x01, 0x01]); // New
    }
}
