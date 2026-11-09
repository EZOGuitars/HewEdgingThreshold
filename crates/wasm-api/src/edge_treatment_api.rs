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
    fn prepare_treatment(
        &mut self,
        object: u64,
        edge: u64,
        parameters: TreatmentParameters,
    ) -> Result<u32, ApiError> {
        let id = object_id(object);
        self.treatment_target(id)?;
        let candidate = self
            .doc
            .prepare_edge_treatment(id, edge_id(edge), parameters.into())
            .map_err(boundary_error)?;
        // Geometry resolved the edge before this read. Still return a typed
        // refusal if that invariant ever changes, never unwrap a JS handle.
        let (a, b) = self
            .doc
            .object(id)
            .and_then(|o| o.edge_endpoints(edge_id(edge)))
            .ok_or_else(|| stale("UnknownEdge", "edge"))?;
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
            endpoints: [coordinates(a), coordinates(b)],
            parameters,
        };
        self.edge_candidate = Some(PreparedTreatment {
            handle,
            candidate,
            context: self.treatment_context(),
            command,
        });
        Ok(handle)
    }
    fn treatment(&self, handle: u32) -> Result<&PreparedTreatment, ApiError> {
        self.edge_candidate
            .as_ref()
            .filter(|p| p.handle == handle)
            .ok_or_else(|| stale("InvalidCandidateHandle", "candidate"))
    }
    pub(crate) fn replay_treatment(&mut self, command: TreatmentCommand) -> Result<(), ApiError> {
        if self.doc.state_hash() != command.source_hash {
            return Err(ApiError::new(
                "ReplaySourceMismatch",
                "edge treatment source state differs",
            ));
        }
        let id = self
            .doc
            .sids()
            .find_map(|(entity, sid)| match entity {
                EntityRef::Object(id) if sid == command.target_sid => Some(*id),
                _ => None,
            })
            .ok_or_else(|| stale("UnknownObject", "recorded target SID"))?;
        let object = self
            .doc
            .object(id)
            .ok_or_else(|| stale("UnknownObject", "recorded target"))?;
        let mut matches = object.edges().keys().filter(|&e| {
            object.edge_endpoints(e).is_some_and(|(a, b)| {
                let ends = [coordinates(a), coordinates(b)];
                ends == command.endpoints || [ends[1], ends[0]] == command.endpoints
            })
        });
        let edge = matches.next().ok_or_else(|| {
            ApiError::new(
                "ReplayEdgeMismatch",
                "recorded edge endpoints do not resolve",
            )
        })?;
        if matches.next().is_some() {
            return Err(ApiError::new(
                "ReplayEdgeMismatch",
                "recorded edge is ambiguous",
            ));
        }
        let handle =
            self.prepare_treatment(id.data().as_ffi(), edge.data().as_ffi(), command.parameters)?;
        self.commit_edge_treatment(handle)?;
        Ok(())
    }
}

#[wasm_bindgen]
impl Scene {
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
        recording::record(recording::RecordedCall::EdgeTreatment {
            command: prepared.command,
        });
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
}
