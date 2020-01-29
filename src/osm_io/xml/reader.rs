use super::super::chrono::{DateTime, Utc};
use super::quick_xml::Reader;
use crate::geo::{Boundary, Coordinate};
use crate::osm_io::error::{Error, ErrorKind, Result};
use crate::osm_io::OsmReader;
use crate::{AuthorInformation, Meta, Node, Osm, Relation, RelationMember, Tag, Way};
use quick_xml::events::{BytesStart, Event};
use std::collections::HashMap;
use std::io::BufRead;
use std::str::FromStr;

/// A reader for the xml format.
pub struct XmlReader<R: BufRead> {
    reader: Reader<R>,
    line: u32,
}

/// Abstract representation of the attributes of an XML element.
/// The attributes of each xml different element contains all information to create that OSM
/// element.
pub struct Attributes {
    map: HashMap<String, String>,
}

impl Attributes {
    /// Create from quick_xml attributes data.
    fn from(attributes: super::quick_xml::events::attributes::Attributes) -> Self {
        let mut map = HashMap::new();
        for attr in attributes {
            if let Ok(attr) = attr {
                if let Ok(value) = attr.unescaped_value() {
                    map.insert(
                        String::from_utf8_lossy(attr.key).into_owned(),
                        String::from_utf8_lossy(value.as_ref()).into_owned(),
                    );
                }
            }
        }
        Attributes { map }
    }

    fn get(&self, key: &str) -> Option<&String> {
        self.map.get(key)
    }

    /// Same as normal get, but returns error instead of option.
    fn get_required(&self, val: &str) -> Result<&String> {
        Ok(self.get(val).ok_or_else(|| {
            ErrorKind::InvalidData(format!("Required attribute '{}' missing.", val))
        })?)
    }

    /// Get element (with get_required) and parse data into F.
    fn get_parse<F: FromStr>(&self, field: &str) -> Result<F>
    where
        <F as std::str::FromStr>::Err: std::fmt::Debug,
    {
        let s = self.get_required(field)?;
        self.parse(field, s)
    }
    /// Get element (with get_required) and parse data into F.
    fn parse<F: FromStr>(&self, field: &str, s: &str) -> Result<F>
    where
        <F as std::str::FromStr>::Err: std::fmt::Debug,
    {
        str::parse(s).map_err(|_| {
            ErrorKind::InvalidData(format!(
                "The '{}' attribute contains invalid data '{}'.",
                field, s
            ))
        })
    }

    /// Check if all attribute `keys` are present.
    fn contains_all(&self, keys: Vec<&str>) -> bool {
        for key in keys {
            if !self.map.contains_key(key) {
                return false;
            }
        }
        true
    }

    /// Try to create a `Coordinate` from attribute values.
    fn create_coordinate(&self) -> Result<Coordinate> {
        Ok(Coordinate::new(
            self.get_parse("lat")?,
            self.get_parse("lon")?,
        ))
    }

    /// Try to create a `Boundary` from attribute values.
    fn create_boundary(&self) -> Result<Boundary> {
        Ok(Boundary {
            min: Coordinate::new(self.get_parse("minlat")?, self.get_parse("minlon")?),
            max: Coordinate::new(self.get_parse("maxlat")?, self.get_parse("maxlon")?),
            freeze: true,
        })
    }

    /// Try to create a `Tag` from attribute values.
    fn create_tag(&self) -> Result<Tag> {
        Ok(Tag {
            key: self.get_required("k")?.to_owned(),
            value: self.get_required("v")?.to_owned(),
        })
    }

    /// Try to create a `Meta` from attribute values.
    fn create_meta(&self) -> Result<Meta> {
        let author = if self.contains_all(vec!["timestamp", "uid", "user", "changeset"]) {
            Some(AuthorInformation {
                created: self.get_timestamp()?,
                uid: self.get_parse("uid")?,
                user: self.get_required("user")?.to_owned(),
                change_set: self.get_parse("changeset")?,
            })
        } else {
            None
        };

        let version = if let Some(version) = self.get("version") {
            Some(self.parse("version", version)?)
        } else {
            None
        };

        Ok(Meta {
            version,
            author,
            ..Meta::default()
        })
    }

    fn get_timestamp(&self) -> Result<i64> {
        let time_str = self.get_required("timestamp")?;
        if let Ok(time) = time_str.parse::<DateTime<Utc>>() {
            Ok(time.timestamp())
        } else {
            return Err(ErrorKind::InvalidData(format!(
                "Invalid timestamp '{}'",
                time_str
            )));
        }
    }

    /// Try to create a `RelationMember` from attribute values.
    fn create_relation_member(&self) -> Result<RelationMember> {
        let default_role = "".to_owned();
        let mem_ref = self.get_parse("ref")?;
        let mem_type = self.get_required("type")?;
        let mem_role = self.get("role").unwrap_or(&default_role);

        match mem_type.as_ref() {
            "node" => Ok(RelationMember::Node(mem_ref, mem_role.to_owned())),
            "way" => Ok(RelationMember::Way(mem_ref, mem_role.to_owned())),
            "rel" => Ok(RelationMember::Relation(mem_ref, mem_role.to_owned())),
            t => Err(ErrorKind::InvalidData(format!(
                "The 'type' attribute contains invalid data '{}'.",
                t
            ))),
        }
    }
}

impl<R: BufRead> XmlReader<R> {
    pub fn new(inner: R) -> XmlReader<R> {
        XmlReader {
            reader: Reader::from_reader(inner),
            line: 0,
        }
    }

    /// Parse next xml element. Returns false if end of file was reached.
    fn parse_event(&mut self, osm: &mut Osm) -> Result<bool> {
        let mut buf = Vec::new();
        match self.reader.read_event(&mut buf)? {
            Event::Start(ref event) => self.parse_element(osm, event)?,
            Event::Empty(ref event) => self.parse_empty_element(osm, event)?,
            Event::Eof => return Ok(false),
            _ => { /* Ignore all other events. */ }
        }

        self.line += buf.iter().filter(|b| **b == b'\n').count() as u32;
        Ok(true)
    }

    /// Read until and end element, or end of file is reached.
    /// Only empty elements are returned, the rest is ignored. This limitation since OSM only use
    /// empty element in a nested context within the <osm> tag.
    ///
    /// TODO Corruption if nested elements are encountered:
    /// This should return error if non empty element is encountered. The end of the nested element
    /// will terminate this read and possibly corrupt the flow.
    fn read_element_content(&mut self, mut buf: &mut Vec<u8>) -> Result<Vec<BytesStart>> {
        let mut events = Vec::new();
        loop {
            match self.reader.read_event(&mut buf)? {
                Event::Empty(ref e) => events.push(e.to_owned()),
                Event::End(_) => break,
                Event::Eof => break,
                _ => { /* Only empty elements are expected in element contents. */ }
            }
        }
        Ok(events)
    }

    /// Parse empty top level element. (<node.../>, <bounds.../>)
    fn parse_empty_element(&mut self, osm: &mut Osm, event: &BytesStart) -> Result<()> {
        match event.name() {
            b"node" => osm.add_node(parse_node(&event)?),
            b"bounds" => osm.boundary = Some(parse_boundary(&event)?),
            _ => {}
        }
        Ok(())
    }

    /// Parse non empty elements. (<node...>, <way...>, ...)
    fn parse_element(&mut self, osm: &mut Osm, event: &BytesStart) -> Result<()> {
        // We only work on one indentation level. To do this we must ignore <osm> since it
        // introduces another one.
        if event.name() == b"osm" {
            return Ok(());
        }

        let mut buf = Vec::new();
        let event_content = self.read_element_content(&mut buf)?;
        match event.name() {
            b"node" => {
                let mut node = parse_node(&event)?;
                node.meta.tags = create_tags(&event_content)?;
                osm.add_node(node);
            }
            b"way" => {
                let mut way = parse_way(&event)?;
                way.refs = create_way_refs(&event_content)?;
                way.meta.tags = create_tags(&event_content)?;
                osm.add_way(way);
            }
            b"relation" => {
                let mut relation = parse_relation(&event)?;
                relation.members = create_relation_members(&event_content)?;
                relation.meta.tags = create_tags(&event_content)?;
                osm.add_relation(relation);
            }
            _ => { /* Ignore unknown elements. */ }
        }

        self.line += buf.iter().filter(|b| **b == b'\n').count() as u32;
        Ok(())
    }
}

impl<R: BufRead> OsmReader for XmlReader<R> {
    fn read(&mut self) -> std::result::Result<Osm, Error> {
        let mut osm = Osm::default();
        loop {
            match self.parse_event(&mut osm) {
                Ok(true) => {}
                Ok(false) => break,
                Err(cause) => {
                    return Err(Error::new(cause, None, Some(self.line)));
                }
            }
        }

        if let Some(boundary) = osm.boundary.as_mut() {
            boundary.freeze = false;
        }

        Ok(osm)
    }
}

fn parse_boundary(event: &BytesStart) -> Result<Boundary> {
    let attributes = Attributes::from(event.attributes());
    Ok(attributes.create_boundary()?)
}

fn parse_node(event: &BytesStart) -> Result<Node> {
    let attributes = Attributes::from(event.attributes());
    Ok(Node {
        id: attributes.get_parse("id")?,
        coordinate: attributes.create_coordinate()?,
        meta: attributes.create_meta()?,
    })
}

fn parse_way(event: &BytesStart) -> Result<Way> {
    let attributes = Attributes::from(event.attributes());
    Ok(Way {
        id: attributes.get_parse("id")?,
        refs: vec![],
        meta: attributes.create_meta()?,
    })
}

fn parse_relation(event: &BytesStart) -> Result<Relation> {
    let attributes = Attributes::from(event.attributes());
    Ok(Relation {
        id: attributes.get_parse("id")?,
        members: vec![],
        meta: attributes.create_meta()?,
    })
}

fn create_tags(events: &[BytesStart]) -> Result<Vec<Tag>> {
    let mut tags = Vec::new();
    for e in events.iter().filter(|e| e.name() == b"tag") {
        tags.push(Attributes::from(e.attributes()).create_tag()?);
    }
    Ok(tags)
}

fn create_way_refs(events: &[BytesStart]) -> Result<Vec<i64>> {
    let mut refs = Vec::new();
    for e in events.iter().filter(|e| e.name() == b"nd") {
        refs.push(Attributes::from(e.attributes()).get_parse("ref")?);
    }
    Ok(refs)
}

fn create_relation_members(events: &[BytesStart]) -> Result<Vec<RelationMember>> {
    let mut members = Vec::new();
    for e in events.iter().filter(|e| e.name() == b"member") {
        members.push(Attributes::from(e.attributes()).create_relation_member()?);
    }
    Ok(members)
}

#[cfg(test)]
mod tests {
    use crate::geo::{Boundary, Coordinate};
    use crate::osm_io::error::ErrorKind;
    use crate::osm_io::xml::XmlReader;
    use crate::osm_io::OsmReader;
    use crate::{AuthorInformation, Meta, Node, Relation, RelationMember, Way};

    #[test]
    fn read_boundary() {
        let xml = r#"<bounds minlat="58.24" minlon="15.16" maxlat="62.18" maxlon="17.34"/>"#;
        let mut reader = XmlReader::new(xml.as_bytes());
        let osm = reader.read().unwrap();

        assert_eq!(
            osm.boundary,
            Some(Boundary {
                min: (58.24, 15.16).into(),
                max: (62.18, 17.34).into(),
                freeze: false,
            })
        );
    }

    #[test]
    fn read_node() {
        let xml = r#"<node id="25496583" lat="51.5173639" lon="-0.140043" version="1"
                           changeset="203496" user="80n" uid="1238" visible="true"
                           timestamp="2007-01-28T11:40:26Z" />"#;
        let mut reader = XmlReader::new(xml.as_bytes());
        let osm = reader.read().unwrap();

        assert_eq!(osm.nodes.len(), 1);
        assert_eq!(osm.ways.len(), 0);
        assert_eq!(osm.relations.len(), 0);

        assert_eq!(
            osm.nodes[0],
            Node {
                id: 25496583,
                coordinate: Coordinate::new(51.5173639, -0.140043),
                meta: Meta {
                    version: Some(1),
                    author: Some(AuthorInformation {
                        created: 1169984426,
                        uid: 1238,
                        user: "80n".to_owned(),
                        change_set: 203496,
                    }),
                    ..Meta::default()
                }
            }
        );
    }

    #[test]
    fn read_node_with_tags() {
        let xml = r#"<node id="25496583" lat="51.5173639" lon="-0.140043" version="1"
                           changeset="203496" user="80n" uid="1238" visible="true"
                           timestamp="2007-01-28T11:40:26Z">
                         <tag k="name" v="light"/>
                         <tag k="highway" v="traffic_signals"/>
                     </node>"#;
        let mut reader = XmlReader::new(xml.as_bytes());
        let osm = reader.read().unwrap();

        assert_eq!(osm.nodes.len(), 1);
        assert_eq!(osm.ways.len(), 0);
        assert_eq!(osm.relations.len(), 0);

        assert_eq!(
            osm.nodes[0].meta.tags,
            vec![
                ("name", "light").into(),
                ("highway", "traffic_signals").into()
            ]
        );
    }

    #[test]
    fn read_node_missing_required_attributes() {
        let missing_id = r#"<node lat="51.12" lon="22.14" version="1" />"#;
        let missing_lat = r#"<node id="1" lon="22.14" version="1" />"#;
        let missing_lon = r#"<node id="1" lat="51.12" version="1" />"#;
        let data = vec![
            ("id", missing_id),
            ("lat", missing_lat),
            ("lon", missing_lon),
        ];

        validate_missing_attributes(data);
    }

    #[test]
    fn read_node_invalid_required_attributes() {
        let invalid_id = r#"<node id="123.22" lat="51.12" lon="22.14" version="1" />"#;
        let invalid_lat = r#"<node id="1" lat="51.INVALID" lon="22.14" version="1" />"#;
        let invalid_lon = r#"<node id="1" lat="51.12" lon="INVALID.14" version="1" />"#;
        let invalid_version = r#"<node id="1" lat="51.12" lon="22.14" version="" />"#;
        let data = vec![
            ("id", "123.22", invalid_id),
            ("lat", "51.INVALID", invalid_lat),
            ("lon", "INVALID.14", invalid_lon),
            ("version", "", invalid_version),
        ];

        validate_invalid_attributes(data);
    }

    #[test]
    fn read_way() {
        let xml = r#"<way id="5090250" version="1" changeset="203496" user="80n" uid="1238"
                           visible="true" timestamp="2007-01-28T11:40:26Z">
                           <nd ref="822403"/>
                           <nd ref="21533912"/>
                           <nd ref="821601"/>
                           <tag k="highway" v="residential"/>
                           <tag k="oneway" v="yes"/>
                     </way>"#;
        let mut reader = XmlReader::new(xml.as_bytes());
        let osm = reader.read().unwrap();

        assert_eq!(osm.nodes.len(), 0);
        assert_eq!(osm.ways.len(), 1);
        assert_eq!(osm.relations.len(), 0);

        assert_eq!(
            osm.ways[0],
            Way {
                id: 5090250,
                refs: vec![822403, 21533912, 821601],
                meta: Meta {
                    version: Some(1),
                    tags: vec![("highway", "residential").into(), ("oneway", "yes").into()],
                    author: Some(AuthorInformation {
                        created: 1169984426,
                        uid: 1238,
                        user: "80n".to_owned(),
                        change_set: 203496,
                    }),
                    ..Meta::default()
                }
            }
        );
    }

    #[test]
    fn read_way_missing_required_attributes() {
        let missing_id = r#"<way version="1"></way>"#;
        let missing_nd_ref = r#"<way id="1" version="1"><nd/></way>"#;
        let missing_tag_k = r#"<way id="1" version="1"><tag v="value"/></way>"#;
        let missing_tag_v = r#"<way id="1" version="1"><tag k="key"/></way>"#;
        let data = vec![
            ("id", missing_id),
            ("ref", missing_nd_ref),
            ("k", missing_tag_k),
            ("v", missing_tag_v),
        ];

        validate_missing_attributes(data);
    }

    #[test]
    fn read_way_invalid_required_attributes() {
        let invalid_id = r#"<way id="INVALID" version="1"></way>"#;
        let invalid_version = r#"<way id="1" version=""></way>"#;
        let invalid_nd_ref = r#"<way id="1" version="1"><nd ref="INVALID"/></way>"#;
        let data = vec![
            ("id", "INVALID", invalid_id),
            ("version", "", invalid_version),
            ("ref", "INVALID", invalid_nd_ref),
        ];

        validate_invalid_attributes(data);
    }

    #[test]
    fn read_relation() {
        let xml = r#"<relation id="56688" version="28" changeset="203496" user="80n" uid="1238"
                           visible="true" timestamp="2009-02-20T19:40:26Z">
                         <member type="node" ref="821601"/>
                         <member type="way" ref="821602" role=""/>
                         <member type="rel" ref="821603" role="outer"/>
                         <tag k="route" v="bus"/>
                         <tag k="ref" v="123"/>
                     </relation>"#;
        let mut reader = XmlReader::new(xml.as_bytes());
        let osm = reader.read().unwrap();

        assert_eq!(osm.nodes.len(), 0);
        assert_eq!(osm.ways.len(), 0);
        assert_eq!(osm.relations.len(), 1);

        assert_eq!(
            osm.relations[0],
            Relation {
                id: 56688,
                members: vec![
                    RelationMember::Node(821601, "".to_owned()),
                    RelationMember::Way(821602, "".to_owned()),
                    RelationMember::Relation(821603, "outer".to_owned()),
                ],
                meta: Meta {
                    version: Some(28),
                    tags: vec![("route", "bus").into(), ("ref", "123").into()],
                    author: Some(AuthorInformation {
                        created: 1235158826,
                        uid: 1238,
                        user: "80n".to_owned(),
                        change_set: 203496,
                    }),
                    ..Meta::default()
                }
            }
        );
    }

    #[test]
    fn read_relation_missing_required_attributes() {
        let missing_id = r#"<relation version="1"></relation>"#;
        let missing_mem_ref = r#"<relation id="1" version="1"><member type="way"/></relation>"#;
        let missing_mem_type = r#"<relation id="1" version="1"><member ref="22"/></relation>"#;
        let data = vec![
            ("id", missing_id),
            ("ref", missing_mem_ref),
            ("type", missing_mem_type),
        ];

        validate_missing_attributes(data);
    }

    #[test]
    fn read_relation_invalid_required_attributes() {
        let invalid_id = r#"<relation id="INVALID" version="1"></relation>"#;
        let invalid_version = r#"<relation id="1" version="INVALID"></relation>"#;
        let invalid_mem_ref =
            r#"<relation id="1" version="1"><member type="way" ref="INVALID"/></relation>"#;
        let invalid_mem_type =
            r#"<relation id="1" version="1"><member type="INVALID" ref="2"/></relation>"#;
        let data = vec![
            ("id", "INVALID", invalid_id),
            ("version", "INVALID", invalid_version),
            ("ref", "INVALID", invalid_mem_ref),
            ("type", "INVALID", invalid_mem_type),
        ];

        validate_invalid_attributes(data);
    }

    fn validate_missing_attributes(data: Vec<(&str, &str)>) {
        for (field, xml) in data.iter() {
            let error = XmlReader::new(xml.as_bytes()).read().unwrap_err();
            assert_eq!(error.line(), Some(0));
            match error.kind() {
                ErrorKind::InvalidData(s) => {
                    assert_eq!(s, &format!("Required attribute '{}' missing.", field))
                }
                e => panic!("Unexpected kind {:?}", e),
            }
        }
    }

    fn validate_invalid_attributes(data: Vec<(&str, &str, &str)>) {
        for (field, value, xml) in data.iter() {
            let error = XmlReader::new(xml.as_bytes()).read().unwrap_err();
            assert_eq!(error.line(), Some(0));
            match error.kind() {
                ErrorKind::InvalidData(s) => assert_eq!(
                    s,
                    &format!(
                        "The '{}' attribute contains invalid data '{}'.",
                        field, value
                    )
                ),
                e => panic!("Unexpected kind {:?}", e),
            }
        }
    }
}
