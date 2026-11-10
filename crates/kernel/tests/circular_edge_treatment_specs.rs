use kernel::{
    CurveGeom, Object, Plane, Point3, Sketch, Vec3, WatertightState,
    edge_treatment::{self, EdgeTreatmentError},
};
fn cylinder(n: usize) -> Object {
    let mut s =
        Sketch::on_plane(Plane::from_point_normal(Point3::ORIGIN, Vec3::new(0., 0., 1.)).unwrap());
    s.begin_curve_with(CurveGeom {
        center: Point3::ORIGIN,
        radius: 0.05,
    })
    .unwrap();
    let p = |i: usize| {
        let t = std::f64::consts::TAU * i as f64 / n as f64;
        Point3::new(0.05 * t.cos(), 0.05 * t.sin(), 0.)
    };
    for i in 0..n {
        s.add_segment(p(i), p(i + 1)).unwrap();
    }
    s.end_curve();
    Object::from_extrusion(
        &s.profile(s.regions().keys().next().unwrap()).unwrap(),
        0.04,
    )
    .unwrap()
}
fn edges(s: &Object, upper: bool) -> Vec<kernel::EdgeId> {
    let f = s
        .faces()
        .iter()
        .find(|(_, f)| {
            if upper {
                f.plane.normal().z > 0.999
            } else {
                f.plane.normal().z < -0.999
            }
        })
        .unwrap()
        .0;
    edge_treatment::face_perimeter_edges(s, f).unwrap()
}
#[test]
fn circular_caps_support_both_modes_and_successive_opposite_bands() {
    for upper in [false, true] {
        for chamfer in [false, true] {
            let source = cylinder(64);
            let before = format!("{source:?}");
            let first = if chamfer {
                edge_treatment::chamfer_edges(&source, &edges(&source, upper), 0.003)
            } else {
                edge_treatment::roundover_edges(&source, &edges(&source, upper), 0.003, 0.0003)
            }
            .unwrap();
            let second = edge_treatment::roundover_edges(
                &first.result.object,
                &edges(&first.result.object, !upper),
                0.002,
                0.0003,
            )
            .unwrap();
            for r in [&first, &second] {
                r.result.object.validate().unwrap();
                assert_eq!(r.result.object.watertight(), WatertightState::Watertight);
                assert!(r.result.removed_volume > 0.);
                assert!((r.result.expected_removed_volume - r.result.removed_volume).abs() < 1e-12);
            }
            assert_eq!(format!("{source:?}"), before);
        }
    }
}
#[test]
fn coarse_source_and_overlapping_bands_refuse() {
    let s = cylinder(24);
    assert!(matches!(
        edge_treatment::roundover_edges(&s, &edges(&s, true), 0.003, 0.00002),
        Err(EdgeTreatmentError::SourceResolution)
    ));
    assert!(edge_treatment::chamfer_edges(&s, &edges(&s, true), 0.05).is_err());
}
