use kernel::*;
use proptest::prelude::*;
fn plane() -> Plane {
    Plane::from_point_normal(Point3::ORIGIN, Vec3::new(0., 0., 1.)).unwrap()
}
fn block(d: &mut Document, x: f64) -> ObjectId {
    let s = d.add_sketch(plane());
    let p = [
        Point3::new(x, 0., 0.),
        Point3::new(x + 2., 0., 0.),
        Point3::new(x + 2., 3., 0.),
        Point3::new(x, 3., 0.),
    ];
    for i in 0..4 {
        d.sketch_mut(s)
            .unwrap()
            .add_segment(p[i], p[(i + 1) % 4])
            .unwrap();
    }
    let r = d.extrudable_regions(s).unwrap()[0];
    d.extrude_region(s, r, 4.).unwrap().0
}
fn edge(d: &Document, id: ObjectId) -> EdgeId {
    let o = d.object(id).unwrap();
    o.edges()
        .keys()
        .find(|&e| {
            let (a, b) = o.edge_endpoints(e).unwrap();
            a.x == 0. && a.y == 0. && b.x == 0. && b.y == 0.
        })
        .unwrap()
}
fn op() -> EdgeTreatmentOperation {
    EdgeTreatmentOperation::Chamfer { distance: 0.25 }
}
fn candidate(d: &Document, id: ObjectId) -> EdgeTreatmentCandidate {
    d.prepare_edge_treatment(id, edge(d, id), op()).unwrap()
}
fn meta() -> CompoundMeta {
    CompoundMeta {
        label: "Treatment".into(),
        origin: HistoryOrigin::Connection("test".into()),
    }
}
fn stale(d: &mut Document, c: EdgeTreatmentCandidate) {
    let before = d.save();
    let depth = d.undo_depth();
    let generation = d.history_generation();
    assert_eq!(
        d.commit_edge_treatment(c).unwrap_err(),
        DocumentEdgeTreatmentError::StaleCandidate
    );
    assert_eq!(d.save(), before);
    assert_eq!(d.undo_depth(), depth);
    assert_eq!(d.history_generation(), generation);
}
#[test]
fn both_modes_exact_history_and_identity() {
    for operation in [
        op(),
        EdgeTreatmentOperation::Roundover {
            radius: 0.25,
            max_chord_error: 0.005,
        },
    ] {
        let mut d = Document::new();
        let id = block(&mut d, 0.);
        let other = block(&mut d, 5.);
        let unrelated = format!("{:?}", d.object(other));
        let sid = d.sid_of(&EntityRef::Object(id));
        let before = d.save();
        let depth = d.undo_depth();
        let c = d
            .prepare_edge_treatment(id, edge(&d, id), operation)
            .unwrap();
        assert_eq!(c.source(), id);
        assert_eq!(c.operation(), operation);
        assert!(c.result().removed_volume > 0.);
        c.result().object.validate().unwrap();
        assert_eq!(c.result().object.watertight(), WatertightState::Watertight);
        if matches!(operation, EdgeTreatmentOperation::Chamfer { .. }) {
            assert!((c.result().removed_volume - 0.125).abs() < 1e-10);
        }
        assert_eq!(d.save(), before);
        assert_eq!(d.undo_depth(), depth);
        let (result, change) = d.commit_edge_treatment(c).unwrap();
        assert_eq!(result, id);
        assert_eq!(change.objects_touched, vec![id]);
        assert_eq!(d.sid_of(&EntityRef::Object(id)), sid);
        assert_eq!(d.visible_object_ids().len(), 2);
        assert_eq!(format!("{:?}", d.object(other)), unrelated);
        assert_eq!(d.undo_depth(), depth + 1);
        let after = d.save();
        assert_ne!(before, after);
        for _ in 0..20 {
            d.undo().unwrap();
            assert_eq!(d.save(), before);
            d.redo().unwrap();
            assert_eq!(d.save(), after);
        }
    }
}
#[test]
fn cancel_invalid_parameters_and_deleted_source() {
    let mut d = Document::new();
    let id = block(&mut d, 0.);
    let before = d.save();
    let generation = d.history_generation();
    drop(candidate(&d, id));
    for operation in [
        EdgeTreatmentOperation::Chamfer { distance: -1. },
        EdgeTreatmentOperation::Chamfer { distance: 100. },
        EdgeTreatmentOperation::Roundover {
            radius: 0.,
            max_chord_error: 0.01,
        },
        EdgeTreatmentOperation::Roundover {
            radius: 0.2,
            max_chord_error: f64::NAN,
        },
    ] {
        assert!(matches!(
            d.prepare_edge_treatment(id, edge(&d, id), operation),
            Err(DocumentEdgeTreatmentError::Geometry(_))
        ));
        assert_eq!(d.save(), before);
        assert_eq!(d.history_generation(), generation);
    }
    let c = candidate(&d, id);
    d.delete_node(NodeId::Object(id)).unwrap();
    assert!(matches!(
        d.prepare_edge_treatment(id, c.edge(), op()),
        Err(DocumentEdgeTreatmentError::Document(
            DocumentError::UnknownObject
        ))
    ));
    stale(&mut d, c);
    d.undo().unwrap();
    assert_eq!(d.save(), before);
}
#[test]
fn geometry_metadata_visibility_and_aba_reject_stale() {
    for kind in 0..5 {
        let mut d = Document::new();
        let id = block(&mut d, 0.);
        let c = candidate(&d, id);
        match kind {
            0 => {
                d.transform_object(id, &Transform::translation(Vec3::new(1., 0., 0.)))
                    .unwrap();
            }
            1 => {
                d.set_node_name(NodeId::Object(id), Some("edited".into()))
                    .unwrap();
            }
            2 => d.set_node_user_hidden(NodeId::Object(id), true),
            3 => {
                d.set_node_name(NodeId::Object(id), Some("edited".into()))
                    .unwrap();
                d.undo().unwrap();
            }
            _ => {
                d.undo().unwrap();
                d.redo().unwrap();
            }
        }
        stale(&mut d, c);
    }
}
#[test]
fn foreign_reload_and_checkpoint_identity() {
    let mut a = Document::new();
    let id = block(&mut a, 0.);
    let mut b = Document::new();
    block(&mut b, 0.);
    assert_eq!(a.save(), b.save());
    stale(&mut b, candidate(&a, id));
    let mut loaded = Document::load(&a.save()).unwrap();
    stale(&mut loaded, candidate(&a, id));
    let mut clone = a.clone();
    clone.commit_edge_treatment(candidate(&a, id)).unwrap();
    assert_ne!(clone.save(), a.save());
}
#[test]
fn nested_transaction_and_abort() {
    let mut d = Document::new();
    let id = block(&mut d, 0.);
    let before = d.save();
    let depth = d.undo_depth();
    let outer = d.begin_transaction();
    let inner = d.begin_transaction();
    d.chamfer_edge(id, edge(&d, id), 0.25).unwrap();
    d.commit_transaction(inner, meta()).unwrap();
    d.set_node_name(NodeId::Object(id), Some("treated".into()))
        .unwrap();
    assert!(d.commit_transaction(outer, meta()).unwrap());
    assert_eq!(d.undo_depth(), depth + 1);
    let after = d.save();
    d.undo().unwrap();
    assert_eq!(d.save(), before);
    d.redo().unwrap();
    assert_eq!(d.save(), after);
    d.undo().unwrap();
    let txn = d.begin_transaction();
    d.chamfer_edge(id, edge(&d, id), 0.25).unwrap();
    d.abort_transaction(txn);
    assert_eq!(d.save(), before);
    assert_eq!(d.redo_depth(), 1);
    let txn = d.begin_transaction();
    d.set_node_name(NodeId::Object(id), Some("caller".into()))
        .unwrap();
    let caller = d.save();
    assert!(d.chamfer_edge(id, edge(&d, id), -1.).is_err());
    assert_eq!(d.save(), caller);
    d.abort_transaction(txn);
    assert_eq!(d.save(), before);
}
#[test]
fn owned_sketch_and_edit_context_refuse() {
    let mut d = Document::new();
    let id = block(&mut d, 0.);
    let c = candidate(&d, id);
    let s = d.add_sketch(
        Plane::from_point_normal(Point3::new(0., 0., 4.), Vec3::new(0., 0., 1.)).unwrap(),
    );
    d.set_sketch_object_owner(s, Some(id)).unwrap();
    let before = d.save();
    assert_eq!(
        d.prepare_edge_treatment(id, edge(&d, id), op())
            .unwrap_err(),
        DocumentEdgeTreatmentError::OwnedSketchesUnsupported
    );
    assert_eq!(d.save(), before);
    stale(&mut d, c);
    d.set_sketch_object_owner(s, None).unwrap();
    let group = d.group_nodes(&[NodeId::Object(id)]).unwrap().0;
    let c = candidate(&d, id);
    d.open_group_session(group).unwrap();
    let before = d.save();
    assert_eq!(
        d.prepare_edge_treatment(id, edge(&d, id), op())
            .unwrap_err(),
        DocumentEdgeTreatmentError::UnsupportedContext
    );
    assert_eq!(d.save(), before);
    stale(&mut d, c);
}
#[test]
fn annotation_detachment_is_explicit_and_reversible() {
    let mut d = Document::new();
    let id = block(&mut d, 0.);
    let attached = d
        .add_leader_text(
            Anchor {
                node: Some(NodeId::Object(id)),
                point: Point3::ORIGIN,
            },
            Vec3::new(1., 1., 1.),
            "corner".into(),
        )
        .unwrap();
    let free = d
        .add_leader_text(
            Anchor {
                node: None,
                point: Point3::ORIGIN,
            },
            Vec3::new(1., 1., 1.),
            "free".into(),
        )
        .unwrap();
    let before = d.save();
    let c = candidate(&d, id);
    assert_eq!(
        c.annotations_to_detach().collect::<Vec<_>>(),
        vec![attached]
    );
    d.commit_edge_treatment(c).unwrap();
    assert_eq!(d.annotation_detached(attached), Some(true));
    assert_eq!(d.annotation_detached(free), Some(false));
    let after = d.save();
    d.undo().unwrap();
    assert_eq!(d.save(), before);
    d.redo().unwrap();
    assert_eq!(d.save(), after);
    assert_eq!(Document::load(&after).unwrap().save(), after);
}
#[test]
fn grouped_metadata_material_and_persistence() {
    let mut d = Document::new();
    let id = block(&mut d, 0.);
    let group = d.group_nodes(&[NodeId::Object(id)]).unwrap().0;
    d.set_node_name(NodeId::Object(id), Some("part".into()))
        .unwrap();
    d.add_node_tag(NodeId::Object(id), vec!["parts".into()])
        .unwrap();
    let target = AttrTarget::Entity(EntityRef::Object(id));
    d.attr_set(
        target.clone(),
        "test",
        "key",
        AttrValue::Text("value".into()),
    )
    .unwrap();
    let attrs = d.attr_get(&target).unwrap().cloned();
    let sid = d.sid_of(&EntityRef::Object(id));
    let material = d.add_material(Material::solid(
        "red",
        Rgba8 {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        },
    ));
    d.set_object_material(id, Some(material)).unwrap();
    let blue = d.add_material(Material::solid(
        "blue",
        Rgba8 {
            r: 0,
            g: 0,
            b: 255,
            a: 255,
        },
    ));
    let top = d
        .object(id)
        .unwrap()
        .faces()
        .iter()
        .find(|(_, f)| f.plane.normal().z > 0.9)
        .unwrap()
        .0;
    d.paint_face(id, top, Some(blue)).unwrap();
    let default_material = d.object(id).unwrap().default_material();
    let before = d.save();
    d.roundover_edge(id, edge(&d, id), 0.25, 0.005).unwrap();
    assert_eq!(d.object(id).unwrap().default_material(), default_material);
    let top_after = d
        .object(id)
        .unwrap()
        .faces()
        .iter()
        .find(|(_, f)| f.plane.normal().z > 0.9)
        .unwrap()
        .0;
    assert_eq!(d.face_material(id, top_after), Some(blue));
    assert_eq!(d.node_parent(NodeId::Object(id)), Some(group));
    assert_eq!(d.attr_get(&target).unwrap().cloned(), attrs);
    assert_eq!(d.sid_of(&EntityRef::Object(id)), sid);
    let after = d.save();
    let loaded = Document::load(&after).unwrap();
    assert_eq!(loaded.save(), after);
    assert_eq!(loaded.undo_depth(), 0);
    d.undo().unwrap();
    assert_eq!(d.save(), before);
    d.redo().unwrap();
    assert_eq!(d.save(), after);
}
proptest! {
 #![proptest_config(ProptestConfig::with_cases(24))]
 #[test]
 fn parameterized_history(distance in 0.05f64..0.7,rounded in any::<bool>()) {
 let mut d=Document::new();let id=block(&mut d,0.);let before=d.save();if rounded {d.roundover_edge(id,edge(&d,id),distance,0.01).unwrap();}else{d.chamfer_edge(id,edge(&d,id),distance).unwrap();}let after=d.save();for _ in 0..3 {d.undo().unwrap();prop_assert_eq!(d.save(),before.clone());d.redo().unwrap();prop_assert_eq!(d.save(),after.clone());}prop_assert_eq!(Document::load(&after).unwrap().save(),after);
 }
}

#[test]
fn existing_object_history_survives_snapshot_actions() {
    let mut d = Document::new();
    let id = block(&mut d, 0.);
    let original = d.save();
    let top = |d: &Document| {
        d.object(id)
            .unwrap()
            .faces()
            .iter()
            .find(|(_, f)| f.plane.normal().z > 0.9)
            .unwrap()
            .0
    };
    d.apply_object_op(
        id,
        KernelOp::PushPull {
            face: top(&d),
            distance: 0.2,
        },
    )
    .unwrap();
    let pushed = d.save();
    d.chamfer_edge(id, edge(&d, id), 0.25).unwrap();
    let treated = d.save();
    d.apply_object_op(
        id,
        KernelOp::PushPull {
            face: top(&d),
            distance: 0.1,
        },
    )
    .unwrap();
    let final_state = d.save();
    for _ in 0..3 {
        d.undo().unwrap();
        assert_eq!(d.save(), treated);
        d.undo().unwrap();
        assert_eq!(d.save(), pushed);
        d.undo().unwrap();
        assert_eq!(d.save(), original);
        d.redo().unwrap();
        assert_eq!(d.save(), pushed);
        d.redo().unwrap();
        assert_eq!(d.save(), treated);
        d.redo().unwrap();
        assert_eq!(d.save(), final_state);
    }
}
#[test]
fn definition_members_and_hidden_tags_refuse() {
    let mut d = Document::new();
    let id = block(&mut d, 0.);
    d.add_node_tag(NodeId::Object(id), vec!["parts".into()])
        .unwrap();
    let c = candidate(&d, id);
    d.set_tag_hidden(vec!["parts".into()], true);
    stale(&mut d, c);
    assert!(d.prepare_edge_treatment(id, edge(&d, id), op()).is_err());
    d.set_tag_hidden(vec!["parts".into()], false);
    d.make_component(&[NodeId::Object(id)]).unwrap();
    let before = d.save();
    assert_eq!(
        d.prepare_edge_treatment(id, edge(&d, id), op())
            .unwrap_err(),
        DocumentEdgeTreatmentError::UnsupportedContext
    );
    assert_eq!(d.save(), before);
}
