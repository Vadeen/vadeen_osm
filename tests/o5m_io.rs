use std::convert::TryInto;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use vadeen_osm::geo::Coordinate;
use vadeen_osm::osm_io::{create_reader, create_writer, read, FileFormat};
use vadeen_osm::RelationMember::Way;

/// real_map.o5m is real_map.osm converted with osmconvert. There seems to be coordinate drifting
/// in that converter, so coordinates do not match up with the .osm version.
#[test]
fn read_o5m_file() {
    let osm = read("./tests/test_data/real_map.o5m").unwrap();

    let boundary = osm.boundary.as_ref().unwrap();
    assert_eq!(boundary.min, (60.6750500, 17.1362500).into());
    assert_eq!(boundary.max, (60.6763100, 17.1389800).into());

    // Assert a node.
    {
        let node = osm.nodes.iter().find(|r| r.id == 60686436).unwrap();
        assert_eq!(node.id, 60686436);
        assert_eq!(node.coordinate, Coordinate::new(60.6763366, 17.1421725));
        assert_eq!(node.meta.tags, []);

        let author = node.meta.author.as_ref().unwrap();
        assert_eq!(Some(3), node.meta.version);
        assert_eq!("Dalkvist", author.user);
        assert_eq!(12140, author.uid);
        assert_eq!(7035827, author.change_set);
        assert_eq!(1295564363, author.created);
    }

    // Assert a node with tags.
    {
        let node = osm.nodes.iter().find(|r| r.id == 232547314).unwrap();
        assert_eq!(node.id, 232547314);
        assert_eq!(node.coordinate, Coordinate::new(60.6770515, 17.1413803));
        assert_eq!(
            node.meta.tags,
            [
                ("highway", "crossing").into(),
                ("source", "extrapolation;survey").into()
            ]
        );

        let author = node.meta.author.as_ref().unwrap();
        assert_eq!(Some(5), node.meta.version);
        assert_eq!("Ice25T", author.user);
        assert_eq!(157205, author.uid);
        assert_eq!(63422528, author.change_set);
        assert_eq!(1539268691, author.created);
    }

    // Assert a way.
    {
        let way = osm.ways.iter().find(|r| r.id == 115494540).unwrap();
        assert_eq!(way.id, 115494540);
        assert_eq!(
            way.refs,
            [1304701749, 1304701930, 1304701751, 1304701661, 1304701749]
        );
        assert_eq!(
            way.meta.tags,
            [("amenity", "parking").into(), ("parking", "surface").into()]
        );

        let author = way.meta.author.as_ref().unwrap();
        assert_eq!(Some(1), way.meta.version);
        assert_eq!(author.user, "maxugglan");
        assert_eq!(author.uid, 107681);
        assert_eq!(author.change_set, 8280205);
        assert_eq!(author.created, 1306670457);
    }

    // Assert a relation.
    {
        let rel = osm.relations.iter().find(|r| r.id == 1604937).unwrap();
        assert_eq!(rel.id, 1604937);
        assert_eq!(
            rel.members,
            [
                Way(115494549, "inner".to_owned()),
                Way(115494554, "outer".to_owned())
            ]
        );
        assert_eq!(
            rel.meta.tags,
            [("building", "yes").into(), ("type", "multipolygon").into()]
        );

        let author = rel.meta.author.as_ref().unwrap();
        assert_eq!(Some(2), rel.meta.version);
        assert_eq!("AndersAndersson", author.user);
        assert_eq!(113813, author.uid);
        assert_eq!(11221181, author.change_set);
        assert_eq!(1333869214, author.created);
    }
}

#[test]
fn write_o5m_file() {
    let path = Path::new("./tests/test_data/generated.osm");
    let format = path.try_into().unwrap();
    let mut file = File::open(path).unwrap();
    let mut input = Vec::new();
    file.read_to_end(&mut input).unwrap();

    // Read xml
    let mut reader = create_reader(BufReader::new(&input[..]), format);
    let osm = reader.read().unwrap();

    // Read expected output
    let mut file = File::open("./tests/test_data/generated.o5m").unwrap();
    let mut expected_output = Vec::new();
    file.read_to_end(&mut expected_output).unwrap();

    // Write o5m
    let output = Vec::new();
    let mut writer = create_writer(output, FileFormat::O5m);
    writer.write(&osm).unwrap();

    assert_eq!(writer.into_inner(), expected_output);
}
