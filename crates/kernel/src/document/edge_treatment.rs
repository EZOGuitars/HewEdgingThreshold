//! Detached edge candidates and atomic, identity-preserving document edits.
//! Geometry remains exclusively in crate::edge_treatment. No cutter is ever
//! inserted into the document, and no new transaction bracket is opened.

use super::*;
use crate::edge_treatment::{self, EdgeTreatmentError, EdgeTreatmentResult};
use std::sync::Arc;

/// Parameters frozen into a candidate; commit accepts no replacement values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EdgeTreatmentOperation {
    Chamfer { distance: f64 },
    Roundover { radius: f64, max_chord_error: f64 },
}

/// Document adapter failures, distinct from geometry failures and history's
/// existing DocumentError. All failures leave the document untouched.
#[derive(Debug, Clone, PartialEq)]
pub enum DocumentEdgeTreatmentError {
    Document(DocumentError),
    Geometry(EdgeTreatmentError),
    /// Different logical document, conflicting history/content, or context.
    StaleCandidate,
    /// Definition members, open sessions, or pending drawing gestures.
    UnsupportedContext,
    /// Object-owned drawing cannot yet be certified on rebuilt faces.
    OwnedSketchesUnsupported,
    /// Candidate topology no longer meets its validated solid contract.
    InvalidCandidate,
}

impl std::fmt::Display for DocumentEdgeTreatmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Document(e) => write!(f, "{e}"),
            Self::Geometry(e) => write!(f, "{e}"),
            Self::StaleCandidate => write!(
                f,
                "edge candidate is stale; regenerate it from the current document"
            ),
            Self::UnsupportedContext => write!(
                f,
                "edge treatments require a world object outside editing and drawing sessions"
            ),
            Self::OwnedSketchesUnsupported => write!(
                f,
                "edge treatment cannot yet preserve object-owned sketches on rebuilt faces"
            ),
            Self::InvalidCandidate => write!(f, "edge candidate is not a valid watertight solid"),
        }
    }
}
impl std::error::Error for DocumentEdgeTreatmentError {}

/// An owned, opaque, single-use candidate, independent of the live document.
/// Only immutable geometry is exposed, so callers cannot substitute an
/// unvalidated mesh. Dropping it cancels without any document mutation.
#[derive(Debug)]
pub struct EdgeTreatmentCandidate {
    source: ObjectId,
    edge: EdgeId,
    operation: EdgeTreatmentOperation,
    result: EdgeTreatmentResult,
    identity: Arc<()>,
    generation: u64,
    // Deliberately conservative first guard: exact canonical content, not a
    // collision-prone hash. Also catches non-undoable visibility/palette edits.
    // Unrelated content/view changes may invalidate a candidate too. Cost is
    // O(document size); this does not rerun the geometric operation on commit.
    state: Vec<u8>,
    reanchored: Vec<AnnotationReanchor>,
}

impl EdgeTreatmentCandidate {
    pub fn source(&self) -> ObjectId {
        self.source
    }
    /// Valid only in the source topology BEFORE commit. Re-pick afterwards.
    pub fn edge(&self) -> EdgeId {
        self.edge
    }
    pub fn operation(&self) -> EdgeTreatmentOperation {
        self.operation
    }
    pub fn result(&self) -> &EdgeTreatmentResult {
        &self.result
    }
    /// Node anchors remain frozen at their recorded points, explicitly
    /// detached because geometric associativity through a rebuild is unproven.
    pub fn annotations_to_detach(&self) -> impl Iterator<Item = AnnotationId> + '_ {
        self.reanchored.iter().map(|r| r.annotation)
    }
}

impl Document {
    fn edge_treatment_target(&self, object: ObjectId) -> Result<(), DocumentEdgeTreatmentError> {
        use DocumentEdgeTreatmentError as E;
        if !self.sessions.is_empty()
            || self.pending_sketch_gesture.is_some()
            || self.pending_draft.is_some()
        {
            return Err(E::UnsupportedContext);
        }
        let record = self
            .objects
            .get(object)
            .filter(|r| !r.hidden)
            .ok_or(E::Document(DocumentError::UnknownObject))?;
        if !record.is_world() {
            return Err(E::UnsupportedContext);
        }
        let mut node = Some(NodeId::Object(object));
        while let Some(current) = node {
            if self.node_user_hidden(current)
                || self
                    .node_tags(current)
                    .iter()
                    .any(|tag| (1..=tag.len()).any(|n| self.tag_hidden(&tag[..n])))
            {
                return Err(E::Document(DocumentError::UnknownObject));
            }
            node = self.node_parent(current).map(NodeId::Group);
        }
        if !self.object_owned_sketches(object).is_empty() {
            return Err(E::OwnedSketchesUnsupported);
        }
        Ok(())
    }

    /// Pure candidate generation for a visible world object (grouped or
    /// top-level). Active editing/drawing sessions and owned sketches refuse.
    /// Annotated source nodes are conservatively detached at commit, exposed
    /// by annotations_to_detach; unrelated/free annotations remain unchanged.
    /// No topological reference is redirected to a fresh face/edge handle.
    pub fn prepare_edge_treatment(
        &self,
        object: ObjectId,
        edge: EdgeId,
        operation: EdgeTreatmentOperation,
    ) -> Result<EdgeTreatmentCandidate, DocumentEdgeTreatmentError> {
        self.edge_treatment_target(object)?;
        let source = &self.objects[object].object;
        let result = match operation {
            EdgeTreatmentOperation::Chamfer { distance } => {
                edge_treatment::chamfer(source, edge, distance)
            }
            EdgeTreatmentOperation::Roundover {
                radius,
                max_chord_error,
            } => edge_treatment::roundover(source, edge, radius, max_chord_error).map(|r| r.result),
        }
        .map_err(DocumentEdgeTreatmentError::Geometry)?;
        let reanchored = self
            .annotations
            .iter()
            .filter(|(_, r)| {
                !r.hidden && !r.detached && r.annotation.touches_any(&[NodeId::Object(object)])
            })
            .map(|(annotation, r)| AnnotationReanchor {
                annotation,
                before: r.annotation.clone(),
                after: r.annotation.clone(),
                before_detached: false,
                after_detached: true,
            })
            .collect();
        Ok(EdgeTreatmentCandidate {
            source: object,
            edge,
            operation,
            result,
            identity: self.edge_candidate_identity.clone(),
            generation: self.history_generation(),
            state: self.save(),
            reanchored,
        })
    }

    /// Consume a compatible candidate without recomputing its geometry.
    /// ObjectId/SID, owner/group, name/tags, visibility, attributes, and the
    /// per-object History remain in place. Face/edge IDs are NOT stable.
    /// One ordinary document action participates in a caller's transaction;
    /// no internal begin/commit bracket is needed: all fallible work precedes
    /// the first write. Exact snapshots drive undo/redo without a new sweep.
    ///
    /// New/load documents reject foreign candidates. Document clones retain
    /// logical identity for checkpoints, but still require exact matching
    /// content and history. Candidates are ephemeral and never serialized.
    pub fn commit_edge_treatment(
        &mut self,
        candidate: EdgeTreatmentCandidate,
    ) -> Result<(ObjectId, DocChange), DocumentEdgeTreatmentError> {
        use DocumentEdgeTreatmentError as E;
        if !Arc::ptr_eq(&self.edge_candidate_identity, &candidate.identity)
            || self.history_generation() != candidate.generation
            || self.save() != candidate.state
            || self.edge_treatment_target(candidate.source).is_err()
        {
            return Err(E::StaleCandidate);
        }
        if candidate.result.object.validate().is_err()
            || candidate.result.object.watertight() != WatertightState::Watertight
        {
            return Err(E::InvalidCandidate);
        }
        let object = candidate.source;
        let after = candidate.result.object;
        let before = std::mem::replace(&mut self.objects[object].object, after.clone());
        for change in &candidate.reanchored {
            self.annotations[change.annotation].detached = change.after_detached;
        }
        self.undo.push(DocAction::EdgeTreated {
            object,
            operation: candidate.operation,
            before,
            after,
            reanchored: candidate.reanchored,
        });
        self.commit_new_action();
        self.debug_validate();
        Ok((
            object,
            DocChange {
                objects_touched: vec![object],
                ..DocChange::default()
            },
        ))
    }

    pub fn chamfer_edge(
        &mut self,
        object: ObjectId,
        edge: EdgeId,
        distance: f64,
    ) -> Result<(ObjectId, DocChange), DocumentEdgeTreatmentError> {
        let candidate = self.prepare_edge_treatment(
            object,
            edge,
            EdgeTreatmentOperation::Chamfer { distance },
        )?;
        self.commit_edge_treatment(candidate)
    }

    pub fn roundover_edge(
        &mut self,
        object: ObjectId,
        edge: EdgeId,
        radius: f64,
        max_chord_error: f64,
    ) -> Result<(ObjectId, DocChange), DocumentEdgeTreatmentError> {
        let candidate = self.prepare_edge_treatment(
            object,
            edge,
            EdgeTreatmentOperation::Roundover {
                radius,
                max_chord_error,
            },
        )?;
        self.commit_edge_treatment(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::Key;

    fn fixture() -> (Document, ObjectId, EdgeId) {
        let mut doc = Document::new();
        let plane = Plane::from_point_normal(Point3::ORIGIN, Vec3::new(0., 0., 1.)).unwrap();
        let sketch = doc.add_sketch(plane);
        let points = [
            Point3::ORIGIN,
            Point3::new(2., 0., 0.),
            Point3::new(2., 3., 0.),
            Point3::new(0., 3., 0.),
        ];
        for i in 0..4 {
            doc.sketch_mut(sketch)
                .unwrap()
                .add_segment(points[i], points[(i + 1) % 4])
                .unwrap();
        }
        let region = doc.extrudable_regions(sketch).unwrap()[0];
        let object = doc.extrude_region(sketch, region, 4.).unwrap().0;
        let edge = doc
            .object(object)
            .unwrap()
            .edges()
            .keys()
            .find(|&e| {
                let (a, b) = doc.object(object).unwrap().edge_endpoints(e).unwrap();
                a.x == 0. && a.y == 0. && b.x == 0. && b.y == 0.
            })
            .unwrap();
        (doc, object, edge)
    }

    #[test]
    fn invalid_candidate_cannot_partially_commit() {
        let (mut doc, object, edge) = fixture();
        let mut candidate = doc
            .prepare_edge_treatment(
                object,
                edge,
                EdgeTreatmentOperation::Chamfer { distance: 0.25 },
            )
            .unwrap();
        candidate.result.object = Object::triangle();
        let before = doc.save();
        let generation = doc.history_generation();
        assert_eq!(
            doc.commit_edge_treatment(candidate).unwrap_err(),
            DocumentEdgeTreatmentError::InvalidCandidate
        );
        assert_eq!(doc.save(), before);
        assert_eq!(doc.history_generation(), generation);
    }

    #[test]
    fn unknown_edge_and_open_surface_refuse_atomically() {
        let (mut doc, object, edge) = fixture();
        let before = doc.save();
        let generation = doc.history_generation();
        assert!(matches!(
            doc.chamfer_edge(object, EdgeId::null(), 0.25),
            Err(DocumentEdgeTreatmentError::Geometry(_))
        ));
        assert_eq!(doc.save(), before);
        assert_eq!(doc.history_generation(), generation);
        doc.objects[object].object = Object::triangle();
        let before = doc.save();
        assert!(matches!(
            doc.chamfer_edge(object, edge, 0.25),
            Err(DocumentEdgeTreatmentError::Geometry(_))
        ));
        assert_eq!(doc.save(), before);
        assert_eq!(doc.history_generation(), generation);
    }
}
