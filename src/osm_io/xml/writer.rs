use super::quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use super::quick_xml::Writer;
use crate::geo::Boundary;
use crate::osm_io::error::{ErrorKind, Result};
use crate::osm_io::OsmWriter;
use crate::{Meta, Node, Osm, Relation, RelationMember, Tag, Way};
use std::io::Write;

const OSM_VERSION: &str = "0.6";
const OSM_GENERATOR: &str = "Vadeen OSM";
const XML_VERSION: &[u8] = b"1.0";
const XML_ENCODING: &[u8] = b"UTF-8";

/// A writer for the xml format.
pub struct XmlWriter<W: Write> {
    writer: Writer<W>,
}

impl<W: Write> XmlWriter<W> {
    pub fn new(inner: W) -> XmlWriter<W> {
        XmlWriter {
            writer: Writer::new(inner),
        }
    }

    /// Write the start tags: Xml header and <osm>-tag.
    fn write_start(&mut self) -> Result<()> {
        self.writer.write_event(Event::Decl(BytesDecl::new(
            XML_VERSION,
            Some(XML_ENCODING),
            None,
        )))?;
        self.writer.write(b"\n")?;

        let elem = BytesStart::owned_name(b"osm".to_vec())
            .with_attributes(vec![("version", OSM_VERSION), ("generator", OSM_GENERATOR)]);
        self.writer.write_event(Event::Start(elem))?;
        self.writer.write(b"\n")?;
        Ok(())
    }

    /// Write end of osm: </osm>.
    fn write_end(&mut self) -> Result<()> {
        let elem = BytesEnd::owned(b"osm".to_vec());
        self.writer.write_event(Event::End(elem))?;
        Ok(())
    }

    /// Optional bounds box tag.
    fn write_bounds(&mut self, bounds: &Boundary) -> Result<()> {
        let elem = BytesStart::owned_name(b"bounds".to_vec()).with_attributes(vec![
            ("minlat", bounds.min.lat().to_string().as_ref()),
            ("minlon", bounds.min.lon().to_string().as_ref()),
            ("maxlat", bounds.max.lat().to_string().as_ref()),
            ("maxlon", bounds.max.lon().to_string().as_ref()),
        ]);

        self.writer.write(b"\t")?;
        self.writer.write_event(Event::Empty(elem))?;
        self.writer.write(b"\n")?;
        Ok(())
    }

    /// See: https://wiki.openstreetmap.org/wiki/Node
    fn write_node(&mut self, node: &Node) -> Result<()> {
        let mut elem = BytesStart::owned_name(b"node".to_vec()).with_attributes(vec![
            ("id", node.id.to_string().as_ref()),
            ("lat", node.coordinate.lat().to_string().as_ref()),
            ("lon", node.coordinate.lon().to_string().as_ref()),
        ]);

        add_meta_attributes(&mut elem, &node.meta);

        if node.meta.tags.is_empty() {
            self.writer.write(b"\t")?;
            self.writer.write_event(Event::Empty(elem))?;
        } else {
            self.writer.write(b"\t")?;
            self.writer.write_event(Event::Start(elem))?;
            self.writer.write(b"\n")?;

            self.write_tags(&node.meta.tags)?;

            self.writer.write(b"\t")?;
            self.writer
                .write_event(Event::End(BytesEnd::owned(b"node".to_vec())))?;
        }
        self.writer.write(b"\n")?;
        Ok(())
    }

    /// See: https://wiki.openstreetmap.org/wiki/Way
    fn write_way(&mut self, way: &Way) -> Result<()> {
        let mut elem = BytesStart::owned_name(b"way".to_vec());
        elem.push_attribute(("id", way.id.to_string().as_ref()));

        add_meta_attributes(&mut elem, &way.meta);

        self.writer.write(b"\t")?;
        self.writer.write_event(Event::Start(elem))?;
        self.writer.write(b"\n")?;

        for r in &way.refs {
            let mut nd = BytesStart::owned_name(b"nd".to_vec());
            nd.push_attribute(("ref", r.to_string().as_ref()));
            self.writer.write(b"\t\t")?;
            self.writer.write_event(Event::Empty(nd))?;
            self.writer.write(b"\n")?;
        }

        self.write_tags(&way.meta.tags)?;

        self.writer.write(b"\t")?;
        self.writer
            .write_event(Event::End(BytesEnd::owned(b"way".to_vec())))?;
        self.writer.write(b"\n")?;
        Ok(())
    }

    /// See: https://wiki.openstreetmap.org/wiki/Relation
    fn write_relation(&mut self, rel: &Relation) -> Result<()> {
        let mut elem = BytesStart::owned_name(b"relation".to_vec());
        elem.push_attribute(("id", rel.id.to_string().as_ref()));

        add_meta_attributes(&mut elem, &rel.meta);

        self.writer.write(b"\t")?;
        self.writer.write_event(Event::Start(elem))?;
        self.writer.write(b"\n")?;

        for m in &rel.members {
            let mut mem = BytesStart::owned_name(b"member".to_vec());
            add_member_attributes(&mut mem, m);

            self.writer.write(b"\t\t")?;
            self.writer.write_event(Event::Empty(mem))?;
            self.writer.write(b"\n")?;
        }

        self.write_tags(&rel.meta.tags)?;

        self.writer.write(b"\t")?;
        self.writer
            .write_event(Event::End(BytesEnd::owned(b"relation".to_vec())))?;
        self.writer.write(b"\n")?;
        Ok(())
    }

    /// See: https://wiki.openstreetmap.org/wiki/Tags
    fn write_tags(&mut self, tags: &[Tag]) -> Result<()> {
        for tag in tags {
            let tag_elem = BytesStart::owned_name(b"tag".to_vec())
                .with_attributes(vec![("k", tag.key.as_ref()), ("v", tag.value.as_ref())]);

            self.writer.write(b"\t\t")?;
            self.writer.write_event(Event::Empty(tag_elem))?;
            self.writer.write(b"\n")?;
        }
        Ok(())
    }
}

impl<W: Write> OsmWriter<W> for XmlWriter<W> {
    fn write(&mut self, osm: &Osm) -> std::result::Result<(), ErrorKind> {
        self.write_start()?;

        if let Some(boundary) = &osm.boundary {
            self.write_bounds(boundary)?;
        }

        for node in &osm.nodes {
            self.write_node(node)?;
        }

        for way in &osm.ways {
            self.write_way(way)?;
        }

        for rel in &osm.relations {
            self.write_relation(rel)?;
        }

        self.write_end()?;
        Ok(())
    }

    fn into_inner(self: Box<Self>) -> W {
        self.writer.into_inner()
    }
}

/// Add relation member attributes to an element.
fn add_member_attributes(elem: &mut BytesStart, mem: &RelationMember) {
    let (mem_type, mem_ref, mem_role) = match mem {
        RelationMember::Node(mem_ref, role) => ("node", mem_ref, role),
        RelationMember::Way(mem_ref, role) => ("way", mem_ref, role),
        RelationMember::Relation(mem_ref, role) => ("relation", mem_ref, role),
    };

    elem.extend_attributes(vec![
        ("type", mem_type),
        ("ref", mem_ref.to_string().as_ref()),
        ("role", mem_role),
    ]);
}

/// Add the meta attributes to an element.
fn add_meta_attributes(elem: &mut BytesStart, meta: &Meta) {
    if let Some(user) = &meta.author {
        elem.extend_attributes(vec![
            ("uid", user.uid.to_string().as_ref()),
            ("user", user.user.as_ref()),
            ("changeset", user.change_set.to_string().as_ref()),
        ]);
    }

    let version = meta.version;
    elem.push_attribute(("version", version.unwrap_or(1).to_string().as_ref()));
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::geo::Boundary;
    use crate::osm_io::xml::XmlWriter;
    use crate::{AuthorInformation, Meta, Node, Relation, RelationMember, Way};

    use super::OSM_GENERATOR;
    use super::OSM_VERSION;

    #[test]
    fn write_start() {
        let mut writer = XmlWriter::new(Cursor::new(Vec::new()));
        writer.write_start().unwrap();

        let xml = writer.writer.into_inner().into_inner();
        assert_eq!(
            String::from_utf8_lossy(&xml),
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                 <osm version=\"{}\" generator=\"{}\">\n",
                OSM_VERSION, OSM_GENERATOR
            )
        );
    }

    #[test]
    fn write_end() {
        let mut writer = XmlWriter::new(Cursor::new(Vec::new()));
        writer.write_end().unwrap();

        let xml = writer.writer.into_inner().into_inner();
        assert_eq!(String::from_utf8_lossy(&xml), "</osm>");
    }

    #[test]
    fn write_node() {
        let mut writer = XmlWriter::new(Cursor::new(Vec::new()));
        writer
            .write_node(&Node {
                id: 10,
                coordinate: (65.12, 55.21).into(),
                meta: Meta {
                    tags: vec![],
                    version: None,
                    author: Some(AuthorInformation {
                        change_set: 1234,
                        uid: 4321,
                        user: "osm".to_owned(),
                    }),
                },
            })
            .unwrap();

        let xml = writer.writer.into_inner().into_inner();
        assert_eq!(
            String::from_utf8_lossy(&xml),
            "\t<node id=\"10\" lat=\"65.12\" lon=\"55.21\" uid=\"4321\" user=\"osm\" \
             changeset=\"1234\" version=\"1\"/>\n"
        );
    }

    #[test]
    fn write_node_with_tags() {
        let mut writer = XmlWriter::new(Cursor::new(Vec::new()));
        writer
            .write_node(&Node {
                id: 10,
                coordinate: (65.12, 55.21).into(),
                meta: Meta {
                    tags: vec![
                        ("name", "Neu Broderstorf").into(),
                        ("traffic_sign", "city_limit").into(),
                    ],
                    version: Some(1),
                    author: Some(AuthorInformation {
                        change_set: 1234,
                        uid: 4321,
                        user: "osm".to_owned(),
                    }),
                },
            })
            .unwrap();

        let xml = writer.writer.into_inner().into_inner();
        assert_eq!(
            String::from_utf8_lossy(&xml),
            "\t<node id=\"10\" lat=\"65.12\" lon=\"55.21\" uid=\"4321\" user=\"osm\" \
             changeset=\"1234\" version=\"1\">\n\
             \t\t<tag k=\"name\" v=\"Neu Broderstorf\"/>\n\
             \t\t<tag k=\"traffic_sign\" v=\"city_limit\"/>\n\
             \t</node>\n"
        );
    }

    #[test]
    fn write_way() {
        let mut writer = XmlWriter::new(Cursor::new(Vec::new()));
        writer
            .write_way(&Way {
                id: 47,
                refs: vec![44, 45, 46],
                meta: Meta {
                    tags: vec![
                        ("highway", "unclassified").into(),
                        ("name", "Pastower Straße").into(),
                    ],
                    version: Some(2),
                    author: Some(AuthorInformation {
                        change_set: 12,
                        uid: 222,
                        user: "mos".to_owned(),
                    }),
                },
            })
            .unwrap();

        let xml = writer.writer.into_inner().into_inner();
        assert_eq!(
            String::from_utf8_lossy(&xml),
            "\t<way id=\"47\" uid=\"222\" user=\"mos\" changeset=\"12\" version=\"2\">\n\
             \t\t<nd ref=\"44\"/>\n\
             \t\t<nd ref=\"45\"/>\n\
             \t\t<nd ref=\"46\"/>\n\
             \t\t<tag k=\"highway\" v=\"unclassified\"/>\n\
             \t\t<tag k=\"name\" v=\"Pastower Straße\"/>\n\
             \t</way>\n"
        );
    }

    #[test]
    fn write_relation() {
        let mut writer = XmlWriter::new(Cursor::new(Vec::new()));
        writer
            .write_relation(&Relation {
                id: 47,
                members: vec![
                    RelationMember::Node(44, "".to_owned()),
                    RelationMember::Way(45, "inner".to_owned()),
                    RelationMember::Relation(46, "role".to_owned()),
                ],
                meta: Meta {
                    tags: vec![
                        ("highway", "unclassified").into(),
                        ("name", "Pastower Straße").into(),
                    ],
                    version: Some(2),
                    author: Some(AuthorInformation {
                        change_set: 12,
                        uid: 222,
                        user: "mos".to_owned(),
                    }),
                },
            })
            .unwrap();

        let xml = writer.writer.into_inner().into_inner();
        assert_eq!(
            String::from_utf8_lossy(&xml),
            "\t<relation id=\"47\" uid=\"222\" user=\"mos\" changeset=\"12\" version=\"2\">\n\
             \t\t<member type=\"node\" ref=\"44\" role=\"\"/>\n\
             \t\t<member type=\"way\" ref=\"45\" role=\"inner\"/>\n\
             \t\t<member type=\"relation\" ref=\"46\" role=\"role\"/>\n\
             \t\t<tag k=\"highway\" v=\"unclassified\"/>\n\
             \t\t<tag k=\"name\" v=\"Pastower Straße\"/>\n\
             \t</relation>\n"
        );
    }

    #[test]
    fn write_bounds() {
        let mut writer = XmlWriter::new(Cursor::new(Vec::new()));
        writer.write_bounds(&Boundary::default()).unwrap();

        let xml = writer.writer.into_inner().into_inner();
        assert_eq!(
            String::from_utf8_lossy(&xml),
            "\t<bounds minlat=\"-90\" minlon=\"-180\" maxlat=\"90\" maxlon=\"180\"/>\n"
        )
    }
}
