//! Fedora polygon-boss reproduction; preserve safe refusals until a generalized
//! termination/local-solid proof exists. Box batching must not hide these cases.
use kernel::{
    Document, DocumentEdgeTreatmentError, EdgeTreatmentOperation, KernelOp, KernelOpReport, Object,
    Plane, Point3, Profile, Vec3, WatertightState,
    edge_treatment::{EdgeTreatmentError, roundover},
};
fn plane(z: f64) -> Plane {
    Plane::from_point_normal(Point3::new(0.0, 0.0, z), Vec3::new(0.0, 0.0, 1.0)).unwrap()
}
fn hex(z: f64) -> Vec<Point3> {
    (0..6)
        .map(|i| {
            let t = std::f64::consts::TAU * i as f64 / 6.0;
            Point3::new(0.5 + 0.2 * t.cos(), 0.5 + 0.2 * t.sin(), z)
        })
        .collect()
}
fn top_edges(o: &Object, z: f64) -> Vec<kernel::EdgeId> {
    o.edges()
        .keys()
        .filter(|&e| {
            let (a, b) = o.edge_endpoints(e).unwrap();
            (a.z - z).abs() < 1e-10 && (b.z - z).abs() < 1e-10
        })
        .collect()
}
#[test]
fn standalone_convex_hexagon_upper_edges_refuse_oblique_termination_not_ownership() {
    let object =
        Object::from_extrusion(&Profile::new(plane(0.0), hex(0.0), vec![]).unwrap(), 0.2).unwrap();
    object.validate().unwrap();
    assert_eq!(object.watertight(), WatertightState::Watertight);
    let edges = top_edges(&object, 0.2);
    assert_eq!(edges.len(), 6);
    for edge in edges {
        assert!(object.edge_endpoints(edge).is_some());
        assert!(matches!(
            roundover(&object, edge, 0.003175, 0.0001),
            Err(EdgeTreatmentError::UnsupportedTermination)
        ));
    }
    for edge in object.edges().keys() {
        let (a, b) = object.edge_endpoints(edge).unwrap();
        if (a.z - b.z).abs() > 0.1 {
            assert!(matches!(
                roundover(&object, edge, 0.003175, 0.0001),
                Err(EdgeTreatmentError::NonPerpendicularFaces)
            ));
        }
    }
}
fn base() -> (Document, kernel::ObjectId) {
    let mut d = Document::new();
    let s = d.add_sketch(plane(0.0));
    let p = [
        Point3::ORIGIN,
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    ];
    for i in 0..4 {
        d.sketch_mut(s)
            .unwrap()
            .add_segment(p[i], p[(i + 1) % 4])
            .unwrap();
    }
    let r = d.extrudable_regions(s).unwrap()[0];
    let o = d.extrude_region(s, r, 0.1).unwrap().0;
    (d, o)
}
#[test]
fn polygon_extruded_from_a_base_face_is_one_valid_nonconvex_object_and_refuses_atomically() {
    let (mut d, o) = base();
    let top = d
        .object(o)
        .unwrap()
        .faces()
        .iter()
        .find(|(_, f)| f.plane.normal().z > 0.9)
        .unwrap()
        .0;
    let sub = match d
        .apply_object_op(
            o,
            KernelOp::SplitFaceInner {
                face: top,
                loop_path: hex(0.1),
                restore: None,
                curves: vec![],
            },
        )
        .unwrap()
        .0
    {
        KernelOpReport::FaceSplitInner(r) => r.sub_face,
        _ => unreachable!(),
    };
    d.apply_object_op(
        o,
        KernelOp::ExtrudeSubFace {
            sub_face: sub,
            distance: 0.2,
        },
    )
    .unwrap();
    assert_eq!(d.visible_object_ids(), vec![o]);
    let object = d.object(o).unwrap();
    object.validate().unwrap();
    assert_eq!(object.watertight(), WatertightState::Watertight);
    let edges = top_edges(object, 0.3);
    assert_eq!(edges.len(), 6);
    let before = d.save();
    let depth = d.undo_depth();
    for edge in edges {
        assert!(matches!(
            d.prepare_edge_treatment(
                o,
                edge,
                EdgeTreatmentOperation::Roundover {
                    radius: 0.003175,
                    max_chord_error: 0.0001
                }
            ),
            Err(DocumentEdgeTreatmentError::Geometry(
                EdgeTreatmentError::UnsupportedSolid
            ))
        ));
    }
    assert_eq!(d.save(), before);
    assert_eq!(d.undo_depth(), depth);
}
#[test]
fn original_exterior_base_edge_remains_resolved_but_global_convexity_refuses_after_boss() {
    let (mut d, o) = base();
    let edge = top_edges(d.object(o).unwrap(), 0.1)
        .into_iter()
        .find(|&e| {
            let (a, b) = d.object(o).unwrap().edge_endpoints(e).unwrap();
            a.y == 0.0 && b.y == 0.0
        })
        .unwrap();
    let endpoints = d.object(o).unwrap().edge_endpoints(edge).unwrap();
    for operation in [
        EdgeTreatmentOperation::Chamfer { distance: 0.005 },
        EdgeTreatmentOperation::Roundover {
            radius: 0.003175,
            max_chord_error: 0.0001,
        },
    ] {
        assert!(d.prepare_edge_treatment(o, edge, operation).is_ok());
    }
    let top = d
        .object(o)
        .unwrap()
        .faces()
        .iter()
        .find(|(_, f)| f.plane.normal().z > 0.9)
        .unwrap()
        .0;
    let sub = match d
        .apply_object_op(
            o,
            KernelOp::SplitFaceInner {
                face: top,
                loop_path: hex(0.1),
                restore: None,
                curves: vec![],
            },
        )
        .unwrap()
        .0
    {
        KernelOpReport::FaceSplitInner(r) => r.sub_face,
        _ => unreachable!(),
    };
    d.apply_object_op(
        o,
        KernelOp::ExtrudeSubFace {
            sub_face: sub,
            distance: 0.2,
        },
    )
    .unwrap();
    assert_eq!(d.visible_object_ids(), vec![o]);
    assert_eq!(d.object(o).unwrap().edge_endpoints(edge), Some(endpoints));
    assert_eq!(
        d.object(o).unwrap().watertight(),
        WatertightState::Watertight
    );
    d.object(o).unwrap().validate().unwrap();
    let record = &d.object(o).unwrap().edges()[edge];
    let normals = [record.half_edge, record.twin_half_edge.unwrap()].map(|h| {
        let object = d.object(o).unwrap();
        object.faces()[object.loops()[object.half_edges()[h].loop_id].face]
            .plane
            .normal()
    });
    assert!(
        normals[0].dot(normals[1]).abs() < 1e-10,
        "the local incident faces are still perpendicular"
    );
    let source = d.object(o).unwrap();
    assert!(
        source.faces().values().any(|f| source
            .vertices()
            .values()
            .any(|v| f.plane.signed_distance(v.position) > 0.01)),
        "the valid stepped solid is globally nonconvex"
    );
    let before = d.save();
    let depth = d.undo_depth();
    for operation in [
        EdgeTreatmentOperation::Chamfer { distance: 0.005 },
        EdgeTreatmentOperation::Roundover {
            radius: 0.003175,
            max_chord_error: 0.0001,
        },
    ] {
        assert!(matches!(
            d.prepare_edge_treatment(o, edge, operation),
            Err(DocumentEdgeTreatmentError::Geometry(
                EdgeTreatmentError::UnsupportedSolid
            ))
        ));
    }
    assert_eq!(d.save(), before);
    assert_eq!(d.undo_depth(), depth);
}
#[test]
fn an_unmerged_polygon_over_the_base_has_separate_ownership_and_oblique_termination() {
    let (mut d, base) = base();
    let s = d.add_sketch(plane(0.1));
    let p = hex(0.1);
    for i in 0..6 {
        d.sketch_mut(s)
            .unwrap()
            .add_segment(p[i], p[(i + 1) % 6])
            .unwrap();
    }
    let r = d.extrudable_regions(s).unwrap()[0];
    let prism = d.extrude_region(s, r, 0.2).unwrap().0;
    assert_ne!(prism, base);
    assert_eq!(d.visible_object_ids().len(), 2);
    let e = top_edges(d.object(prism).unwrap(), 0.3)[0];
    assert!(d.object(prism).unwrap().edges().contains_key(e));
    assert!(matches!(
        d.prepare_edge_treatment(
            prism,
            e,
            EdgeTreatmentOperation::Roundover {
                radius: 0.003175,
                max_chord_error: 0.0001
            }
        ),
        Err(DocumentEdgeTreatmentError::Geometry(
            EdgeTreatmentError::UnsupportedTermination
        ))
    ));
}
