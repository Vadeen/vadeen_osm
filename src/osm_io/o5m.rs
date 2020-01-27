//! Base module for reading and writing o5m data.
//! See: https://wiki.openstreetmap.org/wiki/O5m

mod reader;
mod varint;
mod writer;

pub use reader::*;
use std::fmt::Debug;
pub use writer::*;

const O5M_HEADER_DATA: &[u8] = &[0x04, 0x6f, 0x35, 0x6d, 0x32];
const O5M_HEADER: u8 = 0xE0;
const O5M_EOF: u8 = 0xFE;
const O5M_RESET: u8 = 0xFF;
const O5M_NODE: u8 = 0x10;
const O5M_WAY: u8 = 0x11;
const O5M_RELATION: u8 = 0x12;
const O5M_BOUNDING_BOX: u8 = 0xDB;

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
