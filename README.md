# Vadeen OSM

[![Build Status](https://travis-ci.com/FelixStridsberg/vadeen_osm.svg?branch=master)](https://travis-ci.com/FelixStridsberg/vadeen_osm)
[![Crate](https://img.shields.io/crates/v/vadeen_osm.svg)](https://crates.io/crates/vadeen_osm)
[![API](https://docs.rs/vadeen_osm/badge.svg)](https://docs.rs/vadeen_osm)

Vadeen OSM is a library for reading and writing [`Open Street Map`] files.
Currently support xml and o5m. Pbf in the near future.

## Examples
To run the examples in the `examples/` folder run `cargo build --examples` and look in the
`target/debug/examples/` folder for the binaries.

### Simple read and write
```rust
use vadeen_osm::osm_io::{read, write};

// Read from file, format is determined from path.
let osm = read("map.osm").unwrap();

// ...render or modify the map.

// Write to file, format is determined from path.
write("map.o5m", &osm).unwrap();
```

### Create a map with the builder
The `OsmBuilder` has an abstraction to make it easy to build maps from other map data. It uses
terms as polygon (for areas), polyline (for lines) and points.
```rust
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
write("./example_map.osm", &osm).unwrap();

// Write to file in the o5m format.
write("./example_map.o5m", &osm).unwrap();
```

## Create a map without builder
When not using the builder you have to keep track of all the ids your self.
```rust
let mut osm = Osm::default();

// Add a node
osm.add_node(Node {
    id: 1,
    coordinate: (66.29, -3.177).into(),
    meta: Meta {
        tags: vec![("key", "value").into()],
        version: Some(3),
        author: Some(AuthorInformation {
            created: 12345678,
            change_set: 1,
            uid: 1234,
            user: "Username".to_string(),
        }),
    },
});

// Add a way with no tags or nothing.
osm.add_way(Way {
    id: 2,
    refs: vec![1],
    meta: Default::default(),
});

// Add a relation with no tags or nothing.
osm.add_relation(Relation {
    id: 3,
    members: vec![RelationMember::Way(2, "role".to_owned())],
    meta: Default::default(),
});

// ...etc
```

[`Open Street Map`]: https://wiki.openstreetmap.org/wiki/OSM_file_formats
