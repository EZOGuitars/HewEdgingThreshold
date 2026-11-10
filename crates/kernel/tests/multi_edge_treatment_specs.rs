use kernel::{
    Object, Plane, Point3, Profile, Transform, Vec3, WatertightState,
    edge_treatment::{self, EdgeTreatmentError},
};
fn block() -> Object {
    let p = Profile::new(
        Plane::from_point_normal(Point3::ORIGIN, Vec3::new(0.0, 0.0, 1.0)).unwrap(),
        vec![
            Point3::ORIGIN,
            Point3::new(0.3, 0.0, 0.0),
            Point3::new(0.3, 0.2, 0.0),
            Point3::new(0.0, 0.2, 0.0),
        ],
        vec![],
    )
    .unwrap();
    Object::from_extrusion(&p, 0.1).unwrap()
}
fn at_origin(o: &Object) -> Vec<kernel::EdgeId> {
    o.edges()
        .keys()
        .filter(|&e| {
            let (a, b) = o.edge_endpoints(e).unwrap();
            a == Point3::ORIGIN || b == Point3::ORIGIN
        })
        .collect()
}
fn valid(o: &Object) {
    o.validate().unwrap();
    assert_eq!(o.watertight(), WatertightState::Watertight);
    assert_eq!(o.shells().len(), 1);
}
#[test]
fn simultaneous_chamfers_resolve_connected_and_disconnected_subsets() {
    let s = block();
    let at = at_origin(&s);
    assert_eq!(at.len(), 3);
    let all: Vec<_> = s.edges().keys().collect();
    let top = s
        .faces()
        .iter()
        .find(|(_, f)| f.plane.normal().z > 0.9)
        .unwrap()
        .0;
    let perimeter = edge_treatment::face_perimeter_edges(&s, top).unwrap();
    for edges in [
        &at[..2],
        &at[..],
        &perimeter[..],
        &[perimeter[0], perimeter[2]][..],
        &all[..],
    ] {
        let r = edge_treatment::chamfer_edges(&s, edges, 0.00635).unwrap();
        valid(&r.result.object);
        assert!((r.result.expected_removed_volume - r.result.removed_volume).abs() < 1e-12);
    }
}
#[test]
fn simultaneous_roundovers_have_miters_and_spherical_three_edge_corners() {
    let s = block();
    let at = at_origin(&s);
    let all: Vec<_> = s.edges().keys().collect();
    let top = s
        .faces()
        .iter()
        .find(|(_, f)| f.plane.normal().z > 0.9)
        .unwrap()
        .0;
    let perimeter = edge_treatment::face_perimeter_edges(&s, top).unwrap();
    for edges in [
        &at[..2],
        &at[..],
        &perimeter[..],
        &[perimeter[0], perimeter[2]][..],
        &all[..],
    ] {
        let r = edge_treatment::roundover_edges(&s, edges, 0.00635, 0.00005).unwrap();
        valid(&r.result.object);
        assert!((r.result.expected_removed_volume - r.result.removed_volume).abs() < 1e-12);
        let spheres = r
            .surfaces
            .iter()
            .filter(|(_, p)| matches!(p, edge_treatment::TreatmentSurface::Sphere { .. }))
            .count();
        assert_eq!(spheres > 0, edges.len() == 3 || edges.len() == 12);
    }
}
#[test]
fn order_is_deterministic_and_invalid_batches_preserve_source() {
    let s = block();
    let before = format!("{s:?}");
    let mut e = at_origin(&s);
    let a = edge_treatment::roundover_edges(&s, &e, 0.002, 0.00002).unwrap();
    e.reverse();
    let b = edge_treatment::roundover_edges(&s, &e, 0.002, 0.00002).unwrap();
    assert_eq!(a.result.object.to_polygons(), b.result.object.to_polygons());
    assert!(matches!(
        edge_treatment::roundover_edges(&s, &e, 0.05, 0.00001),
        Err(EdgeTreatmentError::InsufficientClearance)
    ));
    assert!(matches!(
        edge_treatment::chamfer_edges(&s, &[e[0], e[0]], 0.001),
        Err(EdgeTreatmentError::InvalidSelection)
    ));
    assert_eq!(format!("{s:?}"), before);
}
#[test]
fn batch_frames_come_from_geometry_not_world_axes() {
    let mut s = block();
    let edges = at_origin(&s);
    s.apply_transform(
        &Transform::rotation(Vec3::new(1.0, 2.0, 3.0), 0.67)
            .unwrap()
            .then(&Transform::translation(Vec3::new(0.03, -0.02, 0.01))),
    )
    .unwrap();
    valid(
        &edge_treatment::roundover_edges(&s, &edges, 0.00635, 0.00005)
            .unwrap()
            .result
            .object,
    );
}
