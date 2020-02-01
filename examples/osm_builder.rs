use vadeen_osm::osm_io::write;
use vadeen_osm::OsmBuilder;

fn main() {
    // Create a builder.
    let mut builder = OsmBuilder::default();

    // Add a polygon to the map.
    builder.add_polygon(
        &[
            vec![
                // Outer polygon
                (66.29, -3.177).into(),
                (66.29, -0.9422).into(),
                (64.43, -0.9422).into(),
                (64.43, -3.177).into(),
                (66.29, -3.177).into(),
            ],
            vec![
                // One inner polygon
                (66.0, -2.25).into(),
                (65.7, -2.5).into(),
                (65.7, -2.0).into(),
                (66.0, -2.25).into(),
            ],
            // Add more inner polygons here.
        ],
        &[("natural".to_owned(), "water".to_owned())],
    );

    // Add polyline to the map.
    builder.add_polyline(
        &[(66.29, 1.2).into(),
            (64.43, 1.2).into()],
        &[("power".to_owned(), "line".to_owned())],
    );

    // Add point
    builder.add_point((66.19, 1.3).into(), &[("power".to_owned(), "tower".to_owned())]);

    // Build into Osm structure.
    let osm = builder.build();

    // Write to file in the xml format.
    write("./example_map.osm", &osm).unwrap();

    // Write to file in the o5m format.
    write("./example_map.o5m", &osm).unwrap();
}