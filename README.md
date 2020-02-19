# Vadeen OSM

[![Build Status](https://travis-ci.com/FelixStridsberg/vadeen_osm.svg?branch=master)](https://travis-ci.com/FelixStridsberg/vadeen_osm)
[![Crate](https://img.shields.io/crates/v/vadeen_osm.svg)](https://crates.io/crates/vadeen_osm)
[![API](https://docs.rs/vadeen_osm/badge.svg)](https://docs.rs/vadeen_osm)


Vadeen OSM is a library for reading and writing [`Open Street Map`] files.
Currently support xml and o5m. Pbf in the near future.

## Goal
There is many [`great tools`] that works with Open Street Map files, for example [`mkgmap`] which can convert OSM maps to a
format compatible with GPS devices.

This project aims to be an easy to use library to compose OSM maps from other map sources. For example in Sweden the
[`Lantmäteriet`] (The Swedish National Land Survey) provides high quality Creative Common licensed [`map data for free`].

The main goal of this project is not to manage OSM data, but rather give 3rd party maps access to the OSM ecosystem.
The project aims to be fully compliant with OSM however, so managing plain OSM maps should be possible.

## Examples
To run the examples in the `examples/` folder run `cargo build --examples` and look in the
`target/debug/examples/` folder for the binaries.

### Simple read and write
```rust
use vadeen_osm::osm_io::{read, write};

// Read from file, format is determined from path.
let osm = read("map.osm")?;

// ...render or modify the map.

// Write to file, format is determined from path.
write("map.o5m", &osm)?;
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
            (66.29, -3.177),
            (66.29, -0.9422),
            (64.43, -0.9422),
            (64.43, -3.177),
            (66.29, -3.177),
        ],
        vec![
            // One inner polygon
            (66.0, -2.25),
            (65.7, -2.5),
            (65.7, -2.0),
            (66.0, -2.25),
        ],
        // Add more inner polygons here.
    ],
    vec![("natural", "water")],
);

// Add polyline to the map.
builder.add_polyline(vec![(66.29, 1.2), (64.43, 1.2)], vec![("power", "line")]);

// Add point
builder.add_point((66.19, 1.3), vec![("power", "tower")]);

// Build into Osm structure.
let osm = builder.build();

// Write to file in the xml format.
write("example_map.osm", &osm)?;

// Write to file in the o5m format.
write("example_map.o5m", &osm)?;
```

## Custom data
The builder can handle all kind of data as long as it has implemented the correct `Into` traits.

For example if you have a custom tag type `MyTag` and a custom coordinate type `MyCoordinate` you can use them
seamlessly with the builder as long as you implement the appropriate `Into<Tag>` and `Into<Coordinate>`.
```rust
// Your custom tag.
struct MyTag {
    key: String,
    value: String,
}

// Your custom coordinate type.
struct MyCoordinate {
    lat: f64,
    lon: f64,
}

// All you have to do is to implement the Into traits.
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

// Then you can use them with the builder:
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
```

## Create a map without builder
When not using the builder you have to keep track of all the ids your self.
This is only recommended if you work with actual OSM data, or if you want to break the rules of the `OsmBuilder`.
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
[`mkgmap`]: http://www.mkgmap.org.uk/
[`great tools`]: https://wiki.openstreetmap.org/wiki/Software/Desktop
[`Lantmäteriet`]: https://en.wikipedia.org/wiki/Lantm%C3%A4teriet
[`map data for free`]: https://www.lantmateriet.se/en/maps-and-geographic-information/open-geodata/
