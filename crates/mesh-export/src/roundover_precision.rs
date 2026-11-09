//! Certification of a known roundover candidate at STL's actual coordinate
//! precision. No geometry is changed and no persistent curve claims are added.
use super::{ExportError, write_binary_stl};
use kernel::{EdgeId, Object, Point3, Vec3, tol};

/// Certify a candidate returned by `kernel::edge_treatment::roundover(source,
/// edge, radius, error)` at its current world coordinates. This is deliberately
/// a host preflight, not a new kernel tessellation policy. Later transforms or
/// unrelated imported polygon meshes have no retained radius/error metadata and
/// must not be claimed to inherit this certificate.
///
/// Uses the actual STL writer, including its topology/integrity checks. On each
/// transition triangle, radial distance from the reference cylinder is convex,
/// hence its maximum is at a vertex. A separating half-space proves the axis is
/// outside its perpendicular projection, so the minimum is on a projected edge.
/// Checking all three edges includes quantization-induced twist at either cap.
/// Returns the conservative maximum radial deviation (meters), or refuses.
pub fn certify_roundover_stl(
    source: &Object,
    edge: EdgeId,
    candidate: &Object,
    radius: f64,
    requested: f64,
) -> Result<f64, ExportError> {
    let refusal = || {
        ExportError::StlIntegrity(
            "the requested roundover chord tolerance cannot be certified at STL coordinate precision; use a finer chord setting or move closer to the origin",
        )
    };
    if !radius.is_finite() || !requested.is_finite() || radius <= 0.0 || requested <= 0.0 {
        return Err(refusal());
    }
    let edge_record = source.edges().get(edge).ok_or_else(refusal)?;
    let twin = edge_record.twin_half_edge.ok_or_else(refusal)?;
    let normals = [edge_record.half_edge, twin].map(|h| {
        source.faces()[source.loops()[source.half_edges()[h].loop_id].face]
            .plane
            .normal()
    });
    let (start, finish) = source.edge_endpoints(edge).ok_or_else(refusal)?;
    let axis = (finish - start).normalized().map_err(|_| refusal())?;
    let center = start - normals[0] * radius - normals[1] * radius;
    let (soup, faces) =
        tessellate::export_triangles_with_faces(candidate, 0).map_err(ExportError::Failed)?;
    let range = 0..faces.len();
    let bytes = write_binary_stl(&soup, std::slice::from_ref(&range))?;
    let mut maximum = 0.0_f64;
    let mut scale = radius.max((finish - start).length());
    let mut checked = 0;
    for (index, &face_id) in faces.iter().enumerate() {
        let face = &candidate.faces()[face_id];
        let raw = &soup[index * 9..index * 9 + 9];
        let witness = Point3::new(raw[0], raw[1], raw[2]);
        // Inherited side/cap faces are not circular transition surfaces.
        if source.faces().values().any(|old| {
            old.plane
                .normal()
                .approx_eq(face.plane.normal(), tol::NORMAL_DIRECTION)
                && old.plane.signed_distance(witness).abs() <= tol::PLANE_DIST
        }) {
            continue;
        }
        checked += 1;
        let normal = face.plane.normal();
        let radial_normal = (normal - axis * normal.dot(axis))
            .normalized()
            .map_err(|_| refusal())?;
        let mut triangle = [Vec3::ZERO; 3];
        for (corner, projected) in triangle.iter_mut().enumerate() {
            let offset = 84 + 50 * index + 12 + 12 * corner;
            let mut coordinate = [0.0; 3];
            for (dimension, value) in coordinate.iter_mut().enumerate() {
                let at = offset + 4 * dimension;
                *value =
                    f64::from(f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())) / 1000.0;
                scale = scale.max(value.abs());
            }
            let delta = Point3::new(coordinate[0], coordinate[1], coordinate[2]) - center;
            *projected = delta - axis * delta.dot(axis);
        }
        // This numerical budget is separate from STL quantization (which is
        // measured exactly above). Refuse ill-conditioned/large placements.
        let arithmetic = 64.0 * f64::EPSILON * scale;
        if triangle
            .iter()
            .any(|p| !p.length_squared().is_finite() || p.dot(radial_normal) <= arithmetic)
        {
            return Err(refusal());
        }
        for i in 0..3 {
            let a = triangle[i];
            let b = triangle[(i + 1) % 3];
            let chord = b - a;
            let t = if chord.length_squared() > 0.0 {
                (-a.dot(chord) / chord.length_squared()).clamp(0.0, 1.0)
            } else {
                0.0
            };
            for point in [a, b, a + chord * t] {
                maximum = maximum.max((point.length() - radius).abs());
            }
        }
    }
    let bound = maximum + 64.0 * f64::EPSILON * scale;
    if checked == 0 || !bound.is_finite() || bound > requested {
        return Err(refusal());
    }
    Ok(bound)
}
