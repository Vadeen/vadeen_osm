//! This crate contains implementation to read and write [`Open Street Maps`].
//!
//! The [`Osm`] struct is an abstract representation of an OSM map. You can build your map with this
//! struct by adding nodes, ways and relations. Use the [`OsmBuilder`] if you are working with
//! non OSM data, it lets you work with polygons, poly lines and points instead.
//!
//! The [`osm_io`] module contains io functionality for reading and writing multiple OSM formats.
//! Currently osm and o5m is supported.
//!
//! The [`geo`] module contains some more general geographic abstractions used by this crate.
//!
//! [`Open Street Maps`]: https://wiki.openstreetmap.org/wiki/Main_Page
//! [`Osm`]: struct.Osm.html
//! [`OsmBuilder`]: struct.OsmBuilder.html
//! [`osm_io`]: osm_io/index.html
//! [`geo`]: geo/index.html
mod element;
pub mod geo;
pub mod osm_io;

use crate::geo::{Boundary, Coordinate};
pub use element::*;
use std::cmp::max;
use std::collections::HashMap;

/// `OsmBuilder` makes it easy to build OSM maps from non OSM data. Polygons, multi polygons,
/// poly lines and points are all represented as vectors of coordinates.
///
/// Nodes are automatically added and assigned ids. Ways and relations are automatically created
/// with the correct references.
///
/// When building maps from OSM data the elements should be added directly to an [`Osm`] struct to
/// preserve ids. You can also use the [`osm_io`] module which do exactly that when reading data.
///
/// # Examples
/// ```
/// # use vadeen_osm::OsmBuilder;
/// # use vadeen_osm::geo::Boundary;
/// let mut builder = OsmBuilder::default();
///
/// // Add a point, represented as one node.
/// builder.add_point((2.0, 2.0).into(), vec![("power", "tower").into()]);
///
/// // Add a poly line, represented as two nodes and a way.
/// builder.add_polyline(
///     vec![(2.0, 2.0).into(), (4.0, 5.0).into()],
///     vec![("power", "line").into()]
/// );
///
/// // Add a polygon, which is represented as one way and two nodes in osm.
/// builder.add_polygon(
///     vec![
///         // Outer polygon
///         vec![(1.0, 1.0).into(), (10.0, 10.0).into(), (5.0, 5.0).into(), (1.0, 1.0).into()],
///         // If you want inner polygons, add them here...
///         // Each inner polygon is represented as a way, the polygons are connected by a relation.
///     ],
///     vec![("natural", "water").into()]
/// );
///
/// let osm = builder.build();
/// assert_eq!(osm.nodes.len(), 5);
/// assert_eq!(osm.ways.len(), 2);
/// assert_eq!(osm.relations.len(), 0);
///
/// assert_eq!(osm.boundary, Some(Boundary::new((1.0, 1.0).into(), (10.0, 10.0).into())));
/// ```
///
/// [`Osm`]: struct.Osm.html
/// [`osm_io`]: osm_io/index.html
pub struct OsmBuilder {
    osm: Osm,
}

/// Abstract representation of an OSM map.
///
/// An OSM map contains a boundary, nodes, ways and relations. See the OSM documentation over
/// ['Elements'] for an overview of what each element represents.
///
/// The [`osm`] (xml) and [`o5m`] formats have a very similar structure which corresponds to this
/// struct.
///
/// To build an OSM map, you probably want to read it from file (see [`osm_io`]) or use the
/// [`OsmBuilder`].
///
/// [`osm`]: https://wiki.openstreetmap.org/wiki/OSM_XML
/// [`o5m`]: https://wiki.openstreetmap.org/wiki/O5m
/// [`pbf`]: https://wiki.openstreetmap.org/wiki/O5m
/// [`Elements`]: https://wiki.openstreetmap.org/wiki/Elements
/// [`osm_io`]: osm_io/index.html
/// [`OsmBuilder`]: struct.OsmBuilder.html
#[derive(Debug)]
pub struct Osm {
    pub boundary: Option<Boundary>,
    pub nodes: Vec<Node>,
    pub ways: Vec<Way>,
    pub relations: Vec<Relation>,
    max_id: i64,
    node_id_index: HashMap<Coordinate, i64>,
}

impl OsmBuilder {
    pub fn build(self) -> Osm {
        self.osm
    }

    pub fn add_point(&mut self, coordinate: Coordinate, tags: Vec<Tag>) {
        self.add_node(coordinate, tags);
    }

    /// First part is the outer polygon, rest of the parts is inner polygons.
    /// `parts` must not be empty or a panic will occur.
    pub fn add_polygon(&mut self, mut parts: Vec<Vec<Coordinate>>, tags: Vec<Tag>) {
        if parts.len() == 1 {
            self.add_polyline(parts.pop().unwrap(), tags);
        } else {
            self.add_multipolygon(parts, tags);
        }
    }

    pub fn add_polyline(&mut self, coordinates: Vec<Coordinate>, tags: Vec<Tag>) -> i64 {
        let refs = self.add_nodes(coordinates);
        let id = self.next_id();
        let meta = Meta {
            tags,
            ..Default::default()
        };
        self.osm.add_way(Way { id, refs, meta });
        id
    }

    fn add_multipolygon(&mut self, parts: Vec<Vec<Coordinate>>, mut tags: Vec<Tag>) {
        let mut polygon_ids = Vec::new();
        for part in parts {
            polygon_ids.push(self.add_polyline(part, vec![]));
        }

        tags.push(("type", "multipolygon").into());

        let (outer, inner) = polygon_ids.split_first().unwrap();
        self.add_polygon_relations(*outer, inner, tags);
    }

    fn add_polygon_relations(&mut self, outer: i64, inner: &[i64], tags: Vec<Tag>) {
        let mut members = Vec::new();
        for rel_ref in inner {
            members.push(RelationMember::Way(*rel_ref, "inner".to_owned()));
        }

        members.push(RelationMember::Way(outer, "outer".to_owned()));

        let id = self.next_id();
        let meta = Meta {
            tags,
            ..Default::default()
        };
        self.osm.add_relation(Relation { id, members, meta });
    }

    fn add_nodes(&mut self, coordinates: Vec<Coordinate>) -> Vec<i64> {
        coordinates
            .into_iter()
            .map(|c| self.add_node(c, vec![]))
            .collect()
    }

    fn add_node(&mut self, coordinate: Coordinate, tags: Vec<Tag>) -> i64 {
        if let Some(id) = self.osm.find_node_id(coordinate) {
            return id;
        }

        let id = self.osm.max_id + 1;
        let meta = Meta {
            tags,
            ..Default::default()
        };
        self.osm.add_node(Node {
            id,
            coordinate,
            meta,
        });
        id
    }

    fn next_id(&mut self) -> i64 {
        self.osm.max_id += 1;
        self.osm.max_id
    }
}

impl Default for OsmBuilder {
    fn default() -> Self {
        OsmBuilder {
            osm: Osm::default(),
        }
    }
}

impl Osm {
    /// Add a node to the map, the boundary is expanded to include the node.
    pub fn add_node(&mut self, node: Node) {
        if let Some(boundary) = &mut self.boundary {
            boundary.expand(node.coordinate);
        }

        self.max_id = max(self.max_id, node.id);
        self.node_id_index.insert(node.coordinate.clone(), node.id);
        self.nodes.push(node);
    }

    /// Add a way to the map.
    pub fn add_way(&mut self, way: Way) {
        self.ways.push(way);
    }

    pub fn add_relation(&mut self, relation: Relation) {
        self.relations.push(relation);
    }

    /// Find node id in an osm map by coordinate.
    pub fn find_node_id(&mut self, coordinate: Coordinate) -> Option<i64> {
        self.node_id_index.get(&coordinate).cloned()
    }
}

impl Default for Osm {
    fn default() -> Self {
        Osm {
            boundary: Some(Boundary::inverted()),
            nodes: Vec::new(),
            ways: Vec::new(),
            relations: Vec::new(),
            max_id: 0,
            node_id_index: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::geo::Boundary;
    use crate::{Meta, Node, Osm};

    #[test]
    fn osm_add_node() {
        let mut osm = Osm::default();
        assert_eq!(osm.max_id, 0);

        osm.add_node(Node {
            id: 10,
            coordinate: (65.0, 55.0).into(),
            meta: Meta::default(),
        });

        let expected_boundary = Boundary {
            min: (65.0, 55.0).into(),
            max: (65.0, 55.0).into(),
            freeze: false,
        };
        assert_eq!(osm.max_id, 10);
        assert_eq!(osm.boundary, Some(expected_boundary));
    }
}
