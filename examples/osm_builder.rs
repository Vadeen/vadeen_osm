use vadeen_osm::osm_io::error::Result;
use vadeen_osm::osm_io::write;
use vadeen_osm::OsmBuilder;

fn main() -> Result<()> {
    // Create a builder.
    let mut builder = OsmBuilder::default();

    // Add a polygon to the map.
    builder.add_polygon(
        vec![
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
        vec![("natural", "water").into()],
    );

    // Add polyline to the map.
    builder.add_polyline(
        vec![(66.29, 1.2).into(), (64.43, 1.2).into()],
        vec![("power", "line").into()],
    );

    // Add point
    builder.add_point((66.19, 1.3).into(), vec![("power", "tower").into()]);

    // Build into Osm structure.
    let osm = builder.build();

    // Write to file in the xml format.
    write("example_map.osm", &osm)?;

    // Write to file in the o5m format.
    write("example_map.o5m", &osm)?;

    Ok(())
}
