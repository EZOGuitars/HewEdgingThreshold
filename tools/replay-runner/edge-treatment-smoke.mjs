// Actual generated wasm-bindgen bindings, not a mock/native substitute.
// Build pkg-node first as described in README.md; run with node --test.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { loadWasm } from './lib.mjs';
const { Scene } = loadWasm();
function box(s) {
  const sketch = s.begin_ground_sketch();
  s.sketch_begin_gesture(sketch);
  const p = [
    [0, 0],
    [2, 0],
    [2, 3],
    [0, 3],
  ];
  for (let i = 0; i < 4; i++) s.sketch_add_segment(sketch, ...p[i], 0, ...p[(i + 1) % 4], 0);
  s.sketch_end_gesture(sketch);
  const object = s.extrude_region(sketch, s.sketch_regions(sketch)[0], 4);
  return { object, edge: pick(s, object) };
}
function pick(s, object) {
  const snap = s.snap(3, -1, 1.5, -Math.SQRT1_2, Math.SQRT1_2, 0, 0.01);
  assert.ok(snap, 'existing inference must resolve the edge');
  try {
    assert.equal(snap.element_kind(), 'edge');
    assert.equal(snap.object(), object);
    return snap.element();
  } finally {
    snap.free();
  }
}
const code = (f, c) => assert.throws(f, (e) => String(e).startsWith(`${c}:`));
for (const mode of ['chamfer', 'roundover'])
  test(`real WASM: ${mode}, mesh ownership, commit, history, persistence, replay`, () => {
    const s = new Scene();
    const replay = new Scene();
    const loaded = new Scene();
    try {
      s.start_recording();
      const { object, edge } = box(s);
      assert.equal(typeof object, 'bigint');
      assert.equal(typeof edge, 'bigint');
      const before = s.save();
      const depth = s.undo_depth();
      const h =
        mode === 'chamfer'
          ? s.prepare_chamfer(object, edge, 0.25)
          : s.prepare_roundover(object, edge, 0.25, 0.005);
      assert.equal(typeof h, 'number');
      assert.ok(Number.isInteger(h));
      const info = JSON.parse(s.edge_treatment_info_json(h));
      assert.equal(info.object, String(object));
      assert.equal(info.edge, String(edge));
      assert.equal(info.operation.kind, mode);
      assert.deepEqual(info.warnings, []);
      const mesh = s.edge_treatment_mesh(h);
      let positions;
      try {
        positions = mesh.positions();
        assert.ok(positions instanceof Float32Array);
        assert.ok(positions.length > 0);
        const copy = mesh.positions();
        copy[0] = 12345;
        assert.notEqual(mesh.positions()[0], 12345);
      } finally {
        mesh.free();
        assert.equal(mesh.__wbg_ptr, 0);
      }
      assert.deepEqual(s.save(), before);
      assert.equal(s.undo_depth(), depth);
      const change = s.commit_edge_treatment(h);
      try {
        assert.deepEqual([...change.objects_touched()], [object]);
      } finally {
        change.free();
      }
      assert.equal(s.undo_depth(), depth + 1);
      assert.equal(s.object_ids().length, 1);
      const actual = s.object_mesh(object);
      try {
        assert.deepEqual(actual.positions(), positions);
      } finally {
        actual.free();
      }
      code(() => s.commit_edge_treatment(h), 'InvalidCandidateHandle');
      const after = s.save();
      s.scene_undo().free();
      assert.deepEqual(s.save(), before);
      s.scene_redo().free();
      assert.deepEqual(s.save(), after);
      loaded.load(after);
      assert.deepEqual(loaded.save(), after);
      s.stop_recording();
      const recording = s.take_recording();
      assert.equal(
        JSON.parse(recording).calls.filter((c) => c.method === 'edge_treatment').length,
        1,
      );
      assert.equal(replay.replay(recording), s.state_hash());
      assert.deepEqual(replay.save(), after);
    } finally {
      s.stop_recording();
      s.free();
      replay.free();
      loaded.free();
    }
  });
test('real WASM: discard, stale failure, errors, replacement and foreign handles', () => {
  const s = new Scene();
  const other = new Scene();
  try {
    const { object, edge } = box(s);
    const b = box(other);
    const before = s.save();
    const depth = s.undo_depth();
    code(() => s.prepare_chamfer(object, edge, 0), 'InvalidDistance');
    code(() => s.prepare_roundover(object, edge, 0.2, 0), 'InvalidChordError');
    code(() => s.prepare_roundover(object, edge, -1, 0.01), 'InvalidRadius');
    code(() => s.prepare_chamfer(object, 0n, 0.2), 'UnknownEdge');
    const h = s.prepare_chamfer(object, edge, 0.2);
    const foreign = other.prepare_chamfer(b.object, b.edge, 0.2);
    code(() => other.commit_edge_treatment(h), 'InvalidCandidateHandle');
    other.discard_edge_treatment(foreign);
    s.discard_edge_treatment(h);
    code(() => s.discard_edge_treatment(h), 'InvalidCandidateHandle');
    assert.deepEqual(s.save(), before);
    assert.equal(s.undo_depth(), depth);
    const stale = s.prepare_chamfer(object, edge, 0.2);
    s.begin_ground_sketch();
    const edited = s.save();
    const editedDepth = s.undo_depth();
    code(() => s.commit_edge_treatment(stale), 'StaleCandidate');
    assert.deepEqual(s.save(), edited);
    assert.equal(s.undo_depth(), editedDepth);
    code(() => s.edge_treatment_mesh(stale), 'InvalidCandidateHandle');
    const next = s.prepare_chamfer(object, edge, 0.2);
    s.load(before);
    code(() => s.commit_edge_treatment(next), 'InvalidCandidateHandle');
  } finally {
    s.free();
    other.free();
  }
});
test('real WASM: Scene disposal retires candidates; previews record nothing', () => {
  const s = new Scene();
  let old;
  try {
    const { object, edge } = box(s);
    s.start_recording();
    old = s.prepare_chamfer(object, edge, 0.2);
    s.edge_treatment_mesh(old).free();
    s.discard_edge_treatment(old);
    s.stop_recording();
    assert.deepEqual(JSON.parse(s.take_recording()).calls, []);
    old = s.prepare_chamfer(object, edge, 0.2);
  } finally {
    s.stop_recording();
    s.free();
    assert.equal(s.__wbg_ptr, 0);
  }
  const next = new Scene();
  try {
    box(next);
    code(() => next.commit_edge_treatment(old), 'InvalidCandidateHandle');
  } finally {
    next.free();
  }
});
