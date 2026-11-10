//! Simultaneous convex box treatment. All planes are planned from the original
//! topology, then intersected in one shared indexed boundary. No edge is
//! resolved on intermediate geometry and no independently treated meshes join.
use super::*;
use slotmap::Key;
use std::collections::{BTreeMap, BTreeSet};

/// Explicit bound on construction/validation work, never a silent quality clamp.
pub const MAX_BATCH_PLANES: usize = 4096;
#[derive(Debug, Clone, Copy)]
pub enum TreatmentSurface {
    Cylinder {
        axis_point: Point3,
        axis: Vec3,
        radius: f64,
    },
    Sphere {
        center: Point3,
        radius: f64,
    },
}
#[derive(Debug)]
pub struct MultiEdgeTreatmentResult {
    pub result: EdgeTreatmentResult,
    pub segments: usize,
    /// Ephemeral validation references, not persistent analytic face claims.
    pub surfaces: Vec<(FaceId, TreatmentSurface)>,
}
#[derive(Clone)]
struct BoundaryFace {
    vertices: Vec<usize>,
    material: crate::FaceMaterial,
    back: crate::FaceMaterial,
    uv: Option<crate::UvFrame>,
    surface: Option<TreatmentSurface>,
}
pub(super) struct Constraint {
    pub(super) plane: Plane,
    pub(super) surface: Option<TreatmentSurface>,
}
struct BoxFrame {
    origin: Point3,
    axes: [Vec3; 3],
    lengths: [f64; 3],
    corners: BTreeMap<crate::VertexId, usize>,
}
impl BoxFrame {
    fn resolve(source: &Object) -> Result<Self, EdgeTreatmentError> {
        if source.vertices().len() != 8
            || source.edges().len() != 12
            || source.faces().len() != 6
            || source
                .faces()
                .values()
                .any(|f| source.loop_positions(f.outer_loop).count() != 4)
        {
            return Err(EdgeTreatmentError::UnsupportedBox);
        }
        validate_convex_solid(source)?;
        let (vertex, v) = source
            .vertices()
            .iter()
            .next()
            .ok_or(EdgeTreatmentError::InvalidSource)?;
        let origin = v.position;
        let rays: Vec<_> = source
            .edges()
            .keys()
            .filter_map(|e| {
                let record = &source.edges()[e];
                let h = &source.half_edges()[record.half_edge];
                let a = h.origin;
                let b = source.half_edges()[h.next].origin;
                if a == vertex {
                    Some(source.vertices()[b].position - origin)
                } else if b == vertex {
                    Some(source.vertices()[a].position - origin)
                } else {
                    None
                }
            })
            .collect();
        if rays.len() != 3 {
            return Err(EdgeTreatmentError::UnsupportedBox);
        }
        let lengths = std::array::from_fn(|i| rays[i].length());
        let axes = [
            rays[0].normalized(),
            rays[1].normalized(),
            rays[2].normalized(),
        ];
        let [Ok(a), Ok(b), Ok(c)] = axes else {
            return Err(EdgeTreatmentError::UnsupportedBox);
        };
        let axes = [a, b, c];
        if (0..3).any(|i| (i + 1..3).any(|j| axes[i].dot(axes[j]).abs() > tol::NORMAL_DIRECTION)) {
            return Err(EdgeTreatmentError::UnsupportedBox);
        }
        let mut corners = BTreeMap::new();
        let mut masks = BTreeSet::new();
        for (id, v) in source.vertices() {
            let mut mask = 0;
            for i in 0..3 {
                let d = (v.position - origin).dot(axes[i]);
                if d.abs() <= tol::PLANE_DIST {
                } else if (d - lengths[i]).abs() <= tol::PLANE_DIST {
                    mask |= 1 << i;
                } else {
                    return Err(EdgeTreatmentError::UnsupportedBox);
                }
            }
            if !masks.insert(mask) {
                return Err(EdgeTreatmentError::UnsupportedBox);
            }
            corners.insert(id, mask);
        }
        Ok(Self {
            origin,
            axes,
            lengths,
            corners,
        })
    }
    fn point(&self, mask: usize) -> Point3 {
        (0..3).fold(self.origin, |p, i| {
            p + self.axes[i]
                * if mask & (1 << i) != 0 {
                    self.lengths[i]
                } else {
                    0.0
                }
        })
    }
    fn outward(&self, mask: usize) -> [Vec3; 3] {
        std::array::from_fn(|i| self.axes[i] * if mask & (1 << i) != 0 { 1.0 } else { -1.0 })
    }
}
pub(super) fn box_dimension_bound(source: &Object) -> Result<f64, EdgeTreatmentError> {
    Ok(BoxFrame::resolve(source)?
        .lengths
        .into_iter()
        .fold(f64::INFINITY, f64::min)
        * 0.25)
}

/// Topological perimeter IDs, never reconstructed from rounded render positions.
pub fn face_perimeter_edges(
    source: &Object,
    face: FaceId,
) -> Result<Vec<EdgeId>, EdgeTreatmentError> {
    let face = source
        .faces()
        .get(face)
        .ok_or(EdgeTreatmentError::UnknownFace)?;
    if !face.inner_loops.is_empty() {
        return Err(EdgeTreatmentError::UnsupportedSolid);
    }
    let first = source.loops()[face.outer_loop].first_half_edge;
    let mut h = first;
    let mut edges = Vec::new();
    loop {
        edges.push(source.half_edges()[h].edge);
        h = source.half_edges()[h].next;
        if h == first {
            break;
        }
    }
    Ok(edges)
}
pub fn chamfer_edges(
    source: &Object,
    edges: &[EdgeId],
    distance: f64,
) -> Result<MultiEdgeTreatmentResult, EdgeTreatmentError> {
    if edges.len() == 1 {
        return Ok(MultiEdgeTreatmentResult {
            result: chamfer(source, edges[0], distance)?,
            segments: 0,
            surfaces: vec![],
        });
    }
    if super::perimeter::is_complete_cap(source, edges) {
        return super::perimeter::evaluate(source, edges, distance, None);
    }
    batch(source, edges, distance, None)
}
pub fn roundover_edges(
    source: &Object,
    edges: &[EdgeId],
    radius: f64,
    error: f64,
) -> Result<MultiEdgeTreatmentResult, EdgeTreatmentError> {
    if edges.len() == 1 {
        let r = roundover(source, edges[0], radius, error)?;
        return Ok(MultiEdgeTreatmentResult {
            result: r.result,
            segments: r.segments,
            surfaces: vec![],
        });
    }
    if super::perimeter::is_complete_cap(source, edges) {
        return super::perimeter::evaluate(source, edges, radius, Some(error));
    }
    batch(source, edges, radius, Some(error))
}
fn arc(i: usize, n: usize) -> (f64, f64) {
    if i == 0 {
        (1.0, 0.0)
    } else if i == n {
        (0.0, 1.0)
    } else {
        let t = std::f64::consts::FRAC_PI_2 * i as f64 / n as f64;
        (t.cos(), t.sin())
    }
}
// Unit octant quads are planar (parallel latitude chords); the last row has a
// single pole. One constraint per cell avoids nearly coplanar duplicate cuts.
fn octant(n: usize) -> Vec<(Vec3, Vec3)> {
    let mut cells = Vec::with_capacity(n * n);
    for j in 0..n {
        let (ca, sa) = arc(j, n);
        let (cb, sb) = arc(j + 1, n);
        for i in 0..n {
            let (c, s) = arc(i, n);
            let (d, t) = arc(i + 1, n);
            let a = Vec3::new(ca * c, ca * s, sa);
            let b = Vec3::new(ca * d, ca * t, sa);
            let q = Vec3::new(cb * d, cb * t, sb);
            let normal = (b - a)
                .cross(q - a)
                .normalized()
                .expect("nondegenerate unit octant cell");
            cells.push((a, normal));
        }
    }
    cells
}
fn batch(
    source: &Object,
    edges: &[EdgeId],
    radius: f64,
    error: Option<f64>,
) -> Result<MultiEdgeTreatmentResult, EdgeTreatmentError> {
    if !radius.is_finite() || radius <= tol::POINT_MERGE {
        return Err(if error.is_some() {
            EdgeTreatmentError::InvalidRadius
        } else {
            EdgeTreatmentError::InvalidDistance
        });
    }
    let mut selected = edges.to_vec();
    selected.sort_by_key(|e| e.data().as_ffi());
    let old = selected.len();
    selected.dedup();
    if old < 2 || old != selected.len() || old > 12 {
        return Err(EdgeTreatmentError::InvalidSelection);
    }
    for &e in &selected {
        if !source.edges().contains_key(e) {
            return Err(EdgeTreatmentError::UnknownEdge);
        }
    }
    let frame = BoxFrame::resolve(source)?;
    if frame
        .lengths
        .iter()
        .any(|&l| 2.0 * radius >= l - tol::PLANE_DIST)
    {
        return Err(EdgeTreatmentError::InsufficientClearance);
    }
    let mut degree = [0usize; 8];
    let mut lengths = Vec::new();
    for &e in &selected {
        let h = &source.half_edges()[source.edges()[e].half_edge];
        let a = frame.corners[&h.origin];
        let b = frame.corners[&source.half_edges()[h.next].origin];
        if !(a ^ b).is_power_of_two() {
            return Err(EdgeTreatmentError::UnsupportedBox);
        }
        degree[a] += 1;
        degree[b] += 1;
        let (p, q) = source
            .edge_endpoints(e)
            .ok_or(EdgeTreatmentError::UnknownEdge)?;
        lengths.push((q - p).length());
    }
    let spheres = degree.iter().filter(|&&d| d == 3).count();
    let n = if let Some(error) = error {
        if !error.is_finite() || error <= tol::PLANE_DIST {
            return Err(EdgeTreatmentError::InvalidChordError);
        }
        let mut found = None;
        for n in MIN_ROUNDOVER_SEGMENTS..=MAX_ROUNDOVER_SEGMENTS {
            if selected.len() * n + spheres * n * n > MAX_BATCH_PLANES {
                break;
            }
            let chord = 2.0
                * radius
                * (std::f64::consts::FRAC_PI_2 / (4.0 * n as f64))
                    .sin()
                    .powi(2);
            if chord > error - tol::PLANE_DIST {
                continue;
            }
            if spheres > 0
                && octant(n)
                    .iter()
                    .any(|(p, normal)| radius * (1.0 - normal.dot(*p)) > error - tol::PLANE_DIST)
            {
                continue;
            }
            found = Some(n);
            break;
        }
        found.ok_or(EdgeTreatmentError::ApproximationTooFine)?
    } else {
        0
    };
    let mut planes = Vec::new();
    for &edge in &selected {
        let f = resolve_edge_frame(source, edge)?;
        if error.is_none() {
            planes.push(Constraint {
                plane: Plane::from_point_normal(
                    f.start + f.inward[0] * radius,
                    -(f.inward[0] + f.inward[1]),
                )
                .map_err(|_| EdgeTreatmentError::InvalidResult)?,
                surface: None,
            });
            continue;
        }
        let center = f.start + (f.inward[0] + f.inward[1]) * radius;
        for i in 0..n {
            let (c, s) = arc(i, n);
            let (d, t) = arc(i + 1, n);
            let p =
                f.start + f.inward[0] * (radius * (1.0 - s)) + f.inward[1] * (radius * (1.0 - c));
            let q =
                f.start + f.inward[0] * (radius * (1.0 - t)) + f.inward[1] * (radius * (1.0 - d));
            let mut normal = f.direction.cross(q - p);
            if normal.dot(p - center) < 0.0 {
                normal = -normal;
            }
            planes.push(Constraint {
                plane: Plane::from_point_normal(p, normal)
                    .map_err(|_| EdgeTreatmentError::UnresolvableCut)?,
                surface: Some(TreatmentSurface::Cylinder {
                    axis_point: center,
                    axis: f.direction,
                    radius,
                }),
            });
        }
    }
    if spheres > 0 && error.is_some() {
        let cells = octant(n);
        for (mask, &d) in degree.iter().enumerate() {
            if d != 3 {
                continue;
            }
            let normals = frame.outward(mask);
            let center = frame.point(mask) - (normals[0] + normals[1] + normals[2]) * radius;
            for &(p, normal) in &cells {
                let map = |v: Vec3| normals[0] * v.x + normals[1] * v.y + normals[2] * v.z;
                planes.push(Constraint {
                    plane: Plane::from_point_normal(center + map(p) * radius, map(normal))
                        .map_err(|_| EdgeTreatmentError::UnresolvableCut)?,
                    surface: Some(TreatmentSurface::Sphere { center, radius }),
                });
            }
        }
    }
    let (object, surfaces) = intersect_boundary(source, &planes)?;
    // Independent cell integration counts shared corner cuts only once.
    let area = if error.is_some() {
        radius * radius * (1.0 - n as f64 * (std::f64::consts::FRAC_PI_2 / n as f64).sin() / 2.0)
    } else {
        radius * radius / 2.0
    };
    let mut expected = lengths
        .iter()
        .map(|&l| area * (l - 2.0 * radius))
        .sum::<f64>();
    let corner_two = if error.is_some() {
        let mut kept = 0.0;
        for i in 0..n {
            let (c, s) = arc(i, n);
            let (d, t) = arc(i + 1, n);
            kept += (c - d) * (s * s + s * t + t * t) / 3.0;
        }
        radius.powi(3) * (1.0 - kept)
    } else {
        radius.powi(3) * 2.0 / 3.0
    };
    let corner_three = if error.is_some() {
        let h = std::f64::consts::FRAC_PI_2 / n as f64;
        radius.powi(3) * (1.0 - n as f64 * h.sin().powi(2) / (6.0 * (h / 2.0).tan()))
    } else {
        radius.powi(3) * 0.75
    };
    for d in degree {
        expected += match d {
            0 => 0.0,
            1 => area * radius,
            2 => corner_two,
            3 => corner_three,
            _ => return Err(EdgeTreatmentError::InvalidSelection),
        };
    }
    let before = signed_volume(source, frame.origin);
    let removed = before - signed_volume(&object, frame.origin);
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
        segments: n,
        surfaces,
    })
}

pub(super) fn intersect_boundary(
    source: &Object,
    planes: &[Constraint],
) -> Result<(Object, Vec<(FaceId, TreatmentSurface)>), EdgeTreatmentError> {
    let mut positions: Vec<_> = source.vertices().values().map(|v| v.position).collect();
    let indices: BTreeMap<_, _> = source
        .vertices()
        .keys()
        .enumerate()
        .map(|(i, v)| (v, i))
        .collect();
    let mut faces: Vec<_> = source
        .faces()
        .values()
        .map(|f| {
            let first = source.loops()[f.outer_loop].first_half_edge;
            let mut h = first;
            let mut vertices = Vec::new();
            loop {
                vertices.push(indices[&source.half_edges()[h].origin]);
                h = source.half_edges()[h].next;
                if h == first {
                    break;
                }
            }
            BoundaryFace {
                vertices,
                material: f.material,
                back: f.back_material,
                uv: f.uv_frame,
                surface: None,
            }
        })
        .collect();
    for constraint in planes {
        let plane = constraint.plane;
        let distances: Vec<_> = positions
            .iter()
            .map(|&p| plane.signed_distance(p))
            .collect();
        if !faces
            .iter()
            .flat_map(|f| &f.vertices)
            .any(|&i| distances[i] > tol::PLANE_DIST)
        {
            continue;
        }
        let mut cache = BTreeMap::new();
        let mut cap = BTreeSet::new();
        let mut next = Vec::new();
        for face in &faces {
            let mut ring = Vec::new();
            for i in 0..face.vertices.len() {
                let a = face.vertices[i];
                let b = face.vertices[(i + 1) % face.vertices.len()];
                let da = distances[a];
                let db = distances[b];
                let ina = da <= tol::PLANE_DIST;
                let inb = db <= tol::PLANE_DIST;
                if ina {
                    ring.push(a);
                    if da.abs() <= tol::PLANE_DIST {
                        cap.insert(a);
                    }
                }
                if ina != inb {
                    let at = if da.abs() <= tol::PLANE_DIST {
                        a
                    } else if db.abs() <= tol::PLANE_DIST {
                        b
                    } else {
                        *cache.entry((a.min(b), a.max(b))).or_insert_with(|| {
                            let t = da / (da - db);
                            let p = positions[a] + (positions[b] - positions[a]) * t;
                            let id = positions.len();
                            positions.push(p);
                            id
                        })
                    };
                    ring.push(at);
                    cap.insert(at);
                }
            }
            ring.dedup();
            if ring.len() > 1 && ring.first() == ring.last() {
                ring.pop();
            }
            if ring.len() >= 3 {
                let mut f = face.clone();
                f.vertices = ring;
                next.push(f);
            }
        }
        if cap.len() < 3 {
            return Err(EdgeTreatmentError::UnresolvableCut);
        }
        let center = cap
            .iter()
            .fold(Vec3::ZERO, |v, &i| v + positions[i].to_vec())
            / cap.len() as f64;
        let normal = plane.normal();
        let aux = if normal.x.abs() < 0.9 {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };
        let u = aux
            .cross(normal)
            .normalized()
            .map_err(|_| EdgeTreatmentError::UnresolvableCut)?;
        let v = normal.cross(u);
        let mut ring: Vec<_> = cap.into_iter().collect();
        ring.sort_by(|&a, &b| {
            let a = positions[a].to_vec() - center;
            let b = positions[b].to_vec() - center;
            a.dot(v)
                .atan2(a.dot(u))
                .total_cmp(&b.dot(v).atan2(b.dot(u)))
        });
        next.push(BoundaryFace {
            vertices: ring,
            material: None,
            back: None,
            uv: None,
            surface: constraint.surface,
        });
        faces = next;
    }
    let mut used = BTreeMap::new();
    let mut compact = Vec::new();
    let polygons: Vec<Vec<_>> = faces
        .iter()
        .map(|f| {
            f.vertices
                .iter()
                .map(|&i| {
                    *used.entry(i).or_insert_with(|| {
                        let n = compact.len();
                        compact.push(positions[i]);
                        n
                    })
                })
                .collect()
        })
        .collect();
    let materials: Vec<_> = faces.iter().map(|f| f.material).collect();
    let uvs: Vec<_> = faces.iter().map(|f| f.uv).collect();
    let surfaces = vec![None; faces.len()];
    let mut object = Object::from_polygons_with_materials_and_frames(
        &compact, &polygons, &materials, &uvs, &surfaces,
    )
    .map_err(|_| EdgeTreatmentError::InvalidResult)?;
    object.set_built_back_materials(&faces.iter().map(|f| f.back).collect::<Vec<_>>());
    object.default_material = source.default_material;
    object
        .validate()
        .map_err(|_| EdgeTreatmentError::InvalidResult)?;
    if object.watertight() != WatertightState::Watertight || object.face_components().len() != 1 {
        return Err(EdgeTreatmentError::InvalidResult);
    }
    // Convex supporting planes plus the shared indexed clipping construction
    // certify a non-self-intersecting boundary; no independent mesh welding.
    for vertex in object.vertices().values() {
        if source
            .faces()
            .values()
            .any(|f| f.plane.signed_distance(vertex.position) > tol::PLANE_DIST)
            || planes
                .iter()
                .any(|p| p.plane.signed_distance(vertex.position) > tol::PLANE_DIST)
            || object
                .faces()
                .values()
                .any(|f| f.plane.signed_distance(vertex.position) > tol::PLANE_DIST)
        {
            return Err(EdgeTreatmentError::ContainmentFailed);
        }
    }
    let refs = object
        .faces()
        .keys()
        .zip(faces)
        .filter_map(|(id, f)| f.surface.map(|s| (id, s)))
        .collect();
    Ok((object, refs))
}
