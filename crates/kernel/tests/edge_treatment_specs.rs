//! Straight edge treatment acceptance tests. Set membership and volumes below are
//! analytic references, independent of the boolean classifier under test.

use kernel::{
    BooleanOp, EdgeId, Object, Plane, Point3, Profile, Transform, Vec3, WatertightState,
    edge_treatment::{
        EdgeTreatmentError, MAX_ROUNDOVER_SEGMENTS, MIN_ROUNDOVER_SEGMENTS, chamfer, roundover,
    },
    tol,
};
use proptest::prelude::*;

fn prism(points: Vec<Point3>, height: f64) -> Object {
    let plane = Plane::from_point_normal(Point3::ORIGIN, Vec3::new(0.0, 0.0, 1.0)).unwrap();
    Object::from_extrusion(&Profile::new(plane, points, vec![]).unwrap(), height).unwrap()
}

// Independent closed form for the area outside the inscribed quarter-circle
// polygon. Production code computes area from actual world-coordinate points.
fn faceted_roundover_area(radius: f64, segments: usize) -> f64 {
    let step = std::f64::consts::FRAC_PI_2 / segments as f64;
    radius * radius * (1.0 - segments as f64 * step.sin() / 2.0)
}

// Analytic membership against regular inscribed-circle chord planes, without
// consulting the candidate's faces or the boolean engine.
fn roundover_keeps(p: Point3, radius: f64, segments: usize) -> bool {
    let step = std::f64::consts::FRAC_PI_2 / segments as f64;
    (0..segments).all(|i| {
        let theta = (i as f64 + 0.5) * step;
        -(p.x - radius) * theta.sin() - (p.y - radius) * theta.cos()
            <= radius * (step / 2.0).cos() + tol::PLANE_DIST
    })
}

#[test]
fn roundover_is_a_rounded_corner_not_a_cylindrical_groove() {
    let source = block(2.0, 3.0, 4.0);
    let before = format!("{source:?}");
    let radius = 0.5;
    let max_error = 0.001;
    let rounded = roundover(&source, origin_edge(&source), radius, max_error).unwrap();
    let result = &rounded.result;
    assert_eq!(format!("{source:?}"), before);
    result.object.validate().unwrap();
    assert_eq!(result.object.watertight(), WatertightState::Watertight);
    assert_eq!(result.object.shells().len(), 1);
    let expected = faceted_roundover_area(radius, rounded.segments) * 4.0;
    assert!((result.expected_removed_volume - expected).abs() < 1e-12);
    assert!((result.removed_volume - expected).abs() < 1e-12);
    let ideal = (1.0 - std::f64::consts::FRAC_PI_4) * radius * radius * 4.0;
    assert!((rounded.ideal_removed_volume - ideal).abs() < 1e-12);
    assert!(expected > ideal);
    assert!(expected - ideal <= std::f64::consts::FRAC_PI_2 * radius * max_error * 4.0);
    assert!(rounded.chord_error <= max_error);
    for z in [0.0, 2.0, 4.0] {
        assert!(!contains(&result.object, Point3::new(0.0, 0.0, z)));
        assert!(
            contains(&result.object, Point3::new(radius, radius, z)),
            "the circle center must remain material"
        );
        assert!(contains(&result.object, Point3::new(radius, 0.0, z)));
        assert!(contains(&result.object, Point3::new(0.0, radius, z)));
    }
    let removed = Object::boolean(
        BooleanOp::Subtract,
        &source,
        &result.object,
        &Transform::IDENTITY,
    )
    .unwrap();
    assert!((volume(&removed) - expected).abs() < 1e-12);
    for v in removed.vertices().values() {
        assert!(v.position.x >= -tol::PLANE_DIST && v.position.x <= radius + tol::PLANE_DIST);
        assert!(v.position.y >= -tol::PLANE_DIST && v.position.y <= radius + tol::PLANE_DIST);
    }
    assert!(matches!(
        Object::boolean(
            BooleanOp::Subtract,
            &result.object,
            &source,
            &Transform::IDENTITY
        ),
        Err(kernel::BooleanError::EmptyResult)
    ));
    for face in source
        .faces()
        .values()
        .filter(|f| f.plane.normal().x > 0.9 || f.plane.normal().y > 0.9)
    {
        let ring: Vec<_> = source.loop_positions(face.outer_loop).collect();
        assert!(result.object.faces().values().any(|f| {
            let other: Vec<_> = result.object.loop_positions(f.outer_loop).collect();
            ring.len() == other.len() && ring.iter().all(|p| other.contains(p))
        }));
    }
}

#[test]
fn roundover_works_on_every_box_edge_and_at_centimeter_scale() {
    for scale in [1.0, 0.01] {
        let source = block(2.0 * scale, 3.0 * scale, 4.0 * scale);
        for edge in source.edges().keys() {
            let (a, b) = source.edge_endpoints(edge).unwrap();
            let radius = 0.2 * scale;
            let rounded = roundover(&source, edge, radius, scale * 0.001).unwrap();
            let expected = faceted_roundover_area(radius, rounded.segments) * (b - a).length();
            assert!(
                (rounded.result.removed_volume - expected).abs()
                    <= expected * tol::EDGE_TREATMENT_VOLUME_REL
            );
        }
    }
}

#[test]
fn large_translation_roundover_returns_a_typed_precision_refusal() {
    let mut source = block(0.04, 0.35, 0.45);
    let edge = origin_edge(&source);
    source
        .apply_transform(&Transform::translation(Vec3::new(
            100_000.0, 100_000.0, -100_000.0,
        )))
        .unwrap();
    source.validate().unwrap();
    let before = format!("{source:?}");
    for radius in [0.00001, 0.006] {
        assert_eq!(
            roundover(&source, edge, radius, radius * 0.001).unwrap_err(),
            EdgeTreatmentError::UnresolvableCut
        );
    }
    assert_eq!(format!("{source:?}"), before);
}

#[test]
fn deleted_object_local_edge_handles_are_rejected() {
    let mut source = block(1.0, 1.0, 1.0);
    let top = source
        .faces()
        .iter()
        .find(|(_, f)| f.plane.normal().z > 0.9)
        .unwrap()
        .0;
    let report = source
        .split_face(
            top,
            &[Point3::new(0.5, 0.0, 1.0), Point3::new(0.5, 1.0, 1.0)],
        )
        .unwrap();
    let stale = report.new_edges[0];
    source.merge_faces(stale).unwrap();
    assert!(!source.edges().contains_key(stale));
    let before = source.encode(&|_| unreachable!());
    assert_eq!(
        chamfer(&source, stale, 0.1).unwrap_err(),
        EdgeTreatmentError::UnknownEdge
    );
    assert_eq!(
        roundover(&source, stale, 0.1, 0.001).unwrap_err(),
        EdgeTreatmentError::UnknownEdge
    );
    assert_eq!(source.encode(&|_| unreachable!()), before);
}

#[test]
fn small_sections_and_large_translations_succeed_or_refuse_without_panicking() {
    for shift in [0.0, 10.0, 100_000.0] {
        for angle in [0.0, 0.67] {
            let mut source = block(0.04, 0.35, 0.45);
            let edge = origin_edge(&source);
            let pose = Transform::rotation(Vec3::new(1.0, 2.0, 3.0), angle)
                .unwrap()
                .then(&Transform::translation(Vec3::new(shift, shift, -shift)));
            source.apply_transform(&pose).unwrap();
            source.validate().unwrap();
            let before = source.encode(&|_| unreachable!());
            for size in [2e-9, 1e-7, 1e-6, 0.00001, 0.006] {
                for result in [
                    chamfer(&source, edge, size),
                    roundover(&source, edge, size, (size * 0.001).max(2e-9)).map(|r| r.result),
                ] {
                    if let Ok(candidate) = result {
                        candidate.object.validate().unwrap();
                        assert_eq!(candidate.object.watertight(), WatertightState::Watertight);
                        assert!(candidate.removed_volume > 0.0);
                    }
                    assert_eq!(source.encode(&|_| unreachable!()), before);
                }
            }
        }
    }
}

#[test]
fn roundover_rejects_bad_parameters_and_unresolvable_precision_without_mutation() {
    let source = block(1.0, 1.0, 1.0);
    let edge = origin_edge(&source);
    let before = format!("{source:?}");
    for radius in [-1.0, 0.0, tol::POINT_MERGE, f64::NAN, f64::INFINITY] {
        assert_eq!(
            roundover(&source, edge, radius, 0.001).unwrap_err(),
            EdgeTreatmentError::InvalidRadius
        );
    }
    for error in [-1.0, 0.0, tol::PLANE_DIST, f64::NAN, f64::INFINITY] {
        assert_eq!(
            roundover(&source, edge, 0.2, error).unwrap_err(),
            EdgeTreatmentError::InvalidChordError
        );
    }
    assert_eq!(
        roundover(&source, edge, 0.2, 2.0 * tol::PLANE_DIST).unwrap_err(),
        EdgeTreatmentError::ApproximationTooFine
    );
    assert_eq!(
        roundover(&source, edge, f64::MAX, 0.001).unwrap_err(),
        EdgeTreatmentError::UnresolvableCut
    );
    assert_eq!(
        roundover(&source, edge, 1e150, 1e149).unwrap_err(),
        EdgeTreatmentError::InsufficientClearance
    );
    assert_eq!(
        roundover(&source, EdgeId::default(), 0.2, 0.001).unwrap_err(),
        EdgeTreatmentError::UnknownEdge
    );
    for radius in [1.0, 2.0] {
        assert_eq!(
            roundover(&source, edge, radius, 0.001).unwrap_err(),
            EdgeTreatmentError::InsufficientClearance
        );
    }
    assert_eq!(format!("{source:?}"), before);
}

#[test]
fn roundover_shares_unsupported_geometry_refusals() {
    for (transform, expected) in [
        (
            Transform::from_affine(&[1.0, 0.2, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
            EdgeTreatmentError::NonPerpendicularFaces,
        ),
        (
            Transform::from_affine(&[1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.2, 0.0, 1.0, 0.0]),
            EdgeTreatmentError::UnsupportedTermination,
        ),
    ] {
        let mut source = block(1.0, 1.0, 1.0);
        let edge = origin_edge(&source);
        source.apply_transform(&transform).unwrap();
        let before = format!("{source:?}");
        assert_eq!(roundover(&source, edge, 0.2, 0.001).unwrap_err(), expected);
        assert_eq!(format!("{source:?}"), before);
    }
    let source = prism(
        vec![
            Point3::ORIGIN,
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(2.0, 1.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(1.0, 2.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
        ],
        1.0,
    );
    assert_eq!(
        roundover(&source, origin_edge(&source), 0.2, 0.001).unwrap_err(),
        EdgeTreatmentError::UnsupportedSolid
    );
    let open = Object::triangle();
    assert_eq!(
        roundover(&open, open.edges().keys().next().unwrap(), 0.2, 0.001).unwrap_err(),
        EdgeTreatmentError::InvalidSource
    );
}

#[test]
fn roundover_segment_bounds_are_enforced_without_silent_quality_clamping() {
    let source = block(2.0, 2.0, 1.0);
    let edge = origin_edge(&source);
    let coarse = roundover(&source, edge, 0.5, 0.5).unwrap();
    assert_eq!(coarse.segments, MIN_ROUNDOVER_SEGMENTS);
    let sag = |n: usize| {
        2.0 * 0.5
            * (std::f64::consts::FRAC_PI_2 / (4.0 * n as f64))
                .sin()
                .powi(2)
    };
    let fine = roundover(
        &source,
        edge,
        0.5,
        sag(MAX_ROUNDOVER_SEGMENTS) + 2.0 * tol::PLANE_DIST,
    )
    .unwrap();
    assert_eq!(fine.segments, MAX_ROUNDOVER_SEGMENTS);
    assert_eq!(
        roundover(&source, edge, 0.5, sag(MAX_ROUNDOVER_SEGMENTS) / 2.0).unwrap_err(),
        EdgeTreatmentError::ApproximationTooFine
    );
}

fn block(width: f64, depth: f64, length: f64) -> Object {
    prism(
        vec![
            Point3::ORIGIN,
            Point3::new(width, 0.0, 0.0),
            Point3::new(width, depth, 0.0),
            Point3::new(0.0, depth, 0.0),
        ],
        length,
    )
}

fn origin_edge(object: &Object) -> EdgeId {
    object
        .edges()
        .keys()
        .find(|&e| {
            let (a, b) = object.edge_endpoints(e).unwrap();
            a.x == 0.0 && a.y == 0.0 && b.x == 0.0 && b.y == 0.0
        })
        .unwrap()
}

fn volume(object: &Object) -> f64 {
    let origin = object.vertices().values().next().unwrap().position;
    object
        .faces()
        .values()
        .map(|face| {
            let points: Vec<_> = object
                .loop_positions(face.outer_loop)
                .map(|p| p - origin)
                .collect();
            (1..points.len() - 1)
                .map(|i| points[0].dot(points[i].cross(points[i + 1])) / 6.0)
                .sum::<f64>()
        })
        .sum()
}

fn contains(object: &Object, point: Point3) -> bool {
    object
        .faces()
        .values()
        .all(|face| face.plane.signed_distance(point) <= tol::PLANE_DIST)
}

#[test]
fn chamfer_removes_only_the_requested_prism_and_never_mutates_source() {
    let source = block(2.0, 3.0, 4.0);
    let before = format!("{source:?}");
    let result = chamfer(&source, origin_edge(&source), 0.5).unwrap();
    assert_eq!(format!("{source:?}"), before);
    result.object.validate().unwrap();
    assert_eq!(result.object.watertight(), WatertightState::Watertight);
    assert_eq!(result.object.shells().len(), 1);
    assert!((result.expected_removed_volume - 0.5).abs() < 1e-12);
    assert!((result.removed_volume - 0.5).abs() < 1e-12);
    assert!((volume(&result.object) - 23.5).abs() < 1e-12);

    // Independent set-difference check: result is inside the original;
    // removed volume is exactly the triangular prism, not a distant cut.
    assert!(matches!(
        Object::boolean(
            BooleanOp::Subtract,
            &result.object,
            &source,
            &Transform::IDENTITY
        ),
        Err(kernel::BooleanError::EmptyResult)
    ));
    let removed = Object::boolean(
        BooleanOp::Subtract,
        &source,
        &result.object,
        &Transform::IDENTITY,
    )
    .unwrap();
    assert!((volume(&removed) - 0.5).abs() < 1e-12);
    for v in removed.vertices().values() {
        let p = v.position;
        assert!(p.x >= -tol::POINT_MERGE && p.y >= -tol::POINT_MERGE);
        assert!(p.x + p.y <= 0.5 + tol::PLANE_DIST);
        assert!(p.z >= -tol::POINT_MERGE && p.z <= 4.0 + tol::POINT_MERGE);
    }
    // The two distant side faces retain their exact polygon boundaries.
    for face in source
        .faces()
        .values()
        .filter(|f| f.plane.normal().x > 0.9 || f.plane.normal().y > 0.9)
    {
        let ring: Vec<_> = source.loop_positions(face.outer_loop).collect();
        assert!(result.object.faces().values().any(|f| {
            let other: Vec<_> = result.object.loop_positions(f.outer_loop).collect();
            other.len() == ring.len() && ring.iter().all(|p| other.contains(p))
        }));
    }
}

#[test]
fn every_box_edge_uses_its_own_adjacent_face_frame() {
    let source = block(2.0, 3.0, 4.0);
    for edge in source.edges().keys() {
        let (a, b) = source.edge_endpoints(edge).unwrap();
        let expected = 0.2 * 0.2 * (b - a).length() / 2.0;
        let result = chamfer(&source, edge, 0.2).unwrap();
        assert!((result.removed_volume - expected).abs() < 1e-12);
    }
}

#[test]
fn remote_slanted_faces_are_preserved_and_do_not_restrict_the_local_frame() {
    let source = prism(
        vec![
            Point3::ORIGIN,
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(2.0, 1.0, 0.0),
            Point3::new(1.0, 2.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
        ],
        3.0,
    );
    let result = chamfer(&source, origin_edge(&source), 0.25).unwrap();
    assert!((result.removed_volume - 0.25 * 0.25 * 3.0 / 2.0).abs() < 1e-12);
    let face = source
        .faces()
        .values()
        .find(|f| f.plane.normal().x > 0.5 && f.plane.normal().y > 0.5)
        .unwrap();
    let ring: Vec<_> = source.loop_positions(face.outer_loop).collect();
    assert!(result.object.faces().values().any(|f| {
        let other: Vec<_> = result.object.loop_positions(f.outer_loop).collect();
        ring.len() == other.len() && ring.iter().all(|p| other.contains(p))
    }));
}

#[test]
fn centimeter_scale_cut_has_analytic_volume() {
    let source = block(0.02, 0.03, 0.04);
    let result = chamfer(&source, origin_edge(&source), 0.002).unwrap();
    let expected = 0.002 * 0.002 * 0.04 / 2.0;
    assert!((result.removed_volume - expected).abs() <= expected * tol::EDGE_TREATMENT_VOLUME_REL);
}

#[test]
fn holes_are_explicitly_unsupported() {
    let plane = Plane::from_point_normal(Point3::ORIGIN, Vec3::new(0.0, 0.0, 1.0)).unwrap();
    let section = Profile::new(
        plane,
        vec![
            Point3::ORIGIN,
            Point3::new(3.0, 0.0, 0.0),
            Point3::new(3.0, 3.0, 0.0),
            Point3::new(0.0, 3.0, 0.0),
        ],
        vec![vec![
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(1.0, 2.0, 0.0),
            Point3::new(2.0, 2.0, 0.0),
            Point3::new(2.0, 1.0, 0.0),
        ]],
    )
    .unwrap();
    let source = Object::from_extrusion(&section, 1.0).unwrap();
    assert_eq!(
        chamfer(&source, origin_edge(&source), 0.1).unwrap_err(),
        EdgeTreatmentError::UnsupportedSolid
    );
}

#[test]
fn invalid_distances_stale_edge_and_excessive_cuts_refuse_without_mutation() {
    let source = block(1.0, 1.0, 1.0);
    let edge = origin_edge(&source);
    let before = format!("{source:?}");
    for distance in [0.0, -0.1, f64::NAN, f64::INFINITY, tol::POINT_MERGE] {
        assert_eq!(
            chamfer(&source, edge, distance).unwrap_err(),
            EdgeTreatmentError::InvalidDistance
        );
    }
    for distance in [1.0, 2.0] {
        assert_eq!(
            chamfer(&source, edge, distance).unwrap_err(),
            EdgeTreatmentError::InsufficientClearance
        );
    }
    assert_eq!(
        chamfer(&source, EdgeId::default(), 0.1).unwrap_err(),
        EdgeTreatmentError::UnknownEdge
    );
    assert_eq!(format!("{source:?}"), before);
}

#[test]
fn non_right_angles_and_slanted_terminations_refuse() {
    let mut source = block(1.0, 1.0, 1.0);
    let edge = origin_edge(&source);
    source
        .apply_transform(&Transform::from_affine(&[
            1.0, 0.2, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0,
        ]))
        .unwrap();
    assert_eq!(
        chamfer(&source, edge, 0.1).unwrap_err(),
        EdgeTreatmentError::NonPerpendicularFaces
    );
    let mut source = block(1.0, 1.0, 1.0);
    let edge = origin_edge(&source);
    source
        .apply_transform(&Transform::from_affine(&[
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.2, 0.0, 1.0, 0.0,
        ]))
        .unwrap();
    assert_eq!(
        chamfer(&source, edge, 0.1).unwrap_err(),
        EdgeTreatmentError::UnsupportedTermination
    );
}

#[test]
fn nonconvex_and_open_sources_refuse() {
    let source = prism(
        vec![
            Point3::ORIGIN,
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(2.0, 1.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(1.0, 2.0, 0.0),
            Point3::new(0.0, 2.0, 0.0),
        ],
        1.0,
    );
    let before = format!("{source:?}");
    assert_eq!(
        chamfer(&source, origin_edge(&source), 0.1).unwrap_err(),
        EdgeTreatmentError::UnsupportedSolid
    );
    assert_eq!(format!("{source:?}"), before);
    let open = Object::triangle();
    assert_eq!(
        chamfer(&open, open.edges().keys().next().unwrap(), 0.1).unwrap_err(),
        EdgeTreatmentError::InvalidSource
    );
}

#[test]
fn inside_out_and_disconnected_sources_refuse() {
    let source = block(1.0, 1.0, 1.0);
    let (positions, mut faces) = source.to_polygons();
    for face in &mut faces {
        face.reverse();
    }
    let reversed = Object::from_polygons(&positions, &faces).unwrap();
    assert!(chamfer(&reversed, origin_edge(&reversed), 0.1).is_err());
    let shifted = positions.iter().map(|p| *p + Vec3::new(3.0, 0.0, 0.0));
    let all_positions: Vec<_> = positions.iter().copied().chain(shifted).collect();
    let (_, faces) = source.to_polygons();
    let shifted_faces = faces
        .iter()
        .map(|face| face.iter().map(|i| i + positions.len()).collect());
    let all_faces: Vec<_> = faces.iter().cloned().chain(shifted_faces).collect();
    let disconnected = Object::from_polygons(&all_positions, &all_faces).unwrap();
    assert_eq!(
        chamfer(&disconnected, origin_edge(&disconnected), 0.1).unwrap_err(),
        EdgeTreatmentError::UnsupportedSolid
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]
    #[test]
    fn roundover_preserves_radius_volume_and_membership_under_rigid_transforms(
        width in 0.01f64..2.0, depth in 0.01f64..2.0, length in 0.01f64..2.0,
        fraction in 0.05f64..0.8, error_fraction in 0.0005f64..0.02,
        angle in -3.0f64..3.0, tx in -10.0f64..10.0,
        ty in -10.0f64..10.0, tz in -10.0f64..10.0,
        qx in -0.1f64..1.1, qy in -0.1f64..1.1, qz in -0.1f64..1.1,
    ) {
        let mut source = block(width, depth, length);
        let edge = origin_edge(&source);
        let radius = width.min(depth) * fraction;
        let max_error = radius * error_fraction;
        let unposed = roundover(&source, edge, radius, max_error).unwrap();
        let pose = Transform::rotation(Vec3::new(1.0, 2.0, 3.0), angle).unwrap()
            .then(&Transform::translation(Vec3::new(tx, ty, tz)));
        let inverse = pose.inverse().unwrap();
        source.apply_transform(&pose).unwrap();
        let before = format!("{source:?}");
        let rounded = roundover(&source, edge, radius, max_error).unwrap();
        let result = &rounded.result;
        prop_assert_eq!(format!("{source:?}"), before);
        prop_assert_eq!(rounded.segments, unposed.segments);
        prop_assert!(result.object.validate().is_ok());
        prop_assert_eq!(result.object.watertight(), WatertightState::Watertight);
        prop_assert!(rounded.chord_error <= max_error);
        let expected = faceted_roundover_area(radius, rounded.segments) * length;
        prop_assert!((result.removed_volume - expected).abs() <= expected * tol::EDGE_TREATMENT_VOLUME_REL);
        let ideal = (1.0 - std::f64::consts::FRAC_PI_4) * radius * radius * length;
        prop_assert!(expected > ideal);
        prop_assert!(expected - ideal <= std::f64::consts::FRAC_PI_2 * radius * max_error * length);
        for vertex in result.object.vertices().values() {
            prop_assert!(contains(&source, vertex.position));
            let local = inverse.apply_point(vertex.position);
            // Arrangement may insert collinear boundary vertices under a
            // rotation. Mutual containment of convex carriers proves shape
            // invariance without incorrectly requiring identical topology.
            prop_assert!(contains(&unposed.result.object, local));
            if local.x <= radius + tol::PLANE_DIST && local.y <= radius + tol::PLANE_DIST {
                let radial = ((local.x - radius).powi(2) + (local.y - radius).powi(2)).sqrt();
                prop_assert!((radial - radius).abs() <= tol::PLANE_DIST);
            }
        }
        for vertex in unposed.result.object.vertices().values() {
            prop_assert!(contains(&result.object, pose.apply_point(vertex.position)));
        }
        let mut arc_chords = 0;
        for edge in result.object.edges().keys() {
            let (a, b) = result.object.edge_endpoints(edge).unwrap();
            let (a, b) = (inverse.apply_point(a), inverse.apply_point(b));
            if a.z.abs() <= tol::PLANE_DIST && b.z.abs() <= tol::PLANE_DIST
                && [a,b].iter().all(|p| p.x <= radius + tol::PLANE_DIST && p.y <= radius + tol::PLANE_DIST)
            {
                let mx = (a.x + b.x) / 2.0;
                let my = (a.y + b.y) / 2.0;
                let error = radius - ((mx - radius).powi(2) + (my - radius).powi(2)).sqrt();
                prop_assert!(error >= -tol::PLANE_DIST && error <= max_error + tol::PLANE_DIST);
                arc_chords += 1;
            }
        }
        prop_assert_eq!(arc_chords, rounded.segments);
        let local = Point3::new(qx * width, qy * depth, qz * length);
        let expected_inside = (0.0..=1.0).contains(&qx) && (0.0..=1.0).contains(&qy)
            && (0.0..=1.0).contains(&qz) && roundover_keeps(local, radius, rounded.segments);
        prop_assert_eq!(contains(&result.object, pose.apply_point(local)), expected_inside);
        // Unaffected material and the removed corner, at both ends and mid-edge.
        for z in [0.0, length / 2.0, length] {
            prop_assert!(contains(&result.object, pose.apply_point(Point3::new(radius, radius, z))));
            prop_assert!(!contains(&result.object, pose.apply_point(Point3::new(0.0, 0.0, z))));
        }
    }

    #[test]
    fn chamfer_is_rotation_translation_and_scale_independent(
        width in 0.1f64..4.0, depth in 0.1f64..4.0, length in 0.1f64..4.0,
        fraction in 0.05f64..0.8, angle in -3.0f64..3.0,
        tx in -100.0f64..100.0, ty in -100.0f64..100.0, tz in -100.0f64..100.0,
        qx in -0.1f64..1.1, qy in -0.1f64..1.1, qz in -0.1f64..1.1,
    ) {
        let mut source = block(width, depth, length);
        let edge = origin_edge(&source);
        let pose = Transform::rotation(Vec3::new(1.0, 2.0, 3.0), angle).unwrap()
            .then(&Transform::translation(Vec3::new(tx, ty, tz)));
        source.apply_transform(&pose).unwrap();
        let before = format!("{source:?}");
        let distance = width.min(depth) * fraction;
        let result = chamfer(&source, edge, distance).unwrap();
        prop_assert_eq!(format!("{source:?}"), before);
        prop_assert!(result.object.validate().is_ok());
        prop_assert_eq!(result.object.watertight(), WatertightState::Watertight);
        let expected = distance * distance * length / 2.0;
        prop_assert!((result.removed_volume - expected).abs() <= expected * 1e-7);
        for v in result.object.vertices().values() {
            prop_assert!(contains(&source, v.position));
        }
        let p = Point3::new(width * qx, depth * qy, length * qz);
        let expected_inside = (0.0..=1.0).contains(&qx) && (0.0..=1.0).contains(&qy)
            && (0.0..=1.0).contains(&qz) && p.x + p.y >= distance;
        prop_assert_eq!(contains(&result.object, pose.apply_point(p)), expected_inside);
        // Retained and removed witnesses on each side of the section,
        // including arbitrarily near both endpoint caps.
        for z in [0.0, length * 0.001, length * 0.5, length] {
            prop_assert!(contains(&result.object, pose.apply_point(Point3::new(distance, distance, z))));
            prop_assert!(!contains(&result.object, pose.apply_point(Point3::new(distance * 0.1, distance * 0.1, z))));
        }
    }
}
