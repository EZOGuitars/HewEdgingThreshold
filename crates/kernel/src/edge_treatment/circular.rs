//! Circular cap bands on a convex faceted carrier. Circle ancestry is supplied
//! by existing Cylinder metadata; no second persistent curve representation.
//! Radial samples remain inside the original polygonal cylinder envelope.
use super::multi::{Constraint, intersect_boundary};
use super::*;
use std::collections::BTreeSet;

struct Band {
    center: Point3,
    normal: Vec3,
    radius: f64,
    points: Vec<Point3>,
    walls: Vec<Vec3>,
    area: f64,
    limit: f64,
}
pub(super) fn recognizes(source: &Object, edges: &[EdgeId]) -> bool {
    edges.iter().any(|e| {
        source.edges().get(*e).is_some_and(|e| {
            [Some(e.half_edge), e.twin_half_edge]
                .into_iter()
                .flatten()
                .any(|h| {
                    source.faces()[source.loops()[source.half_edges()[h].loop_id].face]
                        .surface
                        .is_some()
                })
        })
    })
}
impl Band {
    fn resolve(source: &Object, edges: &[EdgeId]) -> Result<Self, EdgeTreatmentError> {
        if !(8..=256).contains(&edges.len()) {
            return Err(EdgeTreatmentError::UnsupportedCurvedBoundary);
        }
        validate_convex_carrier(source, true)?;
        let selected: BTreeSet<_> = edges.iter().copied().collect();
        if selected.len() != edges.len() {
            return Err(EdgeTreatmentError::InvalidSelection);
        }
        let (cap_id, cap) = source
            .faces()
            .iter()
            .find(|(id, _)| {
                face_perimeter_edges(source, *id)
                    .is_ok_and(|e| e.into_iter().collect::<BTreeSet<_>>() == selected)
            })
            .ok_or(EdgeTreatmentError::InvalidSelection)?;
        let points: Vec<_> = source.loop_positions(cap.outer_loop).collect();
        let normal = cap.plane.normal();
        let ordered = face_perimeter_edges(source, cap_id)?;
        let mut walls = Vec::new();
        let mut wall_ids = BTreeSet::new();
        let mut cylinder = None;
        for edge in ordered {
            let e = &source.edges()[edge];
            let faces = [
                e.half_edge,
                e.twin_half_edge.ok_or(EdgeTreatmentError::InvalidSource)?,
            ]
            .map(|h| source.loops()[source.half_edges()[h].loop_id].face);
            let other = if faces[0] == cap_id {
                faces[1]
            } else {
                faces[0]
            };
            let face = &source.faces()[other];
            let reference = face
                .surface
                .ok_or(EdgeTreatmentError::UnsupportedCurvedBoundary)?;
            if cylinder.is_some_and(|c: crate::SurfaceRef| !c.same_surface(&reference))
                || normal.dot(face.plane.normal()).abs() > tol::NORMAL_DIRECTION
            {
                return Err(EdgeTreatmentError::UnsupportedCurvedBoundary);
            }
            cylinder = Some(reference);
            walls.push(face.plane.normal());
            wall_ids.insert(other);
        }
        let crate::SurfaceRef::Cylinder {
            axis_point,
            axis,
            radius,
        } = cylinder.ok_or(EdgeTreatmentError::UnsupportedCurvedBoundary)?;
        if axis.dot(normal).abs() < 1.0 - tol::NORMAL_DIRECTION {
            return Err(EdgeTreatmentError::UnsupportedCurvedBoundary);
        }
        let center = axis_point + axis * (points[0] - axis_point).dot(axis);
        let step = std::f64::consts::TAU / points.len() as f64;
        let mut area = 0.;
        for i in 0..points.len() {
            let a = points[i] - center;
            let b = points[(i + 1) % points.len()] - center;
            if (a.length() - radius).abs() > tol::POINT_MERGE
                || (b.length() - radius).abs() > tol::POINT_MERGE
            {
                return Err(EdgeTreatmentError::UnsupportedCurvedBoundary);
            }
            let angle = a.cross(b).dot(normal).atan2(a.dot(b));
            if (angle - step).abs() > tol::NORMAL_DIRECTION {
                return Err(EdgeTreatmentError::UnsupportedCurvedBoundary);
            }
            area += a.cross(b).dot(normal) / 2.;
        }
        let mut limit = radius;
        for (id, face) in source.faces() {
            if id == cap_id || wall_ids.contains(&id) {
                continue;
            }
            let slope = -face.plane.normal().dot(normal);
            for &p in &points {
                let slack = -face.plane.signed_distance(p);
                if slope > tol::NORMAL_DIRECTION {
                    limit = limit.min(slack / slope);
                } else if slack <= tol::PLANE_DIST {
                    return Err(EdgeTreatmentError::InsufficientClearance);
                }
            }
        }
        if !limit.is_finite() || limit <= 4. * tol::POINT_MERGE {
            return Err(EdgeTreatmentError::InsufficientClearance);
        }
        Ok(Self {
            center,
            normal,
            radius,
            points,
            walls,
            area,
            limit,
        })
    }
}
pub(super) fn geometric_limit(
    source: &Object,
    edges: &[EdgeId],
) -> Result<f64, EdgeTreatmentError> {
    Ok(Band::resolve(source, edges)?.limit)
}
pub(super) fn evaluate(
    source: &Object,
    edges: &[EdgeId],
    size: f64,
    error: Option<f64>,
) -> Result<MultiEdgeTreatmentResult, EdgeTreatmentError> {
    if !size.is_finite() || size <= tol::POINT_MERGE {
        return Err(if error.is_some() {
            EdgeTreatmentError::InvalidRadius
        } else {
            EdgeTreatmentError::InvalidDistance
        });
    }
    let band = Band::resolve(source, edges)?;
    if size >= band.limit - 4. * tol::POINT_MERGE {
        return Err(EdgeTreatmentError::InsufficientClearance);
    }
    let segments = if let Some(error) = error {
        if !error.is_finite() || error <= tol::PLANE_DIST {
            return Err(EdgeTreatmentError::InvalidChordError);
        }
        let angular = band.radius * (1. - (std::f64::consts::PI / edges.len() as f64).cos());
        if 4. * angular >= error {
            return Err(EdgeTreatmentError::SourceResolution);
        }
        (MIN_ROUNDOVER_SEGMENTS..=MAX_ROUNDOVER_SEGMENTS)
            .find(|&n| {
                n * edges.len() <= MAX_BATCH_PLANES
                    && 2.
                        * size
                        * (std::f64::consts::FRAC_PI_2 / (4. * n as f64))
                            .sin()
                            .powi(2)
                        <= error / 4. - tol::PLANE_DIST
            })
            .ok_or(EdgeTreatmentError::ApproximationTooFine)?
    } else {
        0
    };
    let samples: Vec<_> = if segments == 0 {
        vec![(band.radius - size, 0.), (band.radius, size)]
    } else {
        (0..=segments)
            .map(|i| {
                let t = std::f64::consts::FRAC_PI_2 * i as f64 / segments as f64;
                if i == 0 {
                    (band.radius - size, 0.)
                } else if i == segments {
                    (band.radius, size)
                } else {
                    (band.radius - size + size * t.sin(), size * (1. - t.cos()))
                }
            })
            .collect()
    };
    let reference = error.map(|_| TreatmentSurface::Torus {
        axis_point: band.center - band.normal * size,
        axis: band.normal,
        major_radius: band.radius - size,
        minor_radius: size,
    });
    let mut planes = Vec::new();
    for (i, &point) in band.points.iter().enumerate() {
        let wall = band.walls[i];
        let factor = (point - band.center).dot(wall) / band.radius;
        for pair in samples.windows(2) {
            let [(a, z), (b, w)] = [pair[0], pair[1]];
            let s = (band.radius - a) * factor;
            let t = (band.radius - b) * factor;
            planes.push(Constraint {
                plane: Plane::from_point_normal(
                    point - wall * s - band.normal * z,
                    wall * (w - z) + band.normal * (s - t),
                )
                .map_err(|_| EdgeTreatmentError::UnresolvableCut)?,
                surface: reference,
            });
        }
    }
    let (object, surfaces) = intersect_boundary(source, &planes)?;
    let expected: f64 = samples
        .windows(2)
        .map(|p| {
            let [(a, z), (b, w)] = [p[0], p[1]];
            band.area * (w - z) * (1. - (a * a + a * b + b * b) / (3. * band.radius * band.radius))
        })
        .sum();
    let before = signed_volume(source, band.center);
    let removed = before - signed_volume(&object, band.center);
    let budget = tol::EDGE_TREATMENT_VOLUME_REL * expected
        + tol::EDGE_TREATMENT_VOLUME_ULPS * f64::EPSILON * before;
    if !expected.is_finite()
        || expected <= budget
        || !removed.is_finite()
        || (removed - expected).abs() > budget
    {
        return Err(EdgeTreatmentError::VolumeMismatch);
    }
    Ok(MultiEdgeTreatmentResult {
        result: EdgeTreatmentResult {
            object,
            expected_removed_volume: expected,
            removed_volume: removed,
        },
        segments,
        surfaces,
    })
}
