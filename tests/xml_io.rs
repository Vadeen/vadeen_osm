use std::convert::TryInto;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use vadeen_osm::geo::Coordinate;
use vadeen_osm::osm_io::{create_reader, create_writer};
use vadeen_osm::RelationMember::Way;

#[test]
fn read_osm_file() {
    let path = Path::new("./tests/test_data/real_map.osm");
    let format = path.try_into().unwrap();
    let file = File::open(path).unwrap();
    let mut reader = create_reader(BufReader::new(file), format);
    let osm = reader.read().unwrap();

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
        // TODO timestamp, 2011-01-20T22:59:23Z
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
        // TODO timestamp, 2018-10-11T14:38:11Z
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
        assert_eq!("maxugglan", author.user);
        assert_eq!(107681, author.uid);
        assert_eq!(8280205, author.change_set);
        // TODO timestamp, 2011-05-29T12:00:57Z
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
        // TODO timestamp, 2012-04-08T07:13:34Z
    }
}

#[test]
fn read_write_osm_file() {
    let path = Path::new("./tests/test_data/generated.osm");
    let format = path.try_into().unwrap();
    let mut file = File::open(path).unwrap();
    let mut input = Vec::new();
    file.read_to_end(&mut input).unwrap();

    // Read data
    let mut reader = create_reader(BufReader::new(&input[..]), format);
    let osm = reader.read().unwrap();

    // Write data
    let output = Vec::new();
    let mut writer = create_writer(output, format);
    writer.write(&osm).unwrap();

    assert_eq!(input, writer.into_inner());
}
