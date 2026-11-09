//! Decode actual binary STL output: closed oriented triangle manifold,
//! correct volume, original envelope, and faceted roundover containment.
//! Includes the small-radius regression that the old area filter opened.

use std::collections::BTreeMap;

use kernel::{
    Document, ImportNode, ImportScene, MeshRecipe, Object, Plane, Point3, Profile, Transform, Vec3,
    edge_treatment::roundover,
};

fn document_from(object: &Object) -> Document {
    let (positions, faces) = object.to_polygons();
    let count = faces.len();
    let mesh = MeshRecipe {
        name: "Roundover".into(),
        positions,
        faces,
        face_materials: vec![kernel::serialize::NO_MATERIAL; count],
        face_uv_frames: vec![None; count],
        face_holes: vec![vec![]; count],
        base_material: kernel::serialize::NO_MATERIAL,
        tags: vec![],
        extra: Default::default(),
    };
    let scene = ImportScene {
        materials: vec![],
        defs: vec![],
        roots: vec![ImportNode::Mesh(mesh)],
        guides: vec![],
        tags: vec![],
        camera: None,
        scenes: vec![],
        section: None,
        view_style: None,
        extra: Default::default(),
        geo: None,
    };
    let mut doc = Document::new();
    doc.ingest(scene, vec![]).unwrap();
    assert_eq!(doc.visible_object_ids().len(), 1);
    doc
}

#[test]
fn roundover_stl_is_closed_outward_contained_and_has_the_faceted_volume() {
    for (width, depth, length, radius, error, angles) in [
        (0.02, 0.03, 0.04, 0.002, 0.000005, &[0.0, 0.67][..]),
        (0.04, 0.35, 0.45, 0.006, 0.00001, &[0.0, 0.67][..]),
        // Original Phase 2.5 reproduction: 88 valid triangles, ten below
        // the former area threshold. Translation below is unchanged.
        (0.04, 0.35, 0.45, 0.00001, 0.00000001, &[0.0, 0.67][..]),
    ] {
        for &angle in angles {
            let plane = Plane::from_point_normal(Point3::ORIGIN, Vec3::new(0.0, 0.0, 1.0)).unwrap();
            let profile = Profile::new(
                plane,
                vec![
                    Point3::ORIGIN,
                    Point3::new(width, 0.0, 0.0),
                    Point3::new(width, depth, 0.0),
                    Point3::new(0.0, depth, 0.0),
                ],
                vec![],
            )
            .unwrap();
            let mut source = Object::from_extrusion(&profile, length).unwrap();
            let edge = source
                .edges()
                .keys()
                .find(|&e| {
                    let (a, b) = source.edge_endpoints(e).unwrap();
                    a.x == 0.0 && a.y == 0.0 && b.x == 0.0 && b.y == 0.0
                })
                .unwrap();
            let pose = Transform::rotation(Vec3::new(1.0, 2.0, 3.0), angle)
                .unwrap()
                .then(&Transform::translation(Vec3::new(0.03, -0.02, 0.01)));
            let inverse = pose.inverse().unwrap();
            source.apply_transform(&pose).unwrap();
            let rounded = roundover(&source, edge, radius, error).unwrap();
            let candidate = &rounded.result.object;
            let doc = document_from(candidate);
            let stored = mesh_export::export_stl(&doc, 0, true).unwrap();
            // Unattributed roundover facets must not be reinterpreted as
            // analytic curves by export's optional resolution setting.
            assert_eq!(stored, mesh_export::export_stl(&doc, 96, true).unwrap());
            let count = u32::from_le_bytes(stored[80..84].try_into().unwrap()) as usize;
            assert_eq!(stored.len(), 84 + 50 * count);
            assert!(count > 12);
            assert_eq!(
                count,
                tessellate::export_triangles(candidate, 0).unwrap().len() / 9,
                "serialization must not discard any facet"
            );
            if radius == 0.00001 && angle == 0.0 {
                assert_eq!(
                    count, 88,
                    "all facets from the original reproduction survive"
                );
            }
            let interior = pose.apply_point(Point3::new(width / 2.0, depth / 2.0, length / 2.0));
            let mesh_tolerance = 8.0 * f64::from(f32::EPSILON) * length.max(width).max(depth);
            let key = |p: [f32; 3]| p.map(|v| if v == 0.0 { 0 } else { v.to_bits() });
            let mut edges = BTreeMap::<([u32; 3], [u32; 3]), usize>::new();
            let mut volume = 0.0;
            for index in 0..count {
                let offset = 84 + index * 50;
                let read = |at: usize| f32::from_le_bytes(stored[at..at + 4].try_into().unwrap());
                let normal = Vec3::new(
                    f64::from(read(offset)),
                    f64::from(read(offset + 4)),
                    f64::from(read(offset + 8)),
                );
                let raw: [[f32; 3]; 3] = std::array::from_fn(|i| {
                    std::array::from_fn(|j| read(offset + 12 + i * 12 + j * 4))
                });
                let vertices = raw.map(|p| {
                    Point3::new(
                        f64::from(p[0]) / 1000.0,
                        f64::from(p[1]) / 1000.0,
                        f64::from(p[2]) / 1000.0,
                    )
                });
                let cross = (vertices[1] - vertices[0]).cross(vertices[2] - vertices[0]);
                assert!(cross.length() > 0.0, "no collapsed f32 triangles");
                assert!(
                    cross.dot(normal) > 0.0,
                    "serialized normal agrees with vertex winding"
                );
                assert!(
                    cross.dot(vertices[0] - interior) > 0.0,
                    "triangle winds away from an interior witness"
                );
                volume += (vertices[0] - interior)
                    .dot((vertices[1] - interior).cross(vertices[2] - interior))
                    / 6.0;
                for i in 0..3 {
                    *edges
                        .entry((key(raw[i]), key(raw[(i + 1) % 3])))
                        .or_default() += 1;
                }
                for point in vertices {
                    let p = inverse.apply_point(point);
                    assert!(p.x >= -mesh_tolerance && p.x <= width + mesh_tolerance);
                    assert!(p.y >= -mesh_tolerance && p.y <= depth + mesh_tolerance);
                    assert!(p.z >= -mesh_tolerance && p.z <= length + mesh_tolerance);
                    assert!(
                        candidate
                            .faces()
                            .values()
                            .all(|f| f.plane.signed_distance(point) <= mesh_tolerance)
                    );
                    if p.x <= radius && p.y <= radius {
                        let radial = ((p.x - radius).powi(2) + (p.y - radius).powi(2)).sqrt();
                        assert!((radial - radius).abs() <= error + mesh_tolerance);
                    }
                }
            }
            for ((a, b), uses) in &edges {
                assert_ne!(a, b);
                assert_eq!(*uses, 1, "one triangle per directed edge");
                assert_eq!(
                    edges.get(&(*b, *a)),
                    Some(&1),
                    "every edge has its reverse twin"
                );
            }
            let expected = width * depth * length - rounded.result.expected_removed_volume;
            assert!(volume > 0.0);
            assert!(
                (volume - expected).abs() <= expected * 1e-5,
                "decoded STL volume {volume} vs {expected}"
            );
        }
    }
}
