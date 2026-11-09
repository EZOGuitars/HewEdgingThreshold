//! Exact opposite-corner reproduction: BRep and export tessellation are f64;
//! millimeter float32 STL is a separate, independently decoded quantization.
use kernel::{
    Document, ImportNode, ImportScene, MaterialPalette, MeshRecipe, Object, Plane, Point3, Profile,
    Vec3, edge_treatment::roundover,
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

fn source(side: f64) -> (Object, kernel::EdgeId) {
    let plane = Plane::from_point_normal(Point3::ORIGIN, Vec3::new(0.0, 0.0, 1.0)).unwrap();
    let object = Object::from_extrusion(
        &Profile::new(
            plane,
            vec![
                Point3::ORIGIN,
                Point3::new(side, 0.0, 0.0),
                Point3::new(side, side, 0.0),
                Point3::new(0.0, side, 0.0),
            ],
            vec![],
        )
        .unwrap(),
        side,
    )
    .unwrap();
    let edge = object
        .edges()
        .keys()
        .find(|&e| {
            let (a, b) = object.edge_endpoints(e).unwrap();
            a.x == side && b.x == side && a.y == 0.0 && b.y == 0.0
        })
        .unwrap();
    (object, edge)
}
fn deviation(points: impl IntoIterator<Item = Point3>, side: f64, radius: f64) -> f64 {
    let center = Point3::new(side - radius, radius, 0.0);
    let mut arc: Vec<_> = points
        .into_iter()
        .filter(|p| {
            p.z.abs() < 1e-12
                && p.x >= side - radius - 1e-7
                && p.y <= radius + 1e-7
                && ((*p - center).length() - radius).abs() < 1e-7
        })
        .collect();
    arc.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    arc.dedup();
    assert!(arc.len() >= 5);
    arc.windows(2)
        .map(|p| {
            let d = p[1] - p[0];
            let t = ((center - p[0]).dot(d) / d.length_squared()).clamp(0.0, 1.0);
            [p[0], p[1], p[0] + d * t]
                .into_iter()
                .map(|p| ((p - center).length() - radius).abs())
                .fold(0.0, f64::max)
        })
        .fold(0.0, f64::max)
}
#[test]
fn opposite_corner_exceedance_begins_at_stl_float32_quantization() {
    let side = 0.3048;
    let radius = 0.1016;
    let requested = 0.00001;
    let (source, edge) = source(side);
    let output = roundover(&source, edge, radius, requested).unwrap();
    let theoretical =
        radius * (1.0 - (std::f64::consts::PI / (4.0 * output.segments as f64)).cos());
    let brep = deviation(
        output.result.object.vertices().values().map(|v| v.position),
        side,
        radius,
    );
    let soup = tessellate::export_triangles(&output.result.object, 0).unwrap();
    let tess = deviation(
        soup.chunks_exact(3).map(|p| Point3::new(p[0], p[1], p[2])),
        side,
        radius,
    );
    let quantized = deviation(
        soup.chunks_exact(3).map(|p| {
            Point3::new(
                f64::from((p[0] * 1000.0) as f32) / 1000.0,
                f64::from((p[1] * 1000.0) as f32) / 1000.0,
                f64::from((p[2] * 1000.0) as f32) / 1000.0,
            )
        }),
        side,
        radius,
    );
    let exported = mesh_export::export_stl(&document_from(&output.result.object), 0, true).unwrap();
    let count = u32::from_le_bytes(exported[80..84].try_into().unwrap()) as usize;
    let decoded = deviation(
        (0..count)
            .flat_map(|i| (0..3).map(move |j| (i, j)))
            .map(|(i, j)| {
                let offset = 84 + 50 * i + 12 + 12 * j;
                let read = |k: usize| {
                    f64::from(f32::from_le_bytes(
                        exported[offset + 4 * k..offset + 4 * k + 4]
                            .try_into()
                            .unwrap(),
                    )) / 1000.0
                };
                Point3::new(read(0), read(1), read(2))
            }),
        side,
        radius,
    );
    assert_eq!(decoded, quantized);
    let rendered =
        tessellate::tessellate(&output.result.object, &MaterialPalette::default()).unwrap();
    let render_error = deviation(
        rendered
            .positions
            .chunks_exact(3)
            .map(|p| Point3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]))),
        side,
        radius,
    );
    println!(
        "CHORD_STL,decoded_mm={:.15},render_f32_meters_mm={:.15},triangles={count}",
        decoded * 1000.0,
        render_error * 1000.0
    );
    println!(
        "CHORD_REPRO,theoretical_mm={:.15},brep_mm={:.15},export_tessellation_mm={:.15},stl_quantized_mm={:.15},segments={}",
        theoretical * 1000.0,
        brep * 1000.0,
        tess * 1000.0,
        quantized * 1000.0,
        output.segments
    );
    assert_eq!(output.segments, 56);
    assert!((brep - theoretical).abs() < 1e-14);
    assert_eq!(brep, tess);
    assert!(brep < requested && quantized > requested);
    assert!((quantized * 1000.0 - 0.010002231570535591).abs() < 1e-10);
}

#[test]
fn preparation_certificate_rejects_the_exact_exceedance_without_changing_geometry() {
    for (side, radius, error, expected) in [
        (0.3048, 0.1016, 0.00001, false),
        (0.3048, 0.00635, 0.00001, true),
        (0.3048, 0.1016, 0.0000099, true),
    ] {
        let (source, edge) = source(side);
        let rounded = roundover(&source, edge, radius, error).unwrap();
        let before = format!("{:?}", rounded.result.object);
        let certified = mesh_export::certify_roundover_stl(
            &source,
            edge,
            &rounded.result.object,
            radius,
            error,
        );
        println!(
            "CERTIFY,radius_mm={},requested_mm={},segments={},result={:?}",
            radius * 1000.0,
            error * 1000.0,
            rounded.segments,
            certified
        );
        assert_eq!(certified.is_ok(), expected);
        if let Ok(bound) = certified {
            assert!(bound <= error);
        }
        assert_eq!(format!("{:?}", rounded.result.object), before);
        if radius == 0.1016 && expected {
            assert_eq!(rounded.segments, 57);
        }
    }
}
#[test]
fn certificate_uses_adjacent_faces_and_refuses_large_coordinate_precision() {
    for (translation, angle, error, expect) in [
        (Vec3::new(0.03, -0.02, 0.01), 0.67, 0.00005, true),
        (Vec3::new(1000.0, 1000.0, 1000.0), 0.0, 0.00001, false),
    ] {
        let (mut object, edge) = source(0.3048);
        object
            .apply_transform(
                &kernel::Transform::rotation(Vec3::new(1.0, 2.0, 3.0), angle)
                    .unwrap()
                    .then(&kernel::Transform::translation(translation)),
            )
            .unwrap();
        let result = roundover(&object, edge, 0.00635, error).unwrap();
        assert_eq!(
            mesh_export::certify_roundover_stl(
                &object,
                edge,
                &result.result.object,
                0.00635,
                error
            )
            .is_ok(),
            expect
        );
    }
}
