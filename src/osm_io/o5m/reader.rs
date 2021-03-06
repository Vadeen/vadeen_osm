use super::varint::ReadVarInt;
use super::varint::VarInt;
use super::*;
use crate::geo::{Boundary, Coordinate};
use crate::osm_io::error::Result;
use crate::osm_io::error::{Error, ErrorKind};
use crate::osm_io::o5m::Delta::*;
use crate::osm_io::OsmRead;
use crate::{AuthorInformation, Meta, Node, Osm, Relation, RelationMember, Tag, Way};
use std::io::{BufRead, Read, Take};

/// A reader for the o5m format.
pub struct O5mReader<R: BufRead> {
    decoder: O5mDecoder<R>,
}

/// Low level decoding from binary to data types.
/// Keeps state of string references and delta values.
struct O5mDecoder<R: BufRead> {
    inner: Take<R>,
    string_table: StringReferenceTable,
    delta: DeltaState,
    limit: u64,
    position: u64,
}

impl<R: BufRead> O5mReader<R> {
    pub fn new(inner: R) -> Self {
        O5mReader {
            decoder: O5mDecoder::new(inner),
        }
    }

    /// Get the current position in the file.
    fn position(&self) -> u64 {
        self.decoder.position()
    }

    /// Parse next data set, returns false when there is no more data.
    fn parse_next(&mut self, osm: &mut Osm) -> Result<bool> {
        match self.read_set_type()? {
            O5M_NODE => {
                let node = self.read_node()?;
                osm.add_node(node)
            }
            O5M_WAY => osm.add_way(self.read_way()?),
            O5M_RELATION => osm.add_relation(self.read_relation()?),
            O5M_BOUNDING_BOX => osm.boundary = Some(self.read_boundary()?),
            O5M_RESET => self.decoder.reset(),
            O5M_EOF => return Ok(false),
            set_type => self.skip_dataset(set_type)?,
        }
        Ok(true)
    }

    /// See: https://wiki.openstreetmap.org/wiki/O5m#File
    fn read_set_type(&mut self) -> Result<u8> {
        self.decoder.set_limit(1);
        Ok(self.decoder.read_u8()?)
    }

    /// Skip a whole data set. Used when data set is unknown.
    fn skip_dataset(&mut self, block_type: u8) -> Result<()> {
        if block_type >= 0xF0 {
            self.decoder.read_limit()?;
            self.decoder.skip_all()?;
        }
        Ok(())
    }

    /// See: https://wiki.openstreetmap.org/wiki/O5m#Bounding_Box
    fn read_boundary(&mut self) -> Result<Boundary> {
        self.decoder.read_limit()?;
        let min_lon = self.decoder.read_varint()? as i32;
        let min_lat = self.decoder.read_varint()? as i32;
        let max_lon = self.decoder.read_varint()? as i32;
        let max_lat = self.decoder.read_varint()? as i32;
        Ok(Boundary {
            min: Coordinate {
                lat: min_lat,
                lon: min_lon,
            },
            max: Coordinate {
                lat: max_lat,
                lon: max_lon,
            },
            freeze: true,
        })
    }

    /// See: https://wiki.openstreetmap.org/wiki/O5m#Node
    fn read_node(&mut self) -> Result<Node> {
        self.decoder.read_limit()?;
        let mut node = Node::default();
        node.id = self.decoder.read_delta(Id)?;
        node.meta = self.read_meta()?;

        node.coordinate = self.decoder.read_delta_coordinate()?;
        node.meta.tags = self.decoder.read_tags()?;

        Ok(node)
    }

    /// See: https://wiki.openstreetmap.org/wiki/O5m#Way
    fn read_way(&mut self) -> Result<Way> {
        self.decoder.read_limit()?;

        let mut way = Way::default();
        way.id = self.decoder.read_delta(Id)?;
        way.meta = self.read_meta()?;

        let ref_size = self.decoder.read_uvarint()?;
        way.refs = self.decoder.read_way_references(ref_size)?;
        way.meta.tags = self.decoder.read_tags()?;

        Ok(way)
    }

    /// See: https://wiki.openstreetmap.org/wiki/O5m#Relation
    fn read_relation(&mut self) -> Result<Relation> {
        self.decoder.read_limit()?;

        let mut relation = Relation::default();
        relation.id = self.decoder.read_delta(Id)?;
        relation.meta = self.read_meta()?;

        let ref_size = self.decoder.read_uvarint()?;
        relation.members = self.decoder.read_relation_members(ref_size)?;
        relation.meta.tags = self.decoder.read_tags()?;

        Ok(relation)
    }

    /// Meta is common data part of every element.
    fn read_meta(&mut self) -> Result<Meta> {
        let mut meta = Meta::default();
        let version = self.decoder.read_uvarint()? as u32;
        meta.version = if version == 0 { None } else { Some(version) };

        // If version is 0 there is no timestamp or author.
        if meta.version.is_some() {
            meta.author = self.decoder.read_author_info()?;
        }

        Ok(meta)
    }
}

impl<R: BufRead> O5mDecoder<R> {
    fn new(inner: R) -> Self {
        O5mDecoder {
            inner: inner.take(0),
            string_table: StringReferenceTable::new(),
            delta: DeltaState::new(),
            limit: 0,
            position: 0,
        }
    }

    /// Reset string reference table and delta states.
    fn reset(&mut self) {
        self.string_table.clear();
        self.delta = DeltaState::new();
    }

    /// Set current limit of reader. If read past this an end of file error will occur.
    /// The limit is hit intentionally when reading tags and references etc.
    fn set_limit(&mut self, limit: u64) {
        // Update position by calculating how much we have read on the current limit setting.
        self.position += self.limit - self.inner.limit();

        self.limit = limit;
        self.inner.set_limit(limit);
    }

    /// Get the current position in file.
    fn position(&self) -> u64 {
        self.position + (self.limit - self.inner.limit())
    }

    /// Sets limit of the reader by reading the limit from the stream.
    fn read_limit(&mut self) -> Result<()> {
        self.set_limit(9);
        let len = self.read_uvarint()?;
        self.set_limit(len);
        Ok(())
    }

    /// Skip until limit or end of file is reached.
    fn skip_all(&mut self) -> Result<()> {
        let _ = self.read_until_eof(|r| {
            r.read_u8()?;
            Ok(())
        })?;
        Ok(())
    }

    /// Read coordinate and delta decode values.
    fn read_delta_coordinate(&mut self) -> Result<Coordinate> {
        let lon = self.read_delta(Lon)? as i32;
        let lat = self.read_delta(Lat)? as i32;
        Ok(Coordinate { lat, lon })
    }

    /// Wrapper for easy reading i64 varint.
    fn read_varint(&mut self) -> Result<i64> {
        Ok(self.inner.read_varint()?.into())
    }

    /// Wrapper for easy reading u64 varint.
    fn read_uvarint(&mut self) -> Result<u64> {
        Ok(self.inner.read_varint()?.into())
    }

    /// Read one single byte.
    fn read_u8(&mut self) -> Result<u8> {
        let mut bytes = [0u8; 1];
        self.inner.read_exact(&mut bytes)?;
        Ok(bytes[0])
    }

    /// Read author information. Uid, user and change set.
    fn read_author_info(&mut self) -> Result<Option<AuthorInformation>> {
        let created = self.read_delta(Time)?;

        // If there is no time, there is no author information.
        if created == 0 {
            return Ok(None);
        }

        let change_set = self.read_delta(ChangeSet)? as u64;
        let (uid, user) = self.read_user()?;
        Ok(Some(AuthorInformation {
            created,
            change_set,
            uid,
            user,
        }))
    }

    /// Read uid and user. Uid is encoded as a varint, but the bytes are treated as a string pair,
    /// i.e. appears in the string reference table.
    fn read_user(&mut self) -> Result<(u64, String)> {
        let reference = self.read_uvarint()?;
        if reference != 0 {
            let bytes = self.string_table.get(reference)?;
            Ok(Self::bytes_to_user(bytes))
        } else {
            let bytes = self.read_string_bytes(2)?;
            Ok(Self::bytes_to_user(&bytes))
        }
    }

    /// Turns bytes into uid and username.
    fn bytes_to_user(bytes: &[u8]) -> (u64, String) {
        let (uid_bytes, user_bytes) = Self::split_string_bytes(&bytes);
        let uid: u64 = VarInt::new(Vec::from(uid_bytes)).into();
        let user = String::from_utf8_lossy(&user_bytes).into_owned();
        (uid, user)
    }

    /// Read tags. There is no size or delimiter for tags, so they are read until there is no more
    /// data to read in the current limit.
    fn read_tags(&mut self) -> Result<Vec<Tag>> {
        let pairs = self.read_until_eof(|r| Ok(r.read_string_pair()?))?;
        let tags = pairs.into_iter().map(|s| s.into()).collect();
        Ok(tags)
    }

    /// Reads way references until `size` is consumed.
    fn read_way_references(&mut self, size: u64) -> Result<Vec<i64>> {
        let limit = self.inner.limit();
        self.set_limit(size);
        let refs = self.read_until_eof(|r| Ok(r.read_delta(WayRef)?))?;
        self.set_limit(limit - size);
        Ok(refs)
    }

    /// Reads relation members until `size` is consumed.
    fn read_relation_members(&mut self, size: u64) -> Result<Vec<RelationMember>> {
        let limit = self.inner.limit();
        self.set_limit(size);
        let members = self.read_until_eof(|r| Ok(r.read_relation_member()?))?;
        self.set_limit(limit - size);
        Ok(members)
    }

    /// Read a single relation member.
    fn read_relation_member(&mut self) -> Result<RelationMember> {
        let id = self.read_varint()?;
        let s = self.read_string()?;

        if !s.is_char_boundary(1) || s.len() < 2 {
            return Err(Error::new(
                ErrorKind::ParseError,
                Some("Corrupt relation member reference data.".to_owned()),
            ));
        }

        let (mem_type, mem_role) = s.split_at(1);

        match mem_type {
            "0" => Ok(RelationMember::Node(
                self.delta.decode(RelNodeRef, id),
                mem_role.to_owned(),
            )),
            "1" => Ok(RelationMember::Way(
                self.delta.decode(RelWayRef, id),
                mem_role.to_owned(),
            )),
            "2" => Ok(RelationMember::Relation(
                self.delta.decode(RelRelRef, id),
                mem_role.to_owned(),
            )),
            s => Err(Error::new(
                ErrorKind::ParseError,
                Some(format!("Invalid relation member type '{}'.", s)),
            )),
        }
    }

    /// Read real string pairs. I.e. data that is actually a pair of 2 strings, not single strings
    /// or a user which consists of one int and a string.
    fn read_string_pair(&mut self) -> Result<(String, String)> {
        let reference: u64 = self.inner.read_varint()?.into();
        if reference != 0 {
            let bytes = self.string_table.get(reference)?;
            Ok(Self::bytes_to_string_pair(bytes))
        } else {
            let bytes = self.read_string_bytes(2)?;
            Ok(Self::bytes_to_string_pair(&bytes))
        }
    }

    /// Read strings that do not come in pairs.
    fn read_string(&mut self) -> Result<String> {
        let reference = self.read_uvarint()?;
        let value = if reference == 0 {
            let value = self.read_string_bytes(1)?;
            String::from_utf8_lossy(&value).into_owned()
        } else {
            let value = self.string_table.get(reference)?;
            String::from_utf8_lossy(&value).into_owned()
        };

        Ok(value)
    }

    /// Turns bytes into two strings by splitting on first zero bytes and utf8 encode them.
    fn bytes_to_string_pair(bytes: &[u8]) -> (String, String) {
        let (key_bytes, value_bytes) = Self::split_string_bytes(bytes);
        let key = String::from_utf8_lossy(key_bytes).into_owned();
        let value = String::from_utf8_lossy(value_bytes).into_owned();
        (key, value)
    }

    /// Splits bytes at the first zero byte.
    /// Panics if 0-byte is not found.
    fn split_string_bytes(bytes: &[u8]) -> (&[u8], &[u8]) {
        let div = bytes.iter().position(|b| b == &0u8).unwrap();
        (&bytes[0..div], &bytes[(div + 1)..])
    }

    /// Reads string bytes from stream. A string can consist of 1 or more parts. Each part is
    /// divided by a zero byte. String pairs have 2 parts. Uid and and username have one part since
    /// they are not divided by a zero byte.
    fn read_string_bytes(&mut self, parts: u8) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        let mut count = 0;
        loop {
            let b = self.read_u8()?;
            if b == 0 {
                count += 1;
                if count == parts {
                    break;
                }
            }
            data.push(b);
        }

        self.string_table.push(&data);
        Ok(data)
    }

    /// Read a delta value.
    fn read_delta(&mut self, delta: Delta) -> Result<i64> {
        let val = self.read_varint()?;
        Ok(self.delta.decode(delta, val))
    }

    /// Calls callback until end of file is reached.
    /// This function assumes that the callback is consuming data on the provided reader (self),
    /// otherwise this will loop in infinity.
    fn read_until_eof<T>(&mut self, f: fn(&mut Self) -> Result<T>) -> Result<Vec<T>> {
        let mut vec = Vec::new();
        loop {
            match f(self) {
                Ok(r) => vec.push(r),
                Err(e) => {
                    if let ErrorKind::IO(err) = e.kind() {
                        if err.kind() == std::io::ErrorKind::UnexpectedEof {
                            break;
                        }
                    }
                    return Err(e);
                }
            }
        }
        Ok(vec)
    }
}

impl<R: BufRead> OsmRead for O5mReader<R> {
    fn read(&mut self) -> std::result::Result<Osm, Error> {
        let mut osm = Osm::default();

        loop {
            match self.parse_next(&mut osm) {
                Ok(true) => {}
                Ok(false) => break,
                Err(mut error) => {
                    if let Some(message) = error.message() {
                        let message = format!("Ending at byte {}: {}", self.position(), message);
                        error.set_message(message);
                    }

                    return Err(error);
                }
            }
        }

        Ok(osm)
    }
}

#[cfg(test)]
mod test {
    use crate::geo::Coordinate;
    use crate::osm_io::o5m::O5mReader;
    use crate::osm_io::OsmRead;
    use crate::{AuthorInformation, Meta, Node, Relation, RelationMember, Way};
    use std::io::BufReader;

    #[test]
    fn read_node() {
        let data: Vec<u8> = vec![
            //0x10, // node
            0x21, // length of following data of this node: 33 bytes
            0xce, 0xad, 0x0f, // id: 0+125799=125799
            0x05, // version: 5
            0xe4, 0x8e, 0xa7, 0xca, 0x09, // timestamp: 2010-09-30T19:23:30Z
            0x94, 0xfe, 0xd2, 0x05, // changeset: 0+5922698=5922698
            0x00, // string pair:
            0x85, 0xe3, 0x02, 0x00, // uid: 45445
            0x55, 0x53, 0x63, 0x68, 0x61, 0x00, // user: "UScha"
            0x86, 0x87, 0xe6, 0x53, // lon: 0+8.7867843=8.7867843
            0xcc, 0xe2, 0x94, 0xfa, 0x03, // lat: 0+53.0749606=53.0749606
        ];

        let mut reader = O5mReader::new(BufReader::new(data.as_slice()));
        let node = reader.read_node().unwrap();

        assert_eq!(
            node,
            Node {
                id: 125799,
                coordinate: Coordinate::new(53.0749606, 8.7867843),
                meta: Meta {
                    tags: vec![],
                    version: Some(5),
                    author: Some(AuthorInformation {
                        created: 1285874610,
                        change_set: 5922698,
                        uid: 45445,
                        user: "UScha".to_string()
                    }),
                    ..Meta::default()
                }
            }
        );
    }

    #[test]
    fn read_way() {
        let data: Vec<u8> = vec![
            // 0x11, // way
            0x20, // length of following data of this node: 32 bytes
            0xec, 0x9b, 0xe8, 0x03, // id: 0+3999478=3999478
            0x00, // no version and no author information
            0x07, // length of node references area: 7 bytes
            0xce, 0xb9, 0xfe, 0x13, // referenced node: 0+20958823=20958823
            0xce, 0xeb, 0x01, // referenced node: 20958823+15079=20973902
            0x00, // string pair:
            0x68, 0x69, 0x67, 0x68, 0x77, 0x61, 0x79, 0x00, // key: "highway"
            0x73, 0x65, 0x63, 0x6f, 0x6e, 0x64, 0x61, 0x72, 0x79, 0x00, // val: "secondary"
        ];

        let mut reader = O5mReader::new(BufReader::new(data.as_slice()));
        let way = reader.read_way().unwrap();

        assert_eq!(
            way,
            Way {
                id: 3999478,
                refs: vec![20958823, 20973902],
                meta: Meta {
                    tags: vec![("highway", "secondary").into()],
                    ..Meta::default()
                }
            }
        )
    }

    #[test]
    fn read_relation() {
        let data: Vec<u8> = vec![
            // 0x12, // relation
            0x28, // length of following data of this node: 40 bytes
            0x90, 0x2e, // id: 0+2952=2952
            0x00, // no version and no author information
            0x11, // length of references section: 17 bytes
            0xf4, 0x98, 0x83, 0x0b, // id: 0+11560506=11560506
            0x00, // string pair:
            0x31, // type: way
            0x69, 0x6e, 0x6e, 0x65, 0x72, 0x00, // role: "inner"
            0xca, 0x93, 0xd3, 0x0d, // id: 11560506+14312677=25873183
            0x01, // string pair: reference 1
            0x00, // string pair:
            0x74, 0x79, 0x70, 0x65, 0x00, // key: "type"
            0x6d, 0x75, 0x6c, 0x74, 0x69, 0x70, 0x6f, 0x6c, 0x79, 0x67, 0x6f, 0x6e,
            0x00, // val: "multipolygon"
        ];

        let mut reader = O5mReader::new(BufReader::new(data.as_slice()));
        let relation = reader.read_relation().unwrap();

        assert_eq!(
            relation,
            Relation {
                id: 2952,
                members: vec![
                    RelationMember::Way(11560506, "inner".to_owned()),
                    RelationMember::Way(25873183, "inner".to_owned()),
                ],
                meta: Meta {
                    tags: vec![("type", "multipolygon").into()],
                    ..Meta::default()
                }
            }
        )
    }

    #[test]
    fn invalid_relation_member_string() {
        let data: Vec<u8> = vec![
            0x12, // relation
            0x28, // length of following data of this node: 40 bytes
            0x90, 0x2e, // id: 0+2952=2952
            0x00, // no version and no author information
            0x11, // length of references section: 17 bytes
            0xf4, 0x98, 0x83, 0x0b, // id: 0+11560506=11560506
            0x00, 0xFF, 0x00, // Invalid string pair member ref.
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut reader = O5mReader::new(BufReader::new(data.as_slice()));
        let error = reader.read().unwrap_err();
        assert_eq!(
            error.to_string(),
            "Ending at byte 13: Corrupt relation member reference data."
        );
    }

    #[test]
    fn invalid_relation_member_type() {
        let data: Vec<u8> = vec![
            0x12, // relation
            0x28, // length of following data of this node: 40 bytes
            0x90, 0x2e, // id: 0+2952=2952
            0x00, // no version and no author information
            0x11, // length of references section: 17 bytes
            0xf4, 0x98, 0x83, 0x0b, // id: 0+11560506=11560506
            0x00, // String pair
            0x35, // Invalid type: 5
            0x69, 0x6e, 0x6e, 0x65, 0x72, 0x00, // role: "inner"
            0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut reader = O5mReader::new(BufReader::new(data.as_slice()));
        let error = reader.read().unwrap_err();
        assert_eq!(
            error.to_string(),
            "Ending at byte 18: Invalid relation member type '5'."
        );
    }

    #[test]
    fn never_ending_varint() {
        let data: Vec<u8> = vec![
            0x12, // relation
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];

        let mut reader = O5mReader::new(BufReader::new(data.as_slice()));
        let error = reader.read().unwrap_err();
        assert_eq!(
            error.to_string(),
            "Ending at byte 10: Varint overflow, read 9 bytes."
        );
    }

    #[test]
    fn invalid_string_reference() {
        let data: Vec<u8> = vec![
            0x12, // relation
            0x28, // length of following data of this node: 40 bytes
            0x90, 0x2e, // id: 0+2952=2952
            0x00, // no version and no author information
            0x11, // length of references section: 17 bytes
            0xf4, 0x98, 0x83, 0x0b, // id: 0+11560506=11560506
            0x03, // Invalid string reference
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut reader = O5mReader::new(BufReader::new(data.as_slice()));
        let error = reader.read().unwrap_err();
        assert_eq!(
            error.to_string(),
            "Ending at byte 11: String reference '3' not found in table with size '0'."
        );
    }

    #[test]
    fn unexpected_eof() {
        let data: Vec<u8> = vec![
            0x12, // relation
                 // No data
        ];

        let mut reader = O5mReader::new(BufReader::new(data.as_slice()));
        let error = reader.read().unwrap_err();
        assert_eq!(error.to_string(), "Unexpected end of file.");
    }
}
