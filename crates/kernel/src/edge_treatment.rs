//! Pure edge treatments: resolve a local frame, construct a closed tool,
//! evaluate an explicit boolean, and validate the candidate before returning.
//!
//! Supported sections are an equal-distance chamfer and a faceted circular
//! roundover of a complete straight edge between perpendicular planar faces. The
//! source must be one convex, outward-oriented, strictly planar solid with
//! no holes or analytic surface claims; both end faces must be perpendicular
//! to the edge. These restrictions make containment and locality provable
//! using supporting half-spaces, without a second approximate CSG classifier.
//! Section construction shares frame/extrusion/evaluation with no document,
//! history, preview, or Follow Me policy. Path transport remains future work.

use crate::{
    BooleanError, BooleanOp, EdgeId, ExtrudeError, FaceId, Object, Plane, Point3, Profile,
    ProfileError, Transform, Vec3, WatertightState, tol,
};

/// Minimum number of chords in a roundover's quarter-circle.
pub const MIN_ROUNDOVER_SEGMENTS: usize = 4;
/// Bounded geometry/validation cost. A finer request is refused, not clamped.
pub const MAX_ROUNDOVER_SEGMENTS: usize = 128;

/// A new candidate; the source object and all its handles remain untouched.
/// Result handles are fresh, following the existing boolean contract.
#[derive(Debug)]
pub struct EdgeTreatmentResult {
    /// Validated candidate solid, ready for a caller-owned preview or commit.
    pub object: Object,
    /// Analytic volume of the requested cutting section times edge length.
    pub expected_removed_volume: f64,
    /// Source volume minus candidate volume, in cubic meters.
    pub removed_volume: f64,
}

/// Faceted roundover candidate and its approximation evidence. There are no
/// analytic surface claims: export uses the same stored polygonal carrier.
#[derive(Debug)]
pub struct RoundoverResult {
    /// Shared candidate and measured/faceted expected removal, in m³.
    pub result: EdgeTreatmentResult,
    /// Number of chords along the quarter-circle (not a full turn).
    pub segments: usize,
    /// Maximum radial error of the actual constructed arc chords, in meters.
    pub chord_error: f64,
    /// Ideal circular removal, (1 − π/4) r² L. The faceted cut is larger.
    pub ideal_removed_volume: f64,
}

/// Typed refusals; no geometry is clamped, moved, or repaired on failure.
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeTreatmentError {
    /// Missing or stale edge handle.
    UnknownEdge,
    /// Distance must be finite and greater than POINT_MERGE.
    InvalidDistance,
    /// Radius must be finite and greater than POINT_MERGE.
    InvalidRadius,
    /// Maximum chord error must be finite and greater than PLANE_DIST.
    InvalidChordError,
    /// Requested error needs more than MAX_ROUNDOVER_SEGMENTS chords.
    ApproximationTooFine,
    /// Invalid topology, open boundary, bad winding, or improper face contact.
    InvalidSource,
    /// Only one convex planar solid without holes/analytic claims is supported.
    UnsupportedSolid,
    /// The two incident faces must be distinct and perpendicular.
    NonPerpendicularFaces,
    /// Both ends need a unique face perpendicular to the edge.
    UnsupportedTermination,
    /// The tool reaches another face or leaves the source envelope.
    InsufficientClearance,
    /// The cut cannot be resolved by the volume check at this scale.
    UnresolvableCut,
    /// Closed section construction failed.
    Profile(ProfileError),
    /// Closed tool construction failed.
    Extrusion(ExtrudeError),
    /// Explicit subtraction failed; never falls back to union.
    Boolean(BooleanError),
    /// Candidate is not a valid convex watertight solid.
    InvalidResult,
    /// Candidate leaves the original solid or enters the removed half-space.
    ContainmentFailed,
    /// Candidate removed a different volume than requested.
    VolumeMismatch,
}

impl std::fmt::Display for EdgeTreatmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownEdge => write!(f, "edge handle does not resolve"),
            Self::InvalidDistance => write!(f, "chamfer distance must be finite and positive"),
            Self::InvalidRadius => write!(f, "roundover radius must be finite and positive"),
            Self::InvalidChordError => write!(
                f,
                "roundover chord error must be finite and greater than the plane tolerance"
            ),
            Self::ApproximationTooFine => {
                write!(f, "roundover chord error requires too many segments")
            }
            Self::InvalidSource => write!(f, "source is not a valid outward watertight solid"),
            Self::UnsupportedSolid => write!(
                f,
                "requires one convex planar solid without holes or analytic surfaces"
            ),
            Self::NonPerpendicularFaces => write!(f, "edge faces must be perpendicular"),
            Self::UnsupportedTermination => write!(f, "edge ends require perpendicular end faces"),
            Self::InsufficientClearance => {
                write!(f, "edge treatment reaches another face or leaves the solid")
            }
            Self::UnresolvableCut => write!(f, "edge treatment cannot be resolved at this scale"),
            Self::Profile(e) => write!(f, "edge treatment profile: {e}"),
            Self::Extrusion(e) => write!(f, "edge treatment tool: {e}"),
            Self::Boolean(e) => write!(f, "edge treatment subtraction: {e}"),
            Self::InvalidResult => write!(f, "edge treatment produced an invalid solid"),
            Self::ContainmentFailed => write!(f, "edge treatment candidate violates containment"),
            Self::VolumeMismatch => write!(f, "edge treatment removed an unexpected volume"),
        }
    }
}

impl std::error::Error for EdgeTreatmentError {}

/// Equal-distance chamfer along one complete straight edge. Distance is
/// measured along each adjacent face, in meters. Orientation comes solely
/// from topology and face normals, never a global axis.
///
/// Preserves the source by construction (`&Object`). Successful output is
/// contained in the source and is the source clipped by the bevel plane,
/// within the kernel's named length/volume tolerances. Only the triangular
/// prism at the selected edge is removed. This preserves geometry, not
/// boolean-rebuilt face handles or edge attribution.
pub fn chamfer(
    source: &Object,
    edge: EdgeId,
    distance: f64,
) -> Result<EdgeTreatmentResult, EdgeTreatmentError> {
    if !distance.is_finite() || distance <= tol::POINT_MERGE {
        return Err(EdgeTreatmentError::InvalidDistance);
    }
    if !source.edges().contains_key(edge) {
        return Err(EdgeTreatmentError::UnknownEdge);
    }
    validate_convex_solid(source)?;
    let frame = resolve_edge_frame(source, edge)?;
    let ring = vec![
        frame.start,
        frame.start + frame.inward[0] * distance,
        frame.start + frame.inward[1] * distance,
    ];
    let cutter = extrude_section(source, &frame, ring)?;
    let bevel = Plane::from_point_normal(
        frame.start + frame.inward[0] * distance,
        -(frame.inward[0] + frame.inward[1]),
    )
    .map_err(|_| EdgeTreatmentError::NonPerpendicularFaces)?;
    evaluate_cut(
        source,
        &cutter,
        &[bevel],
        distance * distance * frame.length / 2.0,
    )
}

/// Subtractive, faceted circular roundover on one complete straight convex
/// edge, with the same source/termination restrictions as [`chamfer`].
/// Radius and maximum chord error are in meters. The latter must exceed
/// PLANE_DIST: that amount is reserved for point-placement rounding. The
/// stable form of r(1 − cos(Δθ/2)), 2r sin²(Δθ/4), chooses the smallest
/// permitted segment count meeting the remaining error budget. Requests
/// exceeding the geometry cap fail rather than silently losing precision.
///
/// The removed section is the corner OUTSIDE a quarter-circle centered one
/// radius inward from each incident face. Its chords cut slightly more than
/// the ideal circle, while adding no material. The returned expected volume
/// uses the actual polygon area; ideal volume is only a quality reference.
pub fn roundover(
    source: &Object,
    edge: EdgeId,
    radius: f64,
    max_chord_error: f64,
) -> Result<RoundoverResult, EdgeTreatmentError> {
    if !radius.is_finite() || radius <= tol::POINT_MERGE {
        return Err(EdgeTreatmentError::InvalidRadius);
    }
    if !max_chord_error.is_finite() || max_chord_error <= tol::PLANE_DIST {
        return Err(EdgeTreatmentError::InvalidChordError);
    }
    if !(radius * radius).is_finite() {
        return Err(EdgeTreatmentError::UnresolvableCut);
    }
    let segments = roundover_segments(radius, max_chord_error)?;
    if !source.edges().contains_key(edge) {
        return Err(EdgeTreatmentError::UnknownEdge);
    }
    validate_convex_solid(source)?;
    let frame = resolve_edge_frame(source, edge)?;
    let center = frame.start + frame.inward[0] * radius + frame.inward[1] * radius;
    let mut ring = Vec::with_capacity(segments + 2);
    ring.push(frame.start);
    for i in 0..=segments {
        // Exact tangent endpoints avoid sin/cos endpoint residues. Interior
        // points use a local f64 frame; no f32 render data enters the kernel.
        let (x, y) = if i == 0 {
            (radius, 0.0)
        } else if i == segments {
            (0.0, radius)
        } else {
            let theta = std::f64::consts::FRAC_PI_2 * i as f64 / segments as f64;
            (radius * (1.0 - theta.sin()), radius * (1.0 - theta.cos()))
        };
        ring.push(frame.start + frame.inward[0] * x + frame.inward[1] * y);
    }
    // Refuse oversized/overflowing placement before constructing planes or
    // topology. In particular, finite but enormous radii must not reach the
    // extrusion builder's debug invariant assertions.
    check_clearance(source, &frame, &ring)?;
    let mut boundaries = Vec::with_capacity(segments);
    let mut chord_error: f64 = 0.0;
    for pair in ring[1..].windows(2) {
        let (p, q) = (pair[0], pair[1]);
        let chord = q - p;
        if chord.length() <= tol::POINT_MERGE {
            return Err(EdgeTreatmentError::UnresolvableCut);
        }
        // Radial distance is minimized at the closest point on a chord,
        // maximized at an endpoint. Check actual world-coordinate samples,
        // including placement error, rather than reporting a theoretical bound.
        for offset in [Vec3::ZERO, frame.direction * frame.length] {
            let (p, q, center) = (p + offset, q + offset, center + offset);
            let chord = q - p;
            let t = ((center - p).dot(chord) / chord.length_squared()).clamp(0.0, 1.0);
            for point in [p, q, p + chord * t] {
                chord_error = chord_error.max(((point - center).length() - radius).abs());
            }
        }
        let mut normal = frame.direction.cross(chord);
        if normal.dot(p - center) < 0.0 {
            normal = -normal;
        }
        boundaries.push(
            Plane::from_point_normal(p, normal).map_err(|_| EdgeTreatmentError::UnresolvableCut)?,
        );
    }
    if !chord_error.is_finite() || chord_error > max_chord_error {
        return Err(EdgeTreatmentError::UnresolvableCut);
    }
    // Oriented fan area about the corner includes negative triangles for
    // concave sections if needed; for this ring the fan covers the section.
    let twice_area: f64 = ring[1..]
        .windows(2)
        .map(|pair| {
            (pair[0] - frame.start)
                .cross(pair[1] - frame.start)
                .dot(frame.direction)
        })
        .sum();
    let expected = twice_area.abs() * frame.length / 2.0;
    let cutter = extrude_section(source, &frame, ring)?;
    let result = evaluate_cut(source, &cutter, &boundaries, expected)?;
    Ok(RoundoverResult {
        result,
        segments,
        chord_error,
        ideal_removed_volume: (1.0 - std::f64::consts::FRAC_PI_4) * radius * radius * frame.length,
    })
}

fn roundover_segments(radius: f64, max_error: f64) -> Result<usize, EdgeTreatmentError> {
    (MIN_ROUNDOVER_SEGMENTS..=MAX_ROUNDOVER_SEGMENTS)
        .find(|&n| {
            let quarter_step = std::f64::consts::FRAC_PI_2 / (4.0 * n as f64);
            2.0 * radius * quarter_step.sin().powi(2) <= max_error - tol::PLANE_DIST
        })
        .ok_or(EdgeTreatmentError::ApproximationTooFine)
}

fn extrude_section(
    source: &Object,
    frame: &EdgeFrame,
    mut ring: Vec<Point3>,
) -> Result<Object, EdgeTreatmentError> {
    check_clearance(source, frame, &ring)?;
    // Keep the corner fixed when winding the section along the actual edge.
    // For a triangle this is exactly the original chamfer's swap(1, 2).
    if (ring[1] - ring[0])
        .cross(ring[2] - ring[0])
        .dot(frame.direction)
        < 0.0
    {
        ring[1..].reverse();
    }
    let plane = Plane::from_point_normal(frame.start, frame.direction)
        .map_err(|_| EdgeTreatmentError::InvalidSource)?;
    let section = Profile::new(plane, ring, vec![]).map_err(EdgeTreatmentError::Profile)?;
    let cutter =
        Object::from_extrusion(&section, frame.length).map_err(EdgeTreatmentError::Extrusion)?;
    Ok(cutter)
}

struct EdgeFrame {
    start: Point3,
    direction: Vec3,
    length: f64,
    inward: [Vec3; 2],
    faces: [FaceId; 2],
    caps: [FaceId; 2],
}

fn resolve_edge_frame(source: &Object, edge: EdgeId) -> Result<EdgeFrame, EdgeTreatmentError> {
    let e = &source.edges()[edge];
    let twin = e.twin_half_edge.ok_or(EdgeTreatmentError::InvalidSource)?;
    let faces = [e.half_edge, twin].map(|h| source.loops()[source.half_edges()[h].loop_id].face);
    let normals = faces.map(|id| source.faces()[id].plane.normal());
    if faces[0] == faces[1] || normals[0].dot(normals[1]).abs() > tol::NORMAL_DIRECTION {
        return Err(EdgeTreatmentError::NonPerpendicularFaces);
    }
    let (start, end) = source
        .edge_endpoints(edge)
        .ok_or(EdgeTreatmentError::UnknownEdge)?;
    let length = (end - start).length();
    if !length.is_finite() || length <= tol::POINT_MERGE {
        return Err(EdgeTreatmentError::InvalidSource);
    }
    let direction = (end - start) / length;
    if normals
        .iter()
        .any(|n| n.dot(direction).abs() > tol::NORMAL_DIRECTION)
    {
        return Err(EdgeTreatmentError::NonPerpendicularFaces);
    }
    let mut caps = [None, None];
    for (id, face) in source.faces() {
        if faces.contains(&id) {
            continue;
        }
        for (i, point) in [start, end].into_iter().enumerate() {
            if face.plane.signed_distance(point).abs() <= tol::PLANE_DIST {
                let expected = if i == 0 { -direction } else { direction };
                if !face
                    .plane
                    .normal()
                    .approx_eq(expected, tol::NORMAL_DIRECTION)
                    || caps[i].replace(id).is_some()
                {
                    return Err(EdgeTreatmentError::UnsupportedTermination);
                }
            }
        }
    }
    let [Some(first), Some(last)] = caps else {
        return Err(EdgeTreatmentError::UnsupportedTermination);
    };
    Ok(EdgeFrame {
        start,
        direction,
        length,
        inward: normals.map(|n| -n),
        faces,
        caps: [first, last],
    })
}

fn check_clearance(
    source: &Object,
    frame: &EdgeFrame,
    ring: &[Point3],
) -> Result<(), EdgeTreatmentError> {
    for (id, face) in source.faces() {
        for &point in ring {
            for offset in [Vec3::ZERO, frame.direction * frame.length] {
                let p = point + offset;
                let d = face.plane.signed_distance(p);
                let boundary = frame.faces.contains(&id) || frame.caps.contains(&id);
                if !p.x.is_finite()
                    || !p.y.is_finite()
                    || !p.z.is_finite()
                    || !d.is_finite()
                    || d > tol::PLANE_DIST
                    || (!boundary && d >= -tol::PLANE_DIST)
                {
                    return Err(EdgeTreatmentError::InsufficientClearance);
                }
            }
        }
    }
    Ok(())
}

// A convex source plus its two supporting end planes makes the removed
// corner exactly this bounded prism. The retained set is S intersected
// with the section chords' negative half-spaces. A valid convex candidate contained
// in that set, with matching volume, is the desired solid in exact arithmetic.
// The checks below bound that claim by PLANE_DIST and the volume budget;
// they do not promise bit-identical outside faces or topology.
fn evaluate_cut(
    source: &Object,
    cutter: &Object,
    boundaries: &[Plane],
    expected: f64,
) -> Result<EdgeTreatmentResult, EdgeTreatmentError> {
    let origin = source
        .vertices()
        .values()
        .next()
        .ok_or(EdgeTreatmentError::InvalidSource)?
        .position;
    let before = signed_volume(source, origin);
    let budget = tol::EDGE_TREATMENT_VOLUME_REL * expected
        + tol::EDGE_TREATMENT_VOLUME_ULPS * f64::EPSILON * before;
    if !expected.is_finite() || expected <= budget {
        return Err(EdgeTreatmentError::UnresolvableCut);
    }
    // Boolean currently transforms B even for IDENTITY. apply_plane refits
    // a world-space triangle and can lose planarity far from the origin;
    // apply_transform's debug assertion runs before we can validate its
    // result. Preflight precisely that plane mapping on a disposable copy,
    // refusing precision loss without catching panics or changing the tool.
    // This guard is local to edge treatments; no Follow Me/boolean policy
    // changes. A future fallible validated-transform API can replace it.
    let mut mapped_tool = cutter.clone();
    for face in mapped_tool.faces.values_mut() {
        face.plane = Transform::IDENTITY
            .apply_plane(&face.plane)
            .map_err(|_| EdgeTreatmentError::UnresolvableCut)?;
    }
    mapped_tool
        .validate()
        .map_err(|_| EdgeTreatmentError::UnresolvableCut)?;
    let object = Object::boolean(BooleanOp::Subtract, source, cutter, &Transform::IDENTITY)
        .map_err(EdgeTreatmentError::Boolean)?;
    validate_convex_solid(&object).map_err(|_| EdgeTreatmentError::InvalidResult)?;
    for vertex in object.vertices().values() {
        if boundaries
            .iter()
            .any(|plane| plane.signed_distance(vertex.position) > tol::PLANE_DIST)
            || source
                .faces()
                .values()
                .any(|face| face.plane.signed_distance(vertex.position) > tol::PLANE_DIST)
        {
            return Err(EdgeTreatmentError::ContainmentFailed);
        }
    }
    let removed = before - signed_volume(&object, origin);
    if !removed.is_finite() || removed <= 0.0 || (removed - expected).abs() > budget {
        return Err(EdgeTreatmentError::VolumeMismatch);
    }
    Ok(EdgeTreatmentResult {
        object,
        expected_removed_volume: expected,
        removed_volume: removed,
    })
}

fn validate_convex_solid(object: &Object) -> Result<(), EdgeTreatmentError> {
    object
        .validate()
        .map_err(|_| EdgeTreatmentError::InvalidSource)?;
    if object.watertight() != WatertightState::Watertight {
        return Err(EdgeTreatmentError::InvalidSource);
    }
    if object.face_components().len() != 1 {
        return Err(EdgeTreatmentError::UnsupportedSolid);
    }
    for face in object.faces().values() {
        if face.surface.is_some() || !face.inner_loops.is_empty() {
            return Err(EdgeTreatmentError::UnsupportedSolid);
        }
        Profile::new(
            face.plane,
            object.loop_positions(face.outer_loop).collect(),
            vec![],
        )
        .map_err(|_| EdgeTreatmentError::InvalidSource)?;
        if object
            .vertices()
            .values()
            .any(|v| face.plane.signed_distance(v.position) > tol::PLANE_DIST)
        {
            return Err(EdgeTreatmentError::UnsupportedSolid);
        }
    }
    let ids: Vec<_> = object.faces().keys().collect();
    for (i, &a) in ids.iter().enumerate() {
        for &b in &ids[i + 1..] {
            if crate::ops::faces_improperly_contact(object, a, b) {
                return Err(EdgeTreatmentError::InvalidSource);
            }
        }
    }
    let origin = object
        .vertices()
        .values()
        .next()
        .ok_or(EdgeTreatmentError::InvalidSource)?
        .position;
    let volume = signed_volume(object, origin);
    if !volume.is_finite() || volume <= 0.0 {
        return Err(EdgeTreatmentError::InvalidSource);
    }
    Ok(())
}

// Divergence-theorem volume, using a local origin to avoid cancellation
// from a large world translation. Unlike an absolute-volume test this also
// rejects an inside-out source. Keep private until another caller needs it.
fn signed_volume(object: &Object, origin: Point3) -> f64 {
    let mut six_volume = 0.0;
    for face in object.faces().values() {
        for lid in std::iter::once(face.outer_loop).chain(face.inner_loops.iter().copied()) {
            let points: Vec<_> = object.loop_positions(lid).map(|p| p - origin).collect();
            for i in 1..points.len().saturating_sub(1) {
                six_volume += points[0].dot(points[i].cross(points[i + 1]));
            }
        }
    }
    six_volume / 6.0
}
