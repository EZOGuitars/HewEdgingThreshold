//! Convex planar cap bands. The same evaluator handles fresh polygonal prisms
//! and untouched opposite cap bands on already treated convex carriers.
//! Junctions are G0 miters, not analytic blends. No persistent feature history.
use super::multi::{Constraint, intersect_boundary};
use super::*;
use std::collections::BTreeSet;

pub const MAX_PERIMETER_EDGES: usize = 64;

/// Cheap routing only; all eligibility/safety checks remain in resolve.
pub(super) fn is_complete_cap(source: &Object, edges: &[EdgeId]) -> bool {
    let selected: BTreeSet<_> = edges.iter().copied().collect();
    edges.len() >= 3
        && selected.len() == edges.len()
        && source.faces().keys().any(|id| {
            face_perimeter_edges(source, id)
                .is_ok_and(|e| e.into_iter().collect::<BTreeSet<_>>() == selected)
        })
}

struct CapBand {
    normal: Vec3,
    points: Vec<Point3>,
    walls: Vec<Vec3>,
    perimeter: f64,
    corner_sum: f64,
    limit: f64,
}

impl CapBand {
    fn resolve(source: &Object, edges: &[EdgeId]) -> Result<Self, EdgeTreatmentError> {
        if edges.len() < 3 || edges.len() > MAX_PERIMETER_EDGES {
            return Err(EdgeTreatmentError::InvalidSelection);
        }
        let selected: BTreeSet<_> = edges.iter().copied().collect();
        if selected.len() != edges.len() {
            return Err(EdgeTreatmentError::InvalidSelection);
        }
        if selected.iter().any(|e| !source.edges().contains_key(*e)) {
            return Err(EdgeTreatmentError::UnknownEdge);
        }
        if selected.iter().any(|e| {
            let edge = &source.edges()[*e];
            edge.curve.is_some()
                || [Some(edge.half_edge), edge.twin_half_edge]
                    .into_iter()
                    .flatten()
                    .any(|h| {
                        source.faces()[source.loops()[source.half_edges()[h].loop_id].face]
                            .surface
                            .is_some()
                    })
        }) {
            return Err(EdgeTreatmentError::UnsupportedCurvedBoundary);
        }
        validate_convex_solid(source)?;
        let mut caps = source.faces().iter().filter(|(id, _)| {
            face_perimeter_edges(source, *id)
                .is_ok_and(|e| e.into_iter().collect::<BTreeSet<_>>() == selected)
        });
        let (cap_id, cap) = caps.next().ok_or(EdgeTreatmentError::UnsupportedSolid)?;
        if caps.next().is_some() {
            return Err(EdgeTreatmentError::UnsupportedSolid);
        }
        let normal = cap.plane.normal();
        let ordered = face_perimeter_edges(source, cap_id)?;
        let points: Vec<_> = source.loop_positions(cap.outer_loop).collect();
        let mut walls = Vec::new();
        let mut wall_ids = BTreeSet::new();
        let mut lengths = Vec::new();
        for (i, edge) in ordered.into_iter().enumerate() {
            let record = &source.edges()[edge];
            let twin = record
                .twin_half_edge
                .ok_or(EdgeTreatmentError::InvalidSource)?;
            let adjacent = [record.half_edge, twin]
                .map(|h| source.loops()[source.half_edges()[h].loop_id].face);
            let other = if adjacent[0] == cap_id {
                adjacent[1]
            } else {
                adjacent[0]
            };
            let wall = source.faces()[other].plane.normal();
            if normal.dot(wall).abs() > tol::NORMAL_DIRECTION {
                return Err(EdgeTreatmentError::NonPerpendicularFaces);
            }
            walls.push(wall);
            wall_ids.insert(other);
            lengths.push((points[(i + 1) % points.len()] - points[i]).length());
        }
        let mut cot = Vec::new();
        for i in 0..walls.len() {
            let a = walls[(i + walls.len() - 1) % walls.len()];
            let b = walls[i];
            let cosine = a.dot(b).clamp(-1.0, 1.0);
            let sine = a.cross(b).dot(normal);
            if sine <= tol::NORMAL_DIRECTION || 1.0 + cosine <= tol::NORMAL_DIRECTION {
                return Err(EdgeTreatmentError::UnsupportedSolid);
            }
            cot.push(sine / (1.0 + cosine));
        }
        let mut limit = f64::INFINITY;
        for i in 0..lengths.len() {
            limit = limit.min(lengths[i] / (cot[i] + cot[(i + 1) % cot.len()]));
        }
        // A complete unmodified prism band must exist down to the requested
        // depth. Each cap vertex translated inward must stay strictly below
        // every unrelated support plane. This excludes overlaps with old cuts.
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
        if !limit.is_finite() || limit <= 4.0 * tol::POINT_MERGE {
            return Err(EdgeTreatmentError::InsufficientClearance);
        }
        Ok(Self {
            normal,
            points,
            walls,
            perimeter: lengths.iter().sum(),
            corner_sum: cot.iter().sum(),
            limit,
        })
    }
}

/// Geometric supremum for a non-collapsing complete cap band, before numerical
/// and export certification. Callers must not advertise this as a certified max.
pub(super) fn geometric_limit(
    source: &Object,
    edges: &[EdgeId],
) -> Result<f64, EdgeTreatmentError> {
    Ok(CapBand::resolve(source, edges)?.limit)
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
    let band = CapBand::resolve(source, edges)?;
    if size >= band.limit - 4.0 * tol::POINT_MERGE {
        return Err(EdgeTreatmentError::InsufficientClearance);
    }
    let segments = if let Some(error) = error {
        if !error.is_finite() || error <= tol::PLANE_DIST {
            return Err(EdgeTreatmentError::InvalidChordError);
        }
        (MIN_ROUNDOVER_SEGMENTS..=MAX_ROUNDOVER_SEGMENTS)
            .find(|&n| {
                n * edges.len() <= MAX_BATCH_PLANES
                    && 2.0
                        * size
                        * (std::f64::consts::FRAC_PI_2 / (4.0 * n as f64))
                            .sin()
                            .powi(2)
                        <= error - tol::PLANE_DIST
            })
            .ok_or(EdgeTreatmentError::ApproximationTooFine)?
    } else {
        0
    };
    let samples: Vec<_> = if segments == 0 {
        vec![(size, 0.0), (0.0, size)]
    } else {
        (0..=segments)
            .map(|i| {
                let t = std::f64::consts::FRAC_PI_2 * i as f64 / segments as f64;
                if i == 0 {
                    (size, 0.0)
                } else if i == segments {
                    (0.0, size)
                } else {
                    (size * (1.0 - t.sin()), size * (1.0 - t.cos()))
                }
            })
            .collect()
    };
    let mut planes = Vec::new();
    for (i, &p) in band.points.iter().enumerate() {
        let wall = band.walls[i];
        let direction = band.normal.cross(wall);
        let center = p - (wall + band.normal) * size;
        for pair in samples.windows(2) {
            let [(s, z), (s1, z1)] = [pair[0], pair[1]];
            planes.push(Constraint {
                plane: Plane::from_point_normal(
                    p - wall * s - band.normal * z,
                    wall * (z1 - z) + band.normal * (s - s1),
                )
                .map_err(|_| EdgeTreatmentError::UnresolvableCut)?,
                surface: error.map(|_| TreatmentSurface::Cylinder {
                    axis_point: center,
                    axis: direction,
                    radius: size,
                }),
            });
        }
    }
    let (object, surfaces) = intersect_boundary(source, &planes)?;
    // Independent integration of A(0)-A(s) = P*s-K*s² over the faceted
    // profile. This is not a second volume measurement of the built mesh.
    let expected: f64 = samples
        .windows(2)
        .map(|p| {
            let [(a, z), (b, w)] = [p[0], p[1]];
            (w - z)
                * (band.perimeter * (a + b) / 2.0 - band.corner_sum * (a * a + a * b + b * b) / 3.0)
        })
        .sum();
    let before = signed_volume(source, band.points[0]);
    let removed = before - signed_volume(&object, band.points[0]);
    let budget = tol::EDGE_TREATMENT_VOLUME_REL * expected
        + tol::EDGE_TREATMENT_VOLUME_ULPS * f64::EPSILON * before;
    if !expected.is_finite()
        || expected <= budget
        || !removed.is_finite()
        || removed <= 0.0
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
