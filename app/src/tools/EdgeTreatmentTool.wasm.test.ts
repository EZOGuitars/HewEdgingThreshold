import { createRequire } from 'node:module'
import { existsSync } from 'node:fs'
import { describe, it, expect, vi } from 'vitest'
import * as THREE from 'three'
import { EdgeTreatmentTool, type TreatmentState } from './EdgeTreatmentTool'
import type { Scene } from '../wasm/loader'
const path = new URL('../../../tools/replay-runner/pkg-node/wasm_api.js', import.meta.url)
// The dedicated Node smoke is mandatory before UI work. This additional tool
// integration suite runs whenever its generated Node package is available.
describe.skipIf(!existsSync(path))('Edge Treatment tool with actual WASM', () => {
  it.each(['chamfer', 'roundover'] as const)(
    '%s: actual preview, Apply, undo/redo, persistence and replay',
    (mode) => {
      const { Scene: NativeScene } = createRequire(import.meta.url)(path.pathname)
      const scene = new NativeScene()
      const replay = new NativeScene()
      const loaded = new NativeScene()
      let state: TreatmentState | null = null
      const group = new THREE.Group()
      const committed = vi.fn()
      const tool = new EdgeTreatmentTool(
        scene as Scene,
        group,
        committed,
        (s) => {
          state = s
        },
        vi.fn(),
      )
      try {
        scene.start_recording()
        const sketch = scene.begin_ground_sketch()
        scene.sketch_begin_gesture(sketch)
        const p = [
          [0, 0],
          [2, 0],
          [2, 3],
          [0, 3],
        ]
        for (let i = 0; i < 4; i++)
          scene.sketch_add_segment(sketch, ...p[i], 0, ...p[(i + 1) % 4], 0)
        scene.sketch_end_gesture(sketch)
        const object = scene.extrude_region(sketch, scene.sketch_regions(sketch)[0], 4)
        const picked = scene.snap(3, -1, 1.5, -Math.SQRT1_2, Math.SQRT1_2, 0, 0.01)
        const edge = picked.element()
        picked.free()
        const annotation = scene.add_leader_text(
          0,
          object,
          new Float64Array([2, 0, 1.5]),
          new Float64Array([0.1, 0.1, 0.1]),
          'corner',
        )
        tool.activate()
        tool.configure({ mode, distance: '250 mm', radius: '250 mm', chord: '5 mm' })
        const before = scene.save(),
          depth = scene.undo_depth()
        tool.onPointerDown(
          { x: 2, y: 0, z: 1.5, kind: 'on-edge', object, element: edge, elementKind: 'edge' },
          { origin: [3, -1, 1.5], direction: [-Math.SQRT1_2, Math.SQRT1_2, 0] },
        )
        tool.generate()
        expect((state as unknown as TreatmentState).phase).toBe('ready')
        expect(scene.save()).toEqual(before)
        expect(scene.undo_depth()).toBe(depth)
        const rendered = group.children[0].children[0].children[0] as THREE.Mesh
        const handle = (state as unknown as TreatmentState).info!.handle
        const mesh = scene.edge_treatment_mesh(handle)
        try {
          expect(rendered.geometry.getAttribute('position').array).toEqual(mesh.positions())
        } finally {
          mesh.free()
        }
        expect(
          (state as unknown as TreatmentState).info!.warnings[0].annotations[0].annotation,
        ).toBe(String(annotation))
        tool.apply()
        expect(committed).not.toHaveBeenCalled()
        tool.acknowledge(true)
        tool.apply()
        expect(scene.annotation_detached(annotation)).toBe(true)
        expect(committed).toHaveBeenCalledOnce()
        expect(scene.undo_depth()).toBe(depth + 1)
        expect((state as unknown as TreatmentState).target).toBeNull()
        const after = scene.save()
        scene.scene_undo().free()
        expect(scene.annotation_detached(annotation)).toBe(false)
        expect(scene.save()).toEqual(before)
        scene.scene_redo().free()
        expect(scene.save()).toEqual(after)
        loaded.load(after)
        expect(loaded.save()).toEqual(after)
        scene.stop_recording()
        const recording = scene.take_recording()
        expect(
          JSON.parse(recording).calls.filter(
            (c: { method: string }) => c.method === 'edge_treatment',
          ),
        ).toHaveLength(1)
        expect(replay.replay(recording)).toBe(scene.state_hash())
        expect(replay.save()).toEqual(after)
      } finally {
        tool.cancel()
        scene.stop_recording()
        scene.free()
        replay.free()
        loaded.free()
      }
    },
  )
})
