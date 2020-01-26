//! OSM elements.
//!
//! See: https://wiki.openstreetmap.org/wiki/Elements

use crate::geo::Coordinate;

type RelationRole = String;

/// A coordinate with meta data. See OSM docs for [`Node`].
///
/// [`Node`]: https://wiki.openstreetmap.org/wiki/Node
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Node {
    pub id: i64,
    pub coordinate: Coordinate,
    pub meta: Meta,
}

/// Group of nodes and meta data. See OSM docs for [`Way`].
///
/// [`Way`]: https://wiki.openstreetmap.org/wiki/Way
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Way {
    pub id: i64,
    pub refs: Vec<i64>,
    pub meta: Meta,
}

/// Group of elements (node, way or relation). See OSM docs for [`Relation`].
///
/// [`Relation`]: https://wiki.openstreetmap.org/wiki/Relation
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Relation {
    pub id: i64,
    pub members: Vec<RelationMember>,
    pub meta: Meta,
}

/// Key value pairs. See OSM docs for [`Tags`].
///
/// [`Tags`]: https://wiki.openstreetmap.org/wiki/Tags
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Tag {
    pub key: String,
    pub value: String,
}

// TODO timestamp
/// Common meta data used by multiple entities.
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Meta {
    pub tags: Vec<Tag>,
    pub version: u32,
    pub author: Option<AuthorInformation>,
}

/// Author information is used to identify what nodes, ways and relation a specific user has
/// added. When working on non osm maps, this data is irrelevant.
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct AuthorInformation {
    pub change_set: u64,
    pub uid: u64,
    pub user: String,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum RelationMember {
    Node(i64, RelationRole),
    Way(i64, RelationRole),
    Relation(i64, RelationRole),
}

impl From<&(String, String)> for Tag {
    fn from((key, value): &(String, String)) -> Self {
        Tag {
            key: key.clone(),
            value: value.clone(),
        }
    }
}

impl From<(&str, &str)> for Tag {
    fn from((key, value): (&str, &str)) -> Self {
        Tag {
            key: key.to_owned(),
            value: value.to_owned(),
        }
    }
}

impl RelationMember {
    pub fn ref_id(&self) -> i64 {
        match self {
            RelationMember::Node(id, _) => *id,
            RelationMember::Way(id, _) => *id,
            RelationMember::Relation(id, _) => *id,
        }
    }

    pub fn role(&self) -> &str {
        match self {
            RelationMember::Node(_, role) => role,
            RelationMember::Way(_, role) => role,
            RelationMember::Relation(_, role) => role,
        }
    }
}

impl Default for Node {
    fn default() -> Self {
        Node {
            id: 0,
            coordinate: Coordinate { lat: 0, lon: 0 },
            meta: Default::default(),
        }
    }
}

impl Default for Way {
    fn default() -> Self {
        Way {
            id: 0,
            refs: vec![],
            meta: Default::default(),
        }
    }
}

impl Default for Relation {
    fn default() -> Self {
        Relation {
            id: 0,
            members: vec![],
            meta: Default::default(),
        }
    }
}

impl Default for Meta {
    fn default() -> Self {
        Meta {
            tags: vec![],
            version: 1,
            author: None,
        }
    }
}
