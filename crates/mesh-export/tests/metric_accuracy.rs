//! Independent metric measurements of stored BRep and decoded millimeter STL.
//! Run with --nocapture to retain the per-operation CSV measurement report.
use kernel::{
    Document, ImportNode, ImportScene, MeshRecipe, Object, Plane, Point3, Profile, Vec3,
    WatertightState,
    edge_treatment::{EdgeTreatmentError, MAX_ROUNDOVER_SEGMENTS, chamfer, roundover},
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

fn cube(mm: f64) -> Object {
    let s = mm / 1000.0;
    let plane = Plane::from_point_normal(Point3::ORIGIN, Vec3::new(0.0, 0.0, 1.0)).unwrap();
    Object::from_extrusion(
        &Profile::new(
            plane,
            vec![
                Point3::ORIGIN,
                Point3::new(s, 0.0, 0.0),
                Point3::new(s, s, 0.0),
                Point3::new(0.0, s, 0.0),
            ],
            vec![],
        )
        .unwrap(),
        s,
    )
    .unwrap()
}
fn origin_edge(object: &Object) -> kernel::EdgeId {
    object
        .edges()
        .keys()
        .find(|&e| {
            let (a, b) = object.edge_endpoints(e).unwrap();
            a.x == 0.0 && a.y == 0.0 && b.x == 0.0 && b.y == 0.0
        })
        .unwrap()
}
fn dimensions(points: &[Point3]) -> [f64; 3] {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in points {
        for (i, v) in [p.x, p.y, p.z].into_iter().enumerate() {
            lo[i] = lo[i].min(v);
            hi[i] = hi[i].max(v);
        }
    }
    std::array::from_fn(|i| hi[i] - lo[i])
}
// Tangent setbacks are independently recovered from actual sidewall vertices,
// rather than trusting the parameter or the producer's approximation metadata.
fn setbacks(points: &[Point3], s: f64, eps: f64) -> [f64; 2] {
    let mut r = [s; 2];
    for p in points {
        if p.y.abs() <= eps && p.x > eps {
            r[0] = r[0].min(p.x);
        }
        if p.x.abs() <= eps && p.y > eps {
            r[1] = r[1].min(p.y);
        }
    }
    r
}
fn chord_deviation(a: Point3, b: Point3, radius: f64) -> f64 {
    let c = Point3::new(radius, radius, a.z);
    let d = b - a;
    let t = ((c - a).dot(d) / d.length_squared()).clamp(0.0, 1.0);
    [a, b, a + d * t]
        .into_iter()
        .map(|p| ((p - c).length() - radius).abs())
        .fold(0.0, f64::max)
}
fn stl_points(bytes: &[u8]) -> Vec<[Point3; 3]> {
    let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
    assert_eq!(bytes.len(), 84 + 50 * count);
    (0..count)
        .map(|i| {
            std::array::from_fn(|j| {
                let at = 84 + 50 * i + 12 + 12 * j;
                let read = |k: usize| {
                    f64::from(f32::from_le_bytes(
                        bytes[at + 4 * k..at + 4 * k + 4].try_into().unwrap(),
                    )) / 1000.0
                };
                Point3::new(read(0), read(1), read(2))
            })
        })
        .collect()
}
fn measure(side_mm: f64, radius_mm: f64, chord_mm: f64, rounded: bool) {
    let source = cube(side_mm); // Fresh source on every single operation.
    let s = side_mm / 1000.0;
    let r = radius_mm / 1000.0;
    let requested = chord_mm / 1000.0;
    let edge = origin_edge(&source);
    let (candidate, segments) = if rounded {
        let output = roundover(&source, edge, r, requested).unwrap();
        (output.result.object, output.segments)
    } else {
        (chamfer(&source, edge, r).unwrap().object, 0)
    };
    candidate.validate().unwrap();
    assert_eq!(candidate.watertight(), WatertightState::Watertight);
    let points: Vec<_> = candidate.vertices().values().map(|v| v.position).collect();
    let eps = 1e-11_f64.max(s * 1e-10);
    let dims = dimensions(&points);
    let actual = setbacks(&points, s, eps);
    let radius_error = actual
        .into_iter()
        .map(|v| (v - r).abs())
        .fold(0.0, f64::max);
    let dimension_error = dims.into_iter().map(|v| (v - s).abs()).fold(0.0, f64::max);
    assert!(radius_error <= eps && dimension_error <= eps);
    let on_arc = |p: Point3, tolerance: f64| {
        p.x <= r + tolerance
            && p.y <= r + tolerance
            && (((p.x - r).hypot(p.y - r)) - r).abs() <= tolerance
    };
    let mut deviation = 0.0_f64;
    let mut chords = 0;
    if rounded {
        for e in candidate.edges().keys() {
            let (a, b) = candidate.edge_endpoints(e).unwrap();
            if a.z.abs() <= eps && b.z.abs() <= eps && on_arc(a, eps) && on_arc(b, eps) {
                deviation = deviation.max(chord_deviation(a, b, r));
                chords += 1;
            }
        }
        assert_eq!(chords, segments);
        assert!(deviation <= requested + eps);
    }
    let doc = document_from(&candidate);
    let bytes = mesh_export::export_stl(&doc, 0, true).unwrap();
    let triangles = stl_points(&bytes);
    let mesh: Vec<_> = triangles.iter().flatten().copied().collect();
    let stl_eps = 8.0 * f64::from(f32::EPSILON) * s;
    let stl_dims = dimensions(&mesh);
    let stl_r = setbacks(&mesh, s, stl_eps);
    let stl_error = stl_dims
        .into_iter()
        .map(|v| (v - s).abs())
        .chain(stl_r.into_iter().map(|v| (v - r).abs()))
        .fold(0.0, f64::max);
    assert!(stl_error <= stl_eps);
    let mut stl_chord = 0.0_f64;
    if rounded {
        for t in &triangles {
            let lo = t.iter().map(|p| p.z).fold(f64::INFINITY, f64::min);
            let hi = t.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);
            if hi - lo < s / 2.0 {
                continue;
            } // Only side facets, not cap triangulation diagonals.
            for i in 0..3 {
                let (a, b) = (t[i], t[(i + 1) % 3]);
                if a.z.abs() <= stl_eps
                    && b.z.abs() <= stl_eps
                    && on_arc(a, stl_eps)
                    && on_arc(b, stl_eps)
                {
                    stl_chord = stl_chord.max(chord_deviation(a, b, r));
                }
            }
        }
        assert!(stl_chord <= requested + stl_eps);
    }
    assert_eq!(
        triangles.len(),
        tessellate::export_triangles(&candidate, 0).unwrap().len() / 9
    );
    let mut directed = std::collections::BTreeMap::new();
    let key = |p: Point3| [p.x, p.y, p.z].map(|v| if v == 0.0 { 0 } else { v.to_bits() });
    let witness = Point3::new(s / 2.0, s / 2.0, s / 2.0);
    for (index, t) in triangles.iter().enumerate() {
        let cross = (t[1] - t[0]).cross(t[2] - t[0]);
        assert!(cross.length() > 0.0 && cross.dot(t[0] - witness) > 0.0);
        let at = 84 + 50 * index;
        let normal: [f64; 3] = std::array::from_fn(|k| {
            f64::from(f32::from_le_bytes(
                bytes[at + 4 * k..at + 4 * k + 4].try_into().unwrap(),
            ))
        });
        assert!(cross.dot(Vec3::new(normal[0], normal[1], normal[2])) > 0.0);
        for i in 0..3 {
            *directed
                .entry((key(t[i]), key(t[(i + 1) % 3])))
                .or_insert(0usize) += 1;
        }
    }
    for ((a, b), count) in &directed {
        assert_eq!(*count, 1);
        assert_eq!(directed.get(&(*b, *a)), Some(&1));
    }
    let brep_volume = |object: &Object| {
        object
            .faces()
            .values()
            .map(|face| {
                let p: Vec<_> = object.loop_positions(face.outer_loop).collect();
                (1..p.len() - 1)
                    .map(|i| (p[0] - witness).dot((p[i] - witness).cross(p[i + 1] - witness)) / 6.0)
                    .sum::<f64>()
            })
            .sum::<f64>()
    };
    let measured_removed = brep_volume(&source) - brep_volume(&candidate);
    // Independent signed tetrahedra on decoded STL, not the producer's volume.
    let volume = triangles
        .iter()
        .map(|t| {
            (t[0] - Point3::ORIGIN).dot((t[1] - Point3::ORIGIN).cross(t[2] - Point3::ORIGIN)) / 6.0
        })
        .sum::<f64>();
    let removal = if rounded {
        r * r
            * s
            * (1.0 - segments as f64 * (std::f64::consts::FRAC_PI_2 / segments as f64).sin() / 2.0)
    } else {
        r * r * s / 2.0
    };
    assert!((volume - (s * s * s - removal)).abs() <= s * s * s * 2e-6);
    assert!(volume > 0.0 && volume < s * s * s);
    let ideal = if rounded {
        s * r * r * (1.0 - std::f64::consts::FRAC_PI_4)
    } else {
        removal
    };
    let theoretical_chord = if rounded {
        r * (1.0 - (std::f64::consts::PI / (4.0 * segments as f64)).cos())
    } else {
        0.0
    };
    assert!((deviation - theoretical_chord).abs() <= eps);
    assert!(
        (measured_removed - removal).abs() <= removal * 1e-7 + 128.0 * f64::EPSILON * s.powi(3)
    );
    let mut row = vec![(
        "operation",
        if rounded {
            "roundover".into()
        } else {
            "chamfer".into()
        },
    )];
    let mut add = |name, value: f64| row.push((name, format!("{value:.16e}")));
    add("cube_mm", side_mm);
    add("requested_radius_mm", radius_mm);
    add("requested_chord_mm", chord_mm);
    add("measured_x_mm", actual[0] * 1000.0);
    add("measured_y_mm", actual[1] * 1000.0);
    add("absolute_error_mm", radius_error * 1000.0);
    add("absolute_error_in", radius_error / 0.0254);
    add("relative_error", radius_error / r);
    add("envelope_error_mm", dimension_error * 1000.0);
    add("max_chord_mm", deviation * 1000.0);
    add("stl_dimension_or_radius_error_mm", stl_error * 1000.0);
    add("stl_max_chord_mm", stl_chord * 1000.0);
    add("segments", segments as f64);
    add("cube_in", s / 0.0254);
    add("requested_radius_in", r / 0.0254);
    add("requested_chord_in", requested / 0.0254);
    add("measured_x_in", actual[0] / 0.0254);
    add("measured_y_in", actual[1] / 0.0254);
    for (axis, value) in [
        ("outer_x_mm", dims[0] * 1000.0),
        ("outer_y_mm", dims[1] * 1000.0),
        ("outer_z_mm", dims[2] * 1000.0),
        ("outer_x_in", dims[0] / 0.0254),
        ("outer_y_in", dims[1] / 0.0254),
        ("outer_z_in", dims[2] / 0.0254),
        ("stl_outer_x_mm", stl_dims[0] * 1000.0),
        ("stl_outer_y_mm", stl_dims[1] * 1000.0),
        ("stl_outer_z_mm", stl_dims[2] * 1000.0),
        ("stl_outer_x_in", stl_dims[0] / 0.0254),
        ("stl_outer_y_in", stl_dims[1] / 0.0254),
        ("stl_outer_z_in", stl_dims[2] / 0.0254),
    ] {
        add(axis, value);
    }
    add("max_chord_in", deviation / 0.0254);
    add("formula_chord_mm", theoretical_chord * 1000.0);
    add(
        "chord_formula_error_mm",
        (deviation - theoretical_chord).abs() * 1000.0,
    );
    add("ideal_removed_mm3", ideal * 1e9);
    add("expected_faceted_removed_mm3", removal * 1e9);
    add("measured_removed_mm3", measured_removed * 1e9);
    add("faceting_excess_mm3", (removal - ideal) * 1e9);
    add(
        "removed_volume_error_mm3",
        (measured_removed - removal).abs() * 1e9,
    );
    add("ideal_removed_in3", ideal / 0.0254_f64.powi(3));
    add("expected_faceted_removed_in3", removal / 0.0254_f64.powi(3));
    add(
        "measured_removed_in3",
        measured_removed / 0.0254_f64.powi(3),
    );
    let source_bytes = mesh_export::export_stl(&document_from(&source), 0, true).unwrap();
    let source_stl_volume = stl_points(&source_bytes)
        .iter()
        .map(|t| {
            (t[0] - Point3::ORIGIN).dot((t[1] - Point3::ORIGIN).cross(t[2] - Point3::ORIGIN)) / 6.0
        })
        .sum::<f64>();
    let stl_cut = source_stl_volume - volume;
    add("stl_source_volume_mm3", source_stl_volume * 1e9);
    add("stl_removed_relative_to_source_mm3", stl_cut * 1e9);
    add("stl_cut_volume_error_mm3", (stl_cut - removal).abs() * 1e9);
    add(
        "removed_volume_relative_error",
        (measured_removed - removal).abs() / removal,
    );
    add("stl_measured_removed_mm3", (s.powi(3) - volume) * 1e9);
    add(
        "stl_removed_volume_error_mm3",
        (s.powi(3) - volume - removal).abs() * 1e9,
    );
    add("stl_dimension_error_in", stl_error / 0.0254);
    add("stl_max_chord_in", stl_chord / 0.0254);
    add("triangles", triangles.len() as f64);
    add("topology_valid", 1.0);
    add("watertight", 1.0);
    add("stl_manifold_outward", 1.0);
    static HEADER: std::sync::Once = std::sync::Once::new();
    HEADER.call_once(|| {
        println!(
            "METRIC_HEADER,{}",
            row.iter().map(|(k, _)| *k).collect::<Vec<_>>().join(",")
        )
    });
    println!(
        "METRIC,{}",
        row.into_iter()
            .map(|(_, v)| v)
            .collect::<Vec<_>>()
            .join(",")
    );
}
#[test]
fn metric_cube_radius_chord_and_stl_accuracy_matrix() {
    let mut combinations = 0;
    for side in [10.0, 25.0, 100.0, 300.0, 1000.0] {
        for radius in [0.5, 1.0, 2.0, 3.0, 5.0, 6.0, 10.0, 12.0, 20.0, 25.0, 50.0] {
            if radius > side / 2.0 {
                continue;
            }
            combinations += 1;
            for chord in [0.01, 0.025, 0.05, 0.1, 0.25] {
                measure(side, radius, chord, true);
            }
            measure(side, radius, 0.0, false);
        }
    }
    assert_eq!(combinations, 46);
    // Additional metric equivalents from the final handoff brief.
    for side in [10.0, 25.0, 100.0, 300.0, 1000.0] {
        for radius in [6.35, 12.7] {
            if radius > side / 2.0 {
                continue;
            }
            for chord in [0.01, 0.025, 0.05, 0.1, 0.25] {
                measure(side, radius, chord, true);
            }
            measure(side, radius, 0.0, false);
        }
    }
    // Fresh 12-inch cubes; conversion is exact by definition, represented f64.
    for inches in [0.125, 0.25, 0.375, 0.5, 0.75, 1.0, 2.0, 4.0] {
        for chord in [0.01, 0.025, 0.05, 0.1, 0.25] {
            measure(304.8, inches * 25.4, chord, true);
        }
    }
    for radius in [0.1, 0.25, 0.5, 1.25, 2.54, 6.35, 12.7] {
        measure(100.0, radius, 0.01, true);
    }
}
#[test]
fn segment_cap_is_an_explicit_refusal_not_silent_tolerance_violation() {
    let source = cube(1000.0);
    assert_eq!(MAX_ROUNDOVER_SEGMENTS, 128);
    assert!(matches!(
        roundover(&source, origin_edge(&source), 0.05, 0.0000001),
        Err(EdgeTreatmentError::ApproximationTooFine)
    ));
}

#[test]
fn invalid_cube_radius_combinations_refuse_without_mutation() {
    let source = cube(10.0);
    let before = format!("{source:?}");
    for radius in [0.01, 0.02] {
        assert!(matches!(
            roundover(&source, origin_edge(&source), radius, 0.00001),
            Err(EdgeTreatmentError::InsufficientClearance)
        ));
        assert_eq!(format!("{source:?}"), before);
    }
}
