use kernel::{
    Object, Plane, Point3, Profile, Transform, Vec3, WatertightState,
    edge_treatment::{self, EdgeTreatmentError},
};
fn prism(points: &[(f64, f64)]) -> Object {
    Object::from_extrusion(
        &Profile::new(
            Plane::from_point_normal(Point3::ORIGIN, Vec3::new(0., 0., 1.)).unwrap(),
            points.iter().map(|&(x, y)| Point3::new(x, y, 0.)).collect(),
            vec![],
        )
        .unwrap(),
        0.04,
    )
    .unwrap()
}
fn perimeter(o: &Object, upper: bool) -> Vec<kernel::EdgeId> {
    let face = o
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
    edge_treatment::face_perimeter_edges(o, face).unwrap()
}
fn valid(source: &Object, result: &edge_treatment::MultiEdgeTreatmentResult) {
    let o = &result.result.object;
    o.validate().unwrap();
    assert_eq!(o.watertight(), WatertightState::Watertight);
    assert_eq!(o.shells().len(), 1);
    assert!(result.result.removed_volume > 0.);
    assert!((result.result.removed_volume - result.result.expected_removed_volume).abs() < 1e-13);
    for v in o.vertices().values() {
        for f in source.faces().values() {
            assert!(f.plane.signed_distance(v.position) < 1e-9);
        }
    }
}
#[test]
fn polygons_and_successive_opposite_bands_share_one_evaluator() {
    let mut shapes = vec![
        vec![(0., 0.), (0.1, 0.), (0.03, 0.07)],
        vec![(0., 0.), (0.1, 0.), (0.07, 0.06), (0.01, 0.06)],
        vec![(0., 0.), (0.1, 0.), (0.1, 0.06), (0., 0.06)],
    ];
    for n in [6, 8, 11] {
        shapes.push(
            (0..n)
                .map(|i| {
                    let t = std::f64::consts::TAU * i as f64 / n as f64;
                    (0.05 * t.cos(), 0.04 * t.sin())
                })
                .collect(),
        );
    }
    for n in [6, 8] {
        shapes.push(
            (0..n)
                .map(|i| {
                    let t = std::f64::consts::TAU * i as f64 / n as f64;
                    (0.05 * t.cos(), 0.05 * t.sin())
                })
                .collect(),
        );
    }
    shapes.push(vec![
        (0., 0.),
        (0.08, 0.),
        (0.11, 0.02),
        (0.09, 0.06),
        (0.03, 0.08),
        (-0.01, 0.04),
    ]);
    for shape in shapes {
        for chamfer in [false, true] {
            let s = prism(&shape);
            let before = format!("{s:?}");
            let upper = perimeter(&s, true);
            let first = if chamfer {
                edge_treatment::chamfer_edges(&s, &upper, 0.003)
            } else {
                edge_treatment::roundover_edges(&s, &upper, 0.003, 0.00002)
            }
            .unwrap();
            valid(&s, &first);
            assert_eq!(format!("{s:?}"), before);
            // Exact Fedora regression dimensions, independent of display rounding.
            let lower = perimeter(&first.result.object, false);
            let second = edge_treatment::roundover_edges(
                &first.result.object,
                &lower,
                0.0003175,
                0.00009906,
            )
            .unwrap();
            valid(&first.result.object, &second);
            let top = perimeter(&second.result.object, true);
            assert!(!top.is_empty());
        }
    }
}
#[test]
fn cap_band_is_orientation_independent_and_rejects_collision() {
    let mut s = prism(&[(0., 0.), (0.1, 0.), (0.07, 0.06), (0.01, 0.06)]);
    let edges = perimeter(&s, true);
    s.apply_transform(
        &Transform::rotation(Vec3::new(1., 2., 3.), 0.73)
            .unwrap()
            .then(&Transform::translation(Vec3::new(0.03, -0.01, 0.02))),
    )
    .unwrap();
    let r = edge_treatment::roundover_edges(&s, &edges, 0.003, 0.00002).unwrap();
    valid(&s, &r);
    assert!(matches!(
        edge_treatment::roundover_edges(&s, &edges, 0.05, 0.00002),
        Err(EdgeTreatmentError::InsufficientClearance)
    ));
    assert!(edge_treatment::chamfer_edges(&s, &[edges[0], edges[0], edges[1]], 0.001).is_err());
}

#[test]
fn actual_curve_ancestry_is_not_silently_reclassified_as_a_polygon() {
    use kernel::{CurveGeom, Sketch};
    let mut sketch =
        Sketch::on_plane(Plane::from_point_normal(Point3::ORIGIN, Vec3::new(0., 0., 1.)).unwrap());
    sketch
        .begin_curve_with(CurveGeom {
            center: Point3::ORIGIN,
            radius: 0.05,
        })
        .unwrap();
    for i in 0..24 {
        let p = |j| {
            let a = std::f64::consts::TAU * j as f64 / 24.;
            Point3::new(0.05 * a.cos(), 0.05 * a.sin(), 0.)
        };
        sketch.add_segment(p(i), p(i + 1)).unwrap();
    }
    sketch.end_curve();
    let region = sketch.regions().keys().next().unwrap();
    let source = Object::from_extrusion(&sketch.profile(region).unwrap(), 0.04).unwrap();
    let before = format!("{source:?}");
    assert!(matches!(
        edge_treatment::roundover_edges(&source, &perimeter(&source, true), 0.003, 0.00002),
        Err(EdgeTreatmentError::SourceResolution)
    ));
    assert_eq!(format!("{source:?}"), before);
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(32))]
    #[test]
    fn convex_cap_bands_preserve_envelope_under_varied_shapes_and_dimensions(n in 3usize..13, width in 0.03f64..0.15, aspect in 0.6f64..1.4, size in 0.0001f64..0.001) {
        let points: Vec<_> = (0..n).map(|i| {
            let t=std::f64::consts::TAU*i as f64/n as f64;
            (width*t.cos(),width*aspect*t.sin())
        }).collect();
        let source=prism(&points);
        let before=format!("{source:?}");
        let upper=perimeter(&source,true);
        let first=edge_treatment::roundover_edges(&source,&upper,size,size*0.02).unwrap();
        valid(&source,&first);
        let lower=perimeter(&first.result.object,false);
        let bound=edge_treatment::treatment_dimension_bound(&first.result.object,&lower).unwrap();
        let second=edge_treatment::chamfer_edges(&first.result.object,&lower,bound).unwrap();
        valid(&first.result.object,&second);
        proptest::prop_assert_eq!(format!("{source:?}"),before);
    }
}
