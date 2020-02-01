use vadeen_osm::{AuthorInformation, Meta, Node, Osm, Relation, RelationMember, Way};

fn main() {
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
}
