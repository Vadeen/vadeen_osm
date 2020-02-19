use vadeen_osm::osm_io::error::Result;
use vadeen_osm::osm_io::write;
use vadeen_osm::{OsmBuilder, Tag};
use vadeen_osm::geo::Coordinate;

struct MyTag {
    key: String,
    value: String,
}

struct MyCoordinate {
    lat: f64,
    lon: f64,
}

impl MyTag {
    fn new(key: &str, value: &str) -> Self {
        MyTag {
            key: key.to_owned(),
            value: value.to_owned(),
        }
    }
}

impl MyCoordinate {
    fn new(lat: f64, lon: f64) -> Self {
        MyCoordinate { lat, lon }
    }
}

impl Into<Coordinate> for MyCoordinate {
    fn into(self) -> Coordinate {
        Coordinate::new(self.lat, self.lon)
    }
}

impl Into<Tag> for MyTag {
    fn into(self) -> Tag {
        Tag {
            key: self.key,
            value: self.value,
        }
    }
}

fn main() -> Result<()> {
    // Create a builder.
    let mut builder = OsmBuilder::default();

    // Add a polygon to the map.
    builder.add_polygon(
        vec![
            vec![
                // Outer polygon
                MyCoordinate::new(66.29, -3.177),
                MyCoordinate::new(66.29, -0.9422),
                MyCoordinate::new(64.43, -0.9422),
                MyCoordinate::new(64.43, -3.177),
                MyCoordinate::new(66.29, -3.177),
            ],
            // Add inner polygons here.
        ],
        vec![MyTag::new("natural", "water")],
    );

    // Build into Osm structure.
    let osm = builder.build();

    // Write to file in the xml format.
    write("example_map.osm", &osm)?;
    Ok(())
}
