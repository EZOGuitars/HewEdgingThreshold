//! One bounded, Scene-owned preview slot. No persistent preview entities.
use super::*;
use kernel::{
    DocumentEdgeTreatmentError, EdgeTreatmentCandidate, EdgeTreatmentOperation, EntityRef,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU32, Ordering};

// Process/module-wide uniqueness prevents handles minted by another Scene
// (including a dropped Scene) from naming this Scene's current candidate.
static NEXT_HANDLE: AtomicU32 = AtomicU32::new(1);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum TreatmentParameters {
    Chamfer { distance: f64 },
    Roundover { radius: f64, max_chord_error: f64 },
}
impl From<TreatmentParameters> for EdgeTreatmentOperation {
    fn from(p: TreatmentParameters) -> Self {
        match p {
            TreatmentParameters::Chamfer { distance } => Self::Chamfer { distance },
            TreatmentParameters::Roundover {
                radius,
                max_chord_error,
            } => Self::Roundover {
                radius,
                max_chord_error,
            },
        }
    }
}

/// Stable target plus an exact, unique geometric edge address, never an
/// ordinal/slot-map key. Replay must match the recorded source-state hash
/// AND both endpoints exactly; no nearest-edge or tolerance fallback.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct TreatmentCommand {
    target_sid: u64,
    source_hash: u64,
    endpoints: [[f64; 3]; 2],
    parameters: TreatmentParameters,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct TreatmentBatchCommand {
    target_sid: u64,
    source_hash: u64,
    endpoints: Vec<[[f64; 3]; 2]>,
    parameters: TreatmentParameters,
}
/// Preview transport is separate from persistent replay commands. The origin's
/// epoch/generation travel opaquely through the isolated evaluator and are
/// checked again by the main tool before publication and final application.
#[derive(Serialize, Deserialize)]
struct PreviewRequest {
    version: u32,
    epoch: u64,
    generation: u64,
    command: TreatmentBatchCommand,
}
fn preview_command(request: &str) -> Result<TreatmentBatchCommand, ApiError> {
    let request: PreviewRequest = serde_json::from_str(request)
        .map_err(|_| ApiError::new("InvalidPreviewRequest", "invalid preview request"))?;
    if request.version != 1 {
        return Err(ApiError::new(
            "InvalidPreviewRequest",
            "unsupported preview version",
        ));
    }
    Ok(request.command)
}

#[derive(PartialEq, Eq)]
struct Context {
    epoch: u64,
    hidden: BTreeSet<ObjectId>,
    hidden_instances: BTreeSet<InstanceId>,
    scope: Option<PickScope>,
    frame_sketches: bool,
    instance: Option<InstanceId>,
}
pub(crate) struct PreparedTreatment {
    handle: u32,
    candidate: EdgeTreatmentCandidate,
    context: Context,
    command: TreatmentCommand,
    batch_endpoints: Option<Vec<[[f64; 3]; 2]>>,
}

fn boundary_error(e: DocumentEdgeTreatmentError) -> ApiError {
    match e {
        DocumentEdgeTreatmentError::Document(e) => doc_err(e),
        DocumentEdgeTreatmentError::Geometry(e) => api_err(&e, &e),
        DocumentEdgeTreatmentError::UnsupportedContext => {
            ApiError::new("ActiveEditingSession", &e.to_string())
        }
        _ => api_err(&e, &e),
    }
}
fn edge_id(handle: u64) -> EdgeId {
    EdgeId::from(KeyData::from_ffi(handle))
}
fn coordinates(p: Point3) -> [f64; 3] {
    [p.x, p.y, p.z]
}

impl Scene {
    fn treatment_context(&self) -> Context {
        Context {
            epoch: self.doc_epoch,
            hidden: self.hidden_objects.iter().copied().collect(),
            hidden_instances: self.hidden_instances.iter().copied().collect(),
            scope: self.inference.pick_scope().cloned(),
            frame_sketches: self.pick_scope_frame_sketches,
            instance: self.active_inference_instance,
        }
    }
    fn treatment_target(&self, id: ObjectId) -> Result<(), ApiError> {
        self.doc
            .object(id)
            .ok_or_else(|| stale("UnknownObject", "object"))?;
        if self.doc.object_owner_component(id).is_some() {
            return Err(ApiError::new(
                "UnsupportedDefinitionMember",
                "edge treatments require a world object",
            ));
        }
        let mut node = Some(NodeId::Object(id));
        while let Some(n) = node {
            if self.doc.node_user_hidden(n)
                || self
                    .doc
                    .node_tags(n)
                    .iter()
                    .any(|t| (1..=t.len()).any(|k| self.doc.tag_hidden(&t[..k])))
            {
                return Err(ApiError::new(
                    "HiddenTarget",
                    "target or ancestor is hidden",
                ));
            }
            node = self.doc.node_parent(n).map(NodeId::Group);
        }
        if self.hidden_objects.contains(&id) {
            return Err(ApiError::new(
                "HiddenTarget",
                "target is hidden in this Scene",
            ));
        }
        if self.open_draft.is_some()
            || self.active_inference_instance.is_some()
            || !self.doc.session_stack().is_empty()
        {
            return Err(ApiError::new(
                "ActiveEditingSession",
                "finish editing or drawing before preparing a treatment",
            ));
        }
        if self
            .inference
            .pick_scope()
            .is_some_and(|s| !s.objects.contains(&id))
        {
            return Err(ApiError::new(
                "UnsupportedSelectionContext",
                "target is outside the current selection scope",
            ));
        }
        Ok(())
    }
    fn certify_treatment(
        &self,
        id: ObjectId,
        candidate: &EdgeTreatmentCandidate,
        parameters: TreatmentParameters,
    ) -> Result<(), ApiError> {
        if let TreatmentParameters::Roundover {
            radius,
            max_chord_error,
        } = parameters
        {
            if candidate.edges().len() == 1 {
                mesh_export::certify_roundover_stl(
                    self.doc
                        .object(id)
                        .ok_or_else(|| stale("UnknownObject", "object"))?,
                    candidate.edge(),
                    &candidate.result().object,
                    radius,
                    max_chord_error,
                )
                .map_err(|e| ApiError::new("StlChordPrecision", &e.message()))?;
            } else {
                mesh_export::certify_treatment_surfaces(
                    &candidate.result().object,
                    candidate.surfaces(),
                    max_chord_error,
                )
                .map_err(|e| ApiError::new("StlChordPrecision", &e.message()))?;
            }
        }
        Ok(())
    }

    fn prepare_treatment(
        &mut self,
        object: u64,
        edge: u64,
        parameters: TreatmentParameters,
    ) -> Result<u32, ApiError> {
        self.prepare_treatments(object, &[edge], parameters)
    }
    fn prepare_treatments(
        &mut self,
        object: u64,
        edges: &[u64],
        parameters: TreatmentParameters,
    ) -> Result<u32, ApiError> {
        let id = object_id(object);
        self.treatment_target(id)?;
        let native_edges: Vec<_> = edges.iter().copied().map(edge_id).collect();
        let candidate = self
            .doc
            .prepare_edge_treatments(id, &native_edges, parameters.into())
            .map_err(boundary_error)?;
        self.certify_treatment(id, &candidate, parameters)?;
        let endpoints: Vec<_> = candidate
            .edges()
            .iter()
            .map(|&edge| {
                self.doc
                    .object(id)
                    .and_then(|o| o.edge_endpoints(edge))
                    .map(|(a, b)| [coordinates(a), coordinates(b)])
                    .ok_or_else(|| stale("UnknownEdge", "edge"))
            })
            .collect::<Result<_, _>>()?;
        let [a, b] = endpoints[0];
        let target_sid = self
            .doc
            .sid_of(&EntityRef::Object(id))
            .ok_or_else(|| stale("UnknownObject", "object"))?;
        let handle = NEXT_HANDLE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| {
                ApiError::new(
                    "CandidateHandleExhausted",
                    "candidate handle space exhausted",
                )
            })?;
        let command = TreatmentCommand {
            target_sid,
            source_hash: self.doc.state_hash(),
            endpoints: [a, b],
            parameters,
        };
        self.edge_candidate = Some(PreparedTreatment {
            handle,
            candidate,
            context: self.treatment_context(),
            command,
            batch_endpoints: if endpoints.len() > 1 {
                Some(endpoints)
            } else {
                None
            },
        });
        Ok(handle)
    }
    fn treatment(&self, handle: u32) -> Result<&PreparedTreatment, ApiError> {
        self.edge_candidate
            .as_ref()
            .filter(|p| p.handle == handle)
            .ok_or_else(|| stale("InvalidCandidateHandle", "candidate"))
    }
    fn recorded_treatment_edges(
        &self,
        target_sid: u64,
        source_hash: u64,
        endpoints: &[[[f64; 3]; 2]],
    ) -> Result<(ObjectId, Vec<u64>), ApiError> {
        if self.doc.state_hash() != source_hash {
            return Err(ApiError::new(
                "ReplaySourceMismatch",
                "edge treatment source state differs",
            ));
        }
        let id = self
            .doc
            .sids()
            .find_map(|(entity, sid)| match entity {
                EntityRef::Object(id) if sid == target_sid => Some(*id),
                _ => None,
            })
            .ok_or_else(|| stale("UnknownObject", "recorded target SID"))?;
        let object = self
            .doc
            .object(id)
            .ok_or_else(|| stale("UnknownObject", "recorded target"))?;
        let mut edges = Vec::new();
        let mut seen = BTreeSet::new();
        for pair in endpoints {
            let mut matches = object.edges().keys().filter(|&e| {
                object.edge_endpoints(e).is_some_and(|(a, b)| {
                    let p = [coordinates(a), coordinates(b)];
                    p == *pair || [p[1], p[0]] == *pair
                })
            });
            let edge = matches.next().ok_or_else(|| {
                ApiError::new(
                    "ReplayEdgeMismatch",
                    "recorded edge endpoints do not resolve",
                )
            })?;
            if matches.next().is_some() || !seen.insert(edge) {
                return Err(ApiError::new(
                    "ReplayEdgeMismatch",
                    "recorded edges are ambiguous or duplicated",
                ));
            }
            edges.push(edge.data().as_ffi());
        }
        Ok((id, edges))
    }
    pub(crate) fn replay_treatment(&mut self, command: TreatmentCommand) -> Result<(), ApiError> {
        let (id, edges) = self.recorded_treatment_edges(
            command.target_sid,
            command.source_hash,
            &[command.endpoints],
        )?;
        let handle = self.prepare_treatment(id.data().as_ffi(), edges[0], command.parameters)?;
        self.commit_edge_treatment(handle)?;
        Ok(())
    }
    pub(crate) fn replay_treatment_batch(
        &mut self,
        command: TreatmentBatchCommand,
    ) -> Result<(), ApiError> {
        let (id, edges) = self.recorded_treatment_edges(
            command.target_sid,
            command.source_hash,
            &command.endpoints,
        )?;
        let handle = self.prepare_treatments(id.data().as_ffi(), &edges, command.parameters)?;
        self.commit_edge_treatment(handle)?;
        Ok(())
    }
}

#[wasm_bindgen]
impl Scene {
    /// Immutable geometric address for isolated preview evaluators. Scene-local
    /// handles never cross the worker boundary: SID/hash/exact endpoints do.
    pub fn edge_treatment_request_json(
        &self,
        object: u64,
        edges: &[u64],
        roundover: bool,
        size: f64,
        chord: f64,
    ) -> Result<String, ApiError> {
        let id = object_id(object);
        self.treatment_target(id)?;
        if edges.is_empty() {
            return Err(ApiError::new("InvalidSelection", "empty edge selection"));
        }
        let source = self
            .doc
            .object(id)
            .ok_or_else(|| stale("UnknownObject", "object"))?;
        let endpoints = edges
            .iter()
            .map(|e| {
                source
                    .edge_endpoints(edge_id(*e))
                    .map(|(a, b)| [coordinates(a), coordinates(b)])
                    .ok_or_else(|| stale("UnknownEdge", "edge"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let command = TreatmentBatchCommand {
            target_sid: self
                .doc
                .sid_of(&EntityRef::Object(id))
                .ok_or_else(|| stale("UnknownObject", "SID"))?,
            source_hash: self.doc.state_hash(),
            endpoints,
            parameters: if roundover {
                TreatmentParameters::Roundover {
                    radius: size,
                    max_chord_error: chord,
                }
            } else {
                TreatmentParameters::Chamfer { distance: size }
            },
        };
        serde_json::to_string(&PreviewRequest {
            version: 1,
            epoch: self.doc_epoch,
            generation: self.doc.history_generation(),
            command,
        })
        .map_err(|_| ApiError::new("InvalidPreviewRequest", "nonfinite preview parameters"))
    }
    pub fn prepare_edge_treatment_request_json(&mut self, request: &str) -> Result<u32, ApiError> {
        let command = preview_command(request)?;
        let (id, edges) = self.recorded_treatment_edges(
            command.target_sid,
            command.source_hash,
            &command.endpoints,
        )?;
        self.prepare_treatments(id.data().as_ffi(), &edges, command.parameters)
    }
    pub fn edge_treatment_request_limits_json(&self, request: &str) -> Result<String, ApiError> {
        let command = preview_command(request)?;
        let (id, edges) = self.recorded_treatment_edges(
            command.target_sid,
            command.source_hash,
            &command.endpoints,
        )?;
        let (round, chord) = match command.parameters {
            TreatmentParameters::Chamfer { .. } => (false, 0.),
            TreatmentParameters::Roundover {
                max_chord_error, ..
            } => (true, max_chord_error),
        };
        self.edge_treatment_limits_json(id.data().as_ffi(), &edges, round, chord)
    }

    /// Read-only geometric range probe. Does not mint/retire a handle, mutate
    /// history or recording, or replace the current prepared candidate.
    /// The endpoint is certified; smaller requests still undergo full validation.
    pub fn edge_treatment_limits_json(
        &self,
        object: u64,
        edges: &[u64],
        roundover: bool,
        chord: f64,
    ) -> Result<String, ApiError> {
        let id = object_id(object);
        self.treatment_target(id)?;
        let native: Vec<_> = edges.iter().copied().map(edge_id).collect();
        let source = self
            .doc
            .object(id)
            .ok_or_else(|| stale("UnknownObject", "object"))?;
        let maximum = kernel::edge_treatment::treatment_dimension_bound(source, &native)
            .map_err(|e| api_err(&e, &e))?;
        let parameters = if roundover {
            TreatmentParameters::Roundover {
                radius: maximum,
                max_chord_error: chord,
            }
        } else {
            TreatmentParameters::Chamfer { distance: maximum }
        };
        let candidate = self
            .doc
            .prepare_edge_treatments(id, &native, parameters.into())
            .map_err(boundary_error)?;
        self.certify_treatment(id, &candidate, parameters)?;
        Ok(serde_json::json!({ "maximum": maximum, "certainty": "conservative", "segments_at_maximum": candidate.segments() }).to_string())
    }

    pub fn prepare_chamfer_edges(
        &mut self,
        object: u64,
        edges: &[u64],
        distance: f64,
    ) -> Result<u32, ApiError> {
        self.prepare_treatments(object, edges, TreatmentParameters::Chamfer { distance })
    }
    pub fn prepare_roundover_edges(
        &mut self,
        object: u64,
        edges: &[u64],
        radius: f64,
        max_chord_error: f64,
    ) -> Result<u32, ApiError> {
        self.prepare_treatments(
            object,
            edges,
            TreatmentParameters::Roundover {
                radius,
                max_chord_error,
            },
        )
    }
    pub fn edge_treatment_face_edges(&self, object: u64, face: u64) -> Result<Vec<u64>, ApiError> {
        let id = object_id(object);
        self.treatment_target(id)?;
        kernel::edge_treatment::face_perimeter_edges(
            self.doc
                .object(id)
                .ok_or_else(|| stale("UnknownObject", "object"))?,
            FaceId::from(KeyData::from_ffi(face)),
        )
        .map(|edges| edges.iter().map(|e| e.data().as_ffi()).collect())
        .map_err(|e| api_err(&e, &e))
    }
    /// Prepares without editing the document. Success replaces/frees this
    /// Scene's previous candidate; failure retains it. Handles are opaque,
    /// process-unique u32 tokens, not geometry or document identifiers.
    pub fn prepare_chamfer(
        &mut self,
        object: u64,
        edge: u64,
        distance: f64,
    ) -> Result<u32, ApiError> {
        self.prepare_treatment(object, edge, TreatmentParameters::Chamfer { distance })
    }
    pub fn prepare_roundover(
        &mut self,
        object: u64,
        edge: u64,
        radius: f64,
        max_chord_error: f64,
    ) -> Result<u32, ApiError> {
        self.prepare_treatment(
            object,
            edge,
            TreatmentParameters::Roundover {
                radius,
                max_chord_error,
            },
        )
    }
    /// Frozen candidate metadata, not a certification that the source is
    /// still current (commit performs that check). u64 identities are decimal
    /// strings to avoid JavaScript Number precision loss. Lengths are meters.
    pub fn edge_treatment_info_json(&self, handle: u32) -> Result<String, ApiError> {
        let p = self.treatment(handle)?;
        let annotations: Vec<_> = p
            .candidate
            .annotations_to_detach()
            .map(|id| {
                serde_json::json!({
                    "annotation":id.data().as_ffi().to_string(),
                    "sid":self.doc.sid_of(&EntityRef::Annotation(id)).map(|s|s.to_string()),
                })
            })
            .collect();
        let warnings = if annotations.is_empty() {
            vec![]
        } else {
            vec![serde_json::json!({
                "code":"AnnotationDetachment", "annotations":annotations,
                "recorded_points_preserved":true,"undo_restores_attachments":true,
            })]
        };
        serde_json::to_string(&serde_json::json!({
            "handle":handle,"object":p.candidate.source().data().as_ffi().to_string(),
            "object_sid":p.command.target_sid.to_string(),"edge":p.candidate.edge().data().as_ffi().to_string(),
            "edges":p.candidate.edges().iter().map(|e|e.data().as_ffi().to_string()).collect::<Vec<_>>(),
            "segments":p.candidate.segments(),
            "junctions":match p.candidate.operation() {
                EdgeTreatmentOperation::Chamfer { .. } => "planar bevel junctions",
                _ if p.candidate.edges().len()>1 => "two-edge miters; three-edge spherical patches",
                _ => "single edge",
            },
            "operation":p.command.parameters,"removed_volume":p.candidate.result().removed_volume,
            "expected_removed_volume":p.candidate.result().expected_removed_volume,"warnings":warnings,
        })).map_err(|e|ApiError::new("CandidateSerialization",&e.to_string()))
    }
    /// Tessellates the ACTUAL candidate using the existing renderer bridge.
    /// Independent MeshJs buffers: JS releases them with .free(). They remain
    /// usable after discard/commit but cannot be committed back as geometry.
    /// Does not populate the persistent object's face-range cache.
    pub fn edge_treatment_mesh(&self, handle: u32) -> Result<MeshJs, ApiError> {
        let object = &self.treatment(handle)?.candidate.result().object;
        let mesh = tessellate(object, self.doc.materials()).map_err(|e| api_err(&e, &e))?;
        Ok(MeshJs {
            mesh,
            watertight: object.watertight() == WatertightState::Watertight,
        })
    }
    /// Consumes a matching handle on EVERY commit attempt, including stale
    /// failure. Native document validation remains the authority. Record only
    /// a successful semantic edit; prepare/inspect/discard are not commands.
    pub fn commit_edge_treatment(&mut self, handle: u32) -> Result<DocChangeJs, ApiError> {
        self.treatment(handle)?;
        let prepared = self
            .edge_candidate
            .take()
            .ok_or_else(|| stale("InvalidCandidateHandle", "candidate"))?;
        if prepared.context != self.treatment_context()
            || self.treatment_target(prepared.candidate.source()).is_err()
        {
            return Err(ApiError::new(
                "StaleCandidate",
                "Scene context changed; regenerate the candidate",
            ));
        }
        let (_, change) = self
            .doc
            .commit_edge_treatment(prepared.candidate)
            .map_err(boundary_error)?;
        self.reconcile(&change);
        if let Some(endpoints) = prepared.batch_endpoints {
            recording::record(recording::RecordedCall::EdgeTreatmentBatch {
                command: TreatmentBatchCommand {
                    target_sid: prepared.command.target_sid,
                    source_hash: prepared.command.source_hash,
                    endpoints,
                    parameters: prepared.command.parameters,
                },
            });
        } else {
            recording::record(recording::RecordedCall::EdgeTreatment {
                command: prepared.command,
            });
        }
        Ok(DocChangeJs { inner: change })
    }
    /// Frees the matching slot only; invalid/foreign handles do not evict it.
    pub fn discard_edge_treatment(&mut self, handle: u32) -> Result<(), ApiError> {
        self.treatment(handle)?;
        self.edge_candidate = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn polygon(scene: &mut Scene, points: &[(f64, f64)]) -> u64 {
        let s = scene.begin_ground_sketch();
        scene.sketch_begin_gesture(s).unwrap();
        for i in 0..points.len() {
            let (a, b) = points[i];
            let (c, d) = points[(i + 1) % points.len()];
            scene.sketch_add_segment(s, a, b, 0., c, d, 0.).unwrap();
        }
        scene.sketch_end_gesture(s).unwrap();
        let r = scene.sketch_regions(s).unwrap()[0];
        scene.extrude_region(s, r, 4.).unwrap()
    }
    fn fixture() -> (Scene, u64, u64) {
        let mut s = Scene::new();
        let o = polygon(&mut s, &[(0., 0.), (2., 0.), (2., 3.), (0., 3.)]);
        let e = s
            .doc
            .object(object_id(o))
            .unwrap()
            .edges()
            .keys()
            .find(|&e| {
                let (a, b) = s
                    .doc
                    .object(object_id(o))
                    .unwrap()
                    .edge_endpoints(e)
                    .unwrap();
                a.x == 0. && a.y == 0. && b.x == 0. && b.y == 0.
            })
            .unwrap()
            .data()
            .as_ffi();
        (s, o, e)
    }
    fn code<T>(r: Result<T, ApiError>, expected: &str) {
        match r {
            Ok(_) => panic!("expected {expected}"),
            Err(e) => assert!(e.0.starts_with(&format!("{expected}:")), "{}", e.0),
        }
    }
    fn state(s: &Scene) -> (Vec<u8>, u32, u32, u64) {
        (
            s.save(),
            s.undo_depth(),
            s.redo_depth(),
            s.history_generation(),
        )
    }

    #[test]
    fn preview_requests_detect_history_aba_and_reject_unknown_versions() {
        let (mut s, o, e) = fixture();
        let before = s.save();
        let first = s
            .edge_treatment_request_json(o, &[e], false, 0.25, 0.)
            .unwrap();
        s.scene_undo().unwrap();
        s.scene_redo().unwrap();
        assert!(before == s.save());
        let second = s
            .edge_treatment_request_json(o, &[e], false, 0.25, 0.)
            .unwrap();
        assert_ne!(first, second);
        let mut invalid: serde_json::Value = serde_json::from_str(&first).unwrap();
        invalid["version"] = serde_json::json!(99);
        code(
            s.prepare_edge_treatment_request_json(&invalid.to_string()),
            "InvalidPreviewRequest",
        );
        let command = preview_command(&first).unwrap();
        s.load(&before).unwrap();
        let (object, edges) = s
            .recorded_treatment_edges(command.target_sid, command.source_hash, &command.endpoints)
            .unwrap();
        let third = s
            .edge_treatment_request_json(object.data().as_ffi(), &edges, false, 0.25, 0.)
            .unwrap();
        assert_ne!(first, third);
        s.prepare_edge_treatment_request_json(&first).unwrap(); // origin epoch is opaque to an isolated Scene
    }

    #[test]
    fn circular_source_certification_sequential_history_and_isolated_requests() {
        for first_round in [false, true] {
            let mut s = Scene::new();
            s.start_recording();
            let sketch = s.begin_ground_sketch();
            s.sketch_begin_gesture(sketch).unwrap();
            s.sketch_begin_curve_with(sketch, 0., 0., 0., 0.05).unwrap();
            for i in 0..64 {
                let a = std::f64::consts::TAU * i as f64 / 64.;
                let b = std::f64::consts::TAU * (i + 1) as f64 / 64.;
                s.sketch_add_segment(
                    sketch,
                    0.05 * a.cos(),
                    0.05 * a.sin(),
                    0.,
                    0.05 * b.cos(),
                    0.05 * b.sin(),
                    0.,
                )
                .unwrap();
            }
            s.sketch_end_curve(sketch).unwrap();
            s.sketch_end_gesture(sketch).unwrap();
            let r = s.sketch_regions(sketch).unwrap()[0];
            let object = s.extrude_region(sketch, r, 0.04).unwrap();
            let select = |s: &Scene, upper: bool| {
                let o = s.doc.object(object_id(object)).unwrap();
                let cap = o
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
                kernel::edge_treatment::face_perimeter_edges(o, cap)
                    .unwrap()
                    .into_iter()
                    .map(|e| e.data().as_ffi())
                    .collect::<Vec<_>>()
            };
            let original = s.save();
            for (upper, round, size) in [(true, first_round, 0.003), (false, true, 0.002)] {
                let edges = select(&s, upper);
                let before = s.save();
                let depth = s.undo_depth();
                let request = s
                    .edge_treatment_request_json(object, &edges, round, size, 0.0003)
                    .unwrap();
                recording::without_capture(|| {
                    let mut isolated = Scene::new();
                    isolated.load(&before).unwrap();
                    let h = isolated
                        .prepare_edge_treatment_request_json(&request)
                        .unwrap();
                    isolated.edge_treatment_mesh(h).unwrap();
                });
                assert_eq!(s.save(), before);
                assert_eq!(s.undo_depth(), depth);
                let h = s.prepare_edge_treatment_request_json(&request).unwrap();
                s.commit_edge_treatment(h).unwrap();
                let after = s.save();
                s.scene_undo().unwrap();
                assert_eq!(s.save(), before);
                s.scene_redo().unwrap();
                assert_eq!(s.save(), after);
            }
            assert_ne!(s.save(), original);
            s.stop_recording();
            let rec = s.take_recording();
            let mut replay = Scene::new();
            replay.replay(&rec).unwrap();
            assert!(
                s.save() == replay.save(),
                "circular replay must preserve canonical geometry"
            );
        }
    }

    #[test]
    fn polygon_sequential_ranges_export_and_history_share_the_candidate_contract() {
        for n in [3, 4, 6, 8, 11] {
            let mut s = Scene::new();
            s.start_recording();
            let points: Vec<_> = (0..n)
                .map(|i| {
                    let t = std::f64::consts::TAU * i as f64 / n as f64;
                    (0.05 * t.cos(), 0.04 * t.sin())
                })
                .collect();
            let object = polygon(&mut s, &points);
            let cap_edges = |s: &Scene, upper: bool| {
                let source = s.doc.object(object_id(object)).unwrap();
                let face = source
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
                kernel::edge_treatment::face_perimeter_edges(source, face)
                    .unwrap()
                    .iter()
                    .map(|e| e.data().as_ffi())
                    .collect::<Vec<_>>()
            };
            let original = s.save();
            let upper = cap_edges(&s, true);
            let h = s.prepare_chamfer_edges(object, &upper, 0.003).unwrap();
            let before = state(&s);
            let limits: serde_json::Value = serde_json::from_str(
                &s.edge_treatment_limits_json(object, &upper, true, 0.00002)
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(limits["certainty"], "conservative");
            assert!(limits["maximum"].as_f64().unwrap() > 0.003);
            assert_eq!(state(&s), before);
            s.edge_treatment_mesh(h).unwrap(); // read-only query preserves the slot
            s.commit_edge_treatment(h).unwrap();
            let first = s.save();
            let lower = cap_edges(&s, false);
            let h = s
                .prepare_roundover_edges(object, &lower, 0.0003175, 0.00009906)
                .unwrap();
            s.edge_treatment_mesh(h).unwrap();
            s.commit_edge_treatment(h).unwrap();
            let final_state = s.save();
            s.scene_undo().unwrap();
            assert_eq!(s.save(), first);
            s.scene_undo().unwrap();
            assert_eq!(s.save(), original);
            s.scene_redo().unwrap();
            s.scene_redo().unwrap();
            assert_eq!(s.save(), final_state);
            let mut loaded = Scene::new();
            loaded.load(&final_state).unwrap();
            assert_eq!(loaded.save(), final_state);
            s.stop_recording();
            let recording = s.take_recording();
            let mut replay = Scene::new();
            replay.replay(&recording).unwrap();
            assert_eq!(replay.save(), final_state);
        }
    }

    #[test]
    fn prepare_inspect_real_mesh_commit_history_and_persistence() {
        for round in [false, true] {
            let (mut s, o, e) = fixture();
            let before = state(&s);
            let h = if round {
                s.prepare_roundover(o, e, 0.25, 0.005)
            } else {
                s.prepare_chamfer(o, e, 0.25)
            }
            .unwrap();
            let info: serde_json::Value =
                serde_json::from_str(&s.edge_treatment_info_json(h).unwrap()).unwrap();
            assert_eq!(info["object"], o.to_string());
            assert_eq!(info["edge"], e.to_string());
            assert_eq!(
                info["object_sid"],
                s.doc
                    .sid_of(&EntityRef::Object(object_id(o)))
                    .unwrap()
                    .to_string()
            );
            assert_eq!(
                info["operation"]["kind"],
                if round { "roundover" } else { "chamfer" }
            );
            assert_eq!(
                info["operation"][if round { "radius" } else { "distance" }],
                0.25
            );
            if round {
                assert_eq!(info["operation"]["max_chord_error"], 0.005);
            }
            assert_eq!(info["warnings"], serde_json::json!([]));
            assert!(info["removed_volume"].as_f64().unwrap() > 0.);
            let preview = s.edge_treatment_mesh(h).unwrap();
            assert!(!preview.positions().is_empty());
            assert!(preview.watertight);
            assert!(!s.face_range_cache.contains_key(object_id(o)));
            assert_eq!(state(&s), before);
            let change = s.commit_edge_treatment(h).unwrap();
            assert_eq!(change.inner.objects_touched, vec![object_id(o)]);
            assert_eq!(s.undo_depth(), before.1 + 1);
            assert_eq!(s.doc.visible_object_ids().len(), 1);
            assert_eq!(s.object_mesh(o).unwrap().positions(), preview.positions());
            let after = s.save();
            assert_ne!(after, before.0);
            for _ in 0..5 {
                s.scene_undo().unwrap();
                assert_eq!(s.save(), before.0);
                s.scene_redo().unwrap();
                assert_eq!(s.save(), after);
            }
            let mut loaded = Scene::new();
            loaded.load_core(&after).unwrap();
            assert_eq!(loaded.save(), after);
            code(s.commit_edge_treatment(h), "InvalidCandidateHandle");
            code(s.edge_treatment_info_json(h), "InvalidCandidateHandle");
        }
    }
    #[test]
    fn discard_replacement_invalid_foreign_and_dropped_scene_handles() {
        let (mut a, o, e) = fixture();
        let (mut b, bo, be) = fixture();
        let before = state(&a);
        let h = a.prepare_chamfer(o, e, 0.25).unwrap();
        let bh = b.prepare_chamfer(bo, be, 0.25).unwrap();
        assert_ne!(h, bh);
        code(a.discard_edge_treatment(bh), "InvalidCandidateHandle");
        code(b.commit_edge_treatment(h), "InvalidCandidateHandle");
        a.edge_treatment_info_json(h).unwrap();
        b.edge_treatment_info_json(bh).unwrap();
        let preview = a.edge_treatment_mesh(h).unwrap();
        let next = a.prepare_roundover(o, e, 0.3, 0.01).unwrap();
        code(a.commit_edge_treatment(h), "InvalidCandidateHandle");
        // Failed preparation keeps the current candidate and original state.
        code(a.prepare_chamfer(o, e, -1.), "InvalidDistance");
        a.edge_treatment_info_json(next).unwrap();
        a.discard_edge_treatment(next).unwrap();
        code(a.discard_edge_treatment(next), "InvalidCandidateHandle");
        assert_eq!(state(&a), before);
        assert!(!preview.positions().is_empty());
        drop(a);
        let (mut fresh, fo, fe) = fixture();
        fresh.prepare_chamfer(fo, fe, 0.25).unwrap();
        code(fresh.commit_edge_treatment(next), "InvalidCandidateHandle");
    }
    #[test]
    fn parameter_geometry_and_target_errors_preserve_state() {
        let (mut s, o, e) = fixture();
        let before = state(&s);
        code(s.prepare_chamfer(o, e, 0.), "InvalidDistance");
        code(s.prepare_chamfer(o, e, 100.), "InsufficientClearance");
        code(s.prepare_roundover(o, e, f64::NAN, 0.01), "InvalidRadius");
        code(s.prepare_roundover(o, e, 0.25, 0.), "InvalidChordError");
        code(s.prepare_chamfer(o, 0, 0.25), "UnknownEdge");
        code(s.prepare_chamfer(0, e, 0.25), "UnknownObject");
        assert_eq!(state(&s), before);
        let mut triangle = Scene::new();
        let o = polygon(&mut triangle, &[(0., 0.), (2., 0.), (0., 3.)]);
        let edge = triangle
            .doc
            .object(object_id(o))
            .unwrap()
            .edges()
            .keys()
            .find(|&e| {
                let (a, b) = triangle
                    .doc
                    .object(object_id(o))
                    .unwrap()
                    .edge_endpoints(e)
                    .unwrap();
                a.x == 2. && b.x == 2.
            })
            .unwrap()
            .data()
            .as_ffi();
        let before = state(&triangle);
        code(
            triangle.prepare_chamfer(o, edge, 0.25),
            "NonPerpendicularFaces",
        );
        assert_eq!(state(&triangle), before);
    }
    #[test]
    fn hidden_selection_drawing_and_definition_contexts_are_distinct() {
        let (mut s, o, e) = fixture();
        s.set_hidden(&[o], &[]);
        code(s.prepare_chamfer(o, e, 0.25), "HiddenTarget");
        s.set_hidden(&[], &[]);
        s.doc
            .set_node_user_hidden(NodeId::Object(object_id(o)), true);
        code(s.prepare_chamfer(o, e, 0.25), "HiddenTarget");
        s.doc
            .set_node_user_hidden(NodeId::Object(object_id(o)), false);
        s.set_pick_scope(vec![], vec![], false);
        code(s.prepare_chamfer(o, e, 0.25), "UnsupportedSelectionContext");
        s.clear_pick_scope();
        let sk = s.begin_ground_sketch();
        s.sketch_begin_gesture(sk).unwrap();
        code(s.prepare_chamfer(o, e, 0.25), "ActiveEditingSession");
        s.sketch_cancel_gesture();
        s.doc
            .make_component(&[NodeId::Object(object_id(o))])
            .unwrap();
        code(s.prepare_chamfer(o, e, 0.25), "UnsupportedDefinitionMember");
    }
    #[test]
    fn owned_sketches_refuse_and_annotations_disclose_detachment() {
        let (mut s, o, e) = fixture();
        let id = object_id(o);
        let sk = s.doc.add_sketch(
            Plane::from_point_normal(Point3::new(0., 0., 4.), Vec3::new(0., 0., 1.)).unwrap(),
        );
        s.doc.set_sketch_object_owner(sk, Some(id)).unwrap();
        code(s.prepare_chamfer(o, e, 0.25), "OwnedSketchesUnsupported");
        s.doc.set_sketch_object_owner(sk, None).unwrap();
        let annotation = s
            .doc
            .add_leader_text(
                Anchor {
                    node: Some(NodeId::Object(id)),
                    point: Point3::ORIGIN,
                },
                Vec3::new(1., 1., 1.),
                "corner".into(),
            )
            .unwrap();
        let h = s.prepare_chamfer(o, e, 0.25).unwrap();
        let before = state(&s);
        let info: serde_json::Value =
            serde_json::from_str(&s.edge_treatment_info_json(h).unwrap()).unwrap();
        let warning = &info["warnings"][0];
        assert_eq!(warning["code"], "AnnotationDetachment");
        assert_eq!(
            warning["annotations"][0]["annotation"],
            annotation.data().as_ffi().to_string()
        );
        assert_eq!(warning["recorded_points_preserved"], true);
        assert_eq!(warning["undo_restores_attachments"], true);
        assert_eq!(state(&s), before);
        s.commit_edge_treatment(h).unwrap();
        assert_eq!(s.doc.annotation_detached(annotation), Some(true));
        s.scene_undo().unwrap();
        assert_eq!(s.doc.annotation_detached(annotation), Some(false));
        assert_eq!(s.save(), before.0);
    }
    #[test]
    fn stale_document_and_scene_context_consume_without_mutation() {
        for kind in 0..6 {
            let (mut s, o, e) = fixture();
            let h = s.prepare_chamfer(o, e, 0.25).unwrap();
            match kind {
                0 => {
                    s.doc
                        .set_node_name(
                            NodeId::Object(object_id(o)),
                            Some("unrelated metadata".into()),
                        )
                        .unwrap();
                }
                1 => {
                    s.scene_undo().unwrap();
                    s.scene_redo().unwrap();
                }
                2 => s.set_hidden(&[o], &[]),
                3 => s.set_pick_scope(vec![], vec![], false),
                4 => {
                    s.doc.delete_node(NodeId::Object(object_id(o))).unwrap();
                }
                _ => {
                    s.begin_ground_sketch();
                }
            }
            let before = state(&s);
            code(s.commit_edge_treatment(h), "StaleCandidate");
            assert_eq!(state(&s), before);
            code(s.commit_edge_treatment(h), "InvalidCandidateHandle");
        }
    }
    #[test]
    fn load_releases_candidates_but_failed_parse_preserves_them() {
        let (mut s, o, e) = fixture();
        let h = s.prepare_chamfer(o, e, 0.25).unwrap();
        let before = state(&s);
        assert!(s.load_core(b"invalid").is_err());
        s.edge_treatment_info_json(h).unwrap();
        assert_eq!(state(&s), before);
        s.load_core(&before.0).unwrap();
        assert!(s.edge_candidate.is_none());
        code(s.edge_treatment_mesh(h), "InvalidCandidateHandle");
    }
    #[test]
    fn caller_nested_transactions_and_abort_are_respected() {
        let (mut s, o, e) = fixture();
        let before = state(&s);
        let outer = s.doc.begin_transaction();
        s.history_bracket_open().unwrap();
        let h = s.prepare_chamfer(o, e, 0.25).unwrap();
        s.commit_edge_treatment(h).unwrap();
        s.history_bracket_commit("Edge").unwrap();
        s.doc
            .commit_transaction(
                outer,
                kernel::CompoundMeta {
                    label: "Outer".into(),
                    origin: HistoryOrigin::User,
                },
            )
            .unwrap();
        assert_eq!(s.undo_depth(), before.1 + 1);
        let after = s.save();
        s.scene_undo().unwrap();
        assert_eq!(s.save(), before.0);
        s.scene_redo().unwrap();
        assert_eq!(s.save(), after);
        s.scene_undo().unwrap();
        s.history_bracket_open().unwrap();
        let h = s.prepare_chamfer(o, e, 0.25).unwrap();
        s.commit_edge_treatment(h).unwrap();
        s.history_bracket_abort();
        assert_eq!(s.save(), before.0);
        code(s.commit_edge_treatment(h), "InvalidCandidateHandle");
    }
    #[test]
    fn recording_replays_both_operations_undo_redo_and_load() {
        for round in [false, true] {
            let (mut s, _, _) = fixture();
            let seed = s.save();
            s.start_recording();
            s.load_core(&seed).unwrap();
            // Load may remap runtime handles. Resolve from the current solid.
            let id = s.doc.visible_object_ids()[0];
            let object = id.data().as_ffi();
            let edge = s
                .doc
                .object(id)
                .unwrap()
                .edges()
                .keys()
                .find(|&e| {
                    let (a, b) = s.doc.object(id).unwrap().edge_endpoints(e).unwrap();
                    a.x == 0. && a.y == 0. && b.x == 0. && b.y == 0.
                })
                .unwrap()
                .data()
                .as_ffi();
            let discarded = s.prepare_chamfer(object, edge, 0.1).unwrap();
            s.discard_edge_treatment(discarded).unwrap();
            let h = if round {
                s.prepare_roundover(object, edge, 0.25, 0.005)
            } else {
                s.prepare_chamfer(object, edge, 0.25)
            }
            .unwrap();
            s.edge_treatment_mesh(h).unwrap();
            s.commit_edge_treatment(h).unwrap();
            s.scene_undo().unwrap();
            s.scene_redo().unwrap();
            let expected = s.save();
            let hash = s.doc.state_hash();
            s.stop_recording();
            let json = s.take_recording();
            let rec: recording::Recording = serde_json::from_str(&json).unwrap();
            assert_eq!(
                rec.calls
                    .iter()
                    .filter(|c| matches!(c, recording::RecordedCall::EdgeTreatment { .. }))
                    .count(),
                1
            );
            let mut replay = Scene::new();
            assert_eq!(replay.replay(&json).unwrap(), hash);
            assert_eq!(replay.save(), expected);
            assert_eq!(replay.undo_depth(), s.undo_depth());
        }
    }
    #[test]
    fn replay_checks_source_sid_and_exact_edge_without_guessing() {
        let (mut s, o, e) = fixture();
        let h = s.prepare_chamfer(o, e, 0.25).unwrap();
        let command = s.treatment(h).unwrap().command.clone();
        let before = state(&s);
        let mut wrong = command.clone();
        wrong.source_hash ^= 1;
        code(s.replay_treatment(wrong), "ReplaySourceMismatch");
        let mut wrong = command.clone();
        wrong.target_sid = u64::MAX;
        code(s.replay_treatment(wrong), "UnknownObject");
        let mut wrong = command.clone();
        wrong.endpoints[0][0] += 0.001;
        code(s.replay_treatment(wrong), "ReplayEdgeMismatch");
        assert_eq!(state(&s), before);
        // Endpoint order is geometric, not the edge's half-edge orientation.
        let mut reverse = command;
        reverse.endpoints.swap(0, 1);
        s.replay_treatment(reverse).unwrap();
        assert_ne!(s.save(), before.0);
    }
    #[test]
    fn recording_resolves_edges_after_topology_reconstruction() {
        let mut s = Scene::new();
        s.start_recording();
        let o = polygon(&mut s, &[(0., 0.), (2., 0.), (2., 3.), (0., 3.)]);
        for (x, y, round) in [(0., 0., false), (2., 3., true)] {
            let id = object_id(o);
            let e = s
                .doc
                .object(id)
                .unwrap()
                .edges()
                .keys()
                .find(|&e| {
                    let (a, b) = s.doc.object(id).unwrap().edge_endpoints(e).unwrap();
                    a.x == x && b.x == x && a.y == y && b.y == y
                })
                .unwrap()
                .data()
                .as_ffi();
            let h = if round {
                s.prepare_roundover(o, e, 0.23456789012345, 0.005)
            } else {
                s.prepare_chamfer(o, e, 0.23456789012345)
            }
            .unwrap();
            s.commit_edge_treatment(h).unwrap();
        }
        s.stop_recording();
        let json = s.take_recording();
        let expected = s.save();
        let mut replay = Scene::new();
        assert_eq!(replay.replay(&json).unwrap(), s.doc.state_hash());
        assert_eq!(replay.save(), expected);
    }
    #[test]
    fn stl_precision_refusal_is_atomic_and_finer_request_roundtrips_history() {
        let mut s = Scene::new();
        let side = 0.3048;
        let sketch = s.begin_ground_sketch();
        s.sketch_begin_gesture(sketch).unwrap();
        let p = [(0.0, 0.0), (side, 0.0), (side, side), (0.0, side)];
        for i in 0..4 {
            s.sketch_add_segment(
                sketch,
                p[i].0,
                p[i].1,
                0.0,
                p[(i + 1) % 4].0,
                p[(i + 1) % 4].1,
                0.0,
            )
            .unwrap();
        }
        s.sketch_end_gesture(sketch).unwrap();
        let region = s.sketch_regions(sketch).unwrap()[0];
        let object = s.extrude_region(sketch, region, side).unwrap();
        let source = s.doc.object(object_id(object)).unwrap();
        let edge = source
            .edges()
            .keys()
            .find(|&e| {
                let (a, b) = source.edge_endpoints(e).unwrap();
                a.x == side && b.x == side && a.y == 0.0 && b.y == 0.0
            })
            .unwrap()
            .data()
            .as_ffi();
        let before = s.save();
        let depth = s.doc.undo_depth();
        s.start_recording();
        code(
            s.prepare_roundover(object, edge, 0.1016, 0.00001),
            "StlChordPrecision",
        );
        assert_eq!(s.save(), before);
        assert_eq!(s.doc.undo_depth(), depth);
        let h = s.prepare_roundover(object, edge, 0.00635, 0.00001).unwrap();
        s.discard_edge_treatment(h).unwrap();
        let h = s
            .prepare_roundover(object, edge, 0.1016, 0.0000099)
            .unwrap();
        s.commit_edge_treatment(h).unwrap();
        assert_eq!(s.doc.undo_depth(), depth + 1);
        let after = s.save();
        assert!(s.export("stl", 0).unwrap().is_some());
        s.scene_undo().unwrap();
        assert_eq!(s.save(), before);
        s.scene_redo().unwrap();
        assert_eq!(s.save(), after);
        s.stop_recording();
        let recorded = s.take_recording();
        let calls: serde_json::Value = serde_json::from_str(&recorded).unwrap();
        assert_eq!(
            calls["calls"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|v| v["method"] == "edge_treatment")
                .count(),
            1
        );
    }
    #[test]
    fn batch_treatments_preview_commit_history_save_and_replay() {
        for round in [false, true] {
            let (mut s, o, _) = fixture();
            let id = object_id(o);
            let edges: Vec<_> = s
                .doc
                .object(id)
                .unwrap()
                .edges()
                .keys()
                .map(|e| e.data().as_ffi())
                .collect();
            let before = s.save();
            let depth = s.doc.undo_depth();
            s.start_recording();
            let h = if round {
                s.prepare_roundover_edges(o, &edges, 0.25, 0.005)
            } else {
                s.prepare_chamfer_edges(o, &edges, 0.25)
            }
            .unwrap();
            let info: serde_json::Value =
                serde_json::from_str(&s.edge_treatment_info_json(h).unwrap()).unwrap();
            assert_eq!(info["edges"].as_array().unwrap().len(), 12);
            assert_eq!(s.save(), before);
            assert_eq!(s.doc.undo_depth(), depth);
            let preview = s.edge_treatment_mesh(h).unwrap().positions();
            s.commit_edge_treatment(h).unwrap();
            assert_eq!(s.doc.undo_depth(), depth + 1);
            assert_eq!(s.object_mesh(o).unwrap().positions(), preview);
            let after = s.save();
            s.scene_undo().unwrap();
            assert_eq!(s.save(), before);
            s.scene_redo().unwrap();
            assert_eq!(s.save(), after);
            let mut loaded = Scene::new();
            loaded.load(&after).unwrap();
            assert_eq!(loaded.save(), after);
            s.stop_recording();
            let recording = s.take_recording();
            let mut replay = Scene::new();
            replay.load(&before).unwrap();
            replay.replay(&recording).unwrap();
            assert_eq!(replay.save(), after);
            assert!(recording.contains("edge_treatment_batch"));
        }
    }
    #[test]
    fn batch_invalid_stale_and_cancel_are_atomic() {
        let (mut s, o, _) = fixture();
        let id = object_id(o);
        let edges: Vec<_> = s
            .doc
            .object(id)
            .unwrap()
            .edges()
            .keys()
            .map(|e| e.data().as_ffi())
            .collect();
        let before = s.save();
        code(
            s.prepare_roundover_edges(o, &[], 0.25, 0.005),
            "InvalidSelection",
        );
        code(
            s.prepare_chamfer_edges(o, &[edges[0], edges[0]], 0.25),
            "InvalidSelection",
        );
        let h = s.prepare_roundover_edges(o, &edges, 0.25, 0.005).unwrap();
        s.discard_edge_treatment(h).unwrap();
        assert_eq!(s.save(), before);
        let h = s.prepare_roundover_edges(o, &edges, 0.25, 0.005).unwrap();
        s.scene_undo().unwrap();
        let newer = s.save();
        code(s.commit_edge_treatment(h), "StaleCandidate");
        assert_eq!(s.save(), newer);
    }
}
