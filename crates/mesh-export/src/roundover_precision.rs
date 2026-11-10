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

/// Certify all transition patches of a simultaneous treatment using actual
/// serialized triangles. Sphere minima may be inside the triangle, unlike the
/// single-cylinder projected-boundary case.
pub fn certify_treatment_surfaces(
    candidate: &Object,
    references: &[(kernel::FaceId, kernel::edge_treatment::TreatmentSurface)],
    requested: f64,
) -> Result<f64, ExportError> {
    use kernel::edge_treatment::TreatmentSurface;
    let fail = || {
        ExportError::StlIntegrity(
            "the connected roundover cannot preserve its requested chord tolerance at STL precision",
        )
    };
    if !requested.is_finite() || requested <= 0.0 || references.is_empty() {
        return Err(fail());
    }
    let references: std::collections::BTreeMap<_, _> = references.iter().copied().collect();
    let (soup, faces) =
        tessellate::export_triangles_with_faces(candidate, 0).map_err(ExportError::Failed)?;
    let range = 0..faces.len();
    let bytes = write_binary_stl(&soup, std::slice::from_ref(&range))?;
    let mut maximum = 0.0_f64;
    let mut scale = 0.0_f64;
    let mut count = 0;
    for (index, face) in faces.iter().enumerate() {
        let Some(surface) = references.get(face) else {
            continue;
        };
        count += 1;
        let mut points = [Point3::ORIGIN; 3];
        for (j, p) in points.iter_mut().enumerate() {
            let at = 84 + 50 * index + 12 + 12 * j;
            let read = |k: usize| {
                f64::from(f32::from_le_bytes(
                    bytes[at + 4 * k..at + 4 * k + 4].try_into().unwrap(),
                )) / 1000.0
            };
            *p = Point3::new(read(0), read(1), read(2));
            scale = scale.max(p.x.abs()).max(p.y.abs()).max(p.z.abs());
        }
        let (center, radius, axis) = match *surface {
            TreatmentSurface::Cylinder {
                axis_point,
                axis,
                radius,
            } => (axis_point, radius, Some(axis)),
            TreatmentSurface::Sphere { center, radius } => (center, radius, None),
        };
        if !radius.is_finite()
            || radius <= 0.0
            || axis.is_some_and(|a| {
                !a.length().is_finite() || (a.length() - 1.0).abs() > tol::NORMAL_DIRECTION
            })
        {
            return Err(fail());
        }
        scale = scale.max(radius);
        let mut v = points.map(|p| p - center);
        if let Some(axis) = axis {
            for p in &mut v {
                *p = *p - axis * p.dot(axis);
            }
        }
        if v.iter().any(|p| !p.length_squared().is_finite()) {
            return Err(fail());
        }
        let nearest = if axis.is_some() {
            // Cylinder axis must be outside this convex projected triangle.
            let normal = candidate.faces()[*face].plane.normal();
            if v.iter()
                .any(|p| p.dot(normal) <= 64.0 * f64::EPSILON * scale)
            {
                return Err(fail());
            }
            (0..3)
                .map(|i| nearest_segment_origin(v[i], v[(i + 1) % 3]).length())
                .fold(f64::INFINITY, f64::min)
        } else {
            nearest_triangle_origin(v).ok_or_else(fail)?.length()
        };
        if !nearest.is_finite() {
            return Err(fail());
        }
        maximum = maximum.max((radius - nearest).abs());
        for p in v {
            maximum = maximum.max((p.length() - radius).abs());
        }
    }
    let bound = maximum + 64.0 * f64::EPSILON * scale;
    if count == 0 || !bound.is_finite() || bound > requested {
        Err(fail())
    } else {
        Ok(bound)
    }
}
fn nearest_segment_origin(a: Vec3, b: Vec3) -> Vec3 {
    let d = b - a;
    let t = if d.length_squared() > 0.0 {
        (-a.dot(d) / d.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    a + d * t
}
fn nearest_triangle_origin(t: [Vec3; 3]) -> Option<Vec3> {
    let [a, b, c] = t;
    let ab = b - a;
    let ac = c - a;
    let ap = -a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return Some(a);
    }
    let bp = -b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return Some(b);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return Some(a + ab * (d1 / (d1 - d3)));
    }
    let cp = -c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return Some(c);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return Some(a + ac * (d2 / (d2 - d6)));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        return Some(b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6))));
    }
    let denominator = va + vb + vc;
    if denominator <= 0.0 || !denominator.is_finite() {
        return None;
    }
    Some(a + ab * (vb / denominator) + ac * (vc / denominator))
}
