import { createRequire } from 'node:module'
import { existsSync } from 'node:fs'
import { describe, it, expect, vi } from 'vitest'
import * as THREE from 'three'
import { EdgeTreatmentTool, type TreatmentState } from './EdgeTreatmentTool'
import type { Scene } from '../wasm/loader'
import { inspectClosedStl, stlVertexSet } from '../../e2e/stlIntegrity'
const path = new URL('../../../tools/replay-runner/pkg-node/wasm_api.js', import.meta.url)

describe.skipIf(!existsSync(path))('connected treatments through actual WASM', () => {
  it.each(['chamfer', 'roundover'] as const)('%s: all junction selections, STL, history and replay', mode => {
    const { Scene: NativeScene } = createRequire(import.meta.url)(path.pathname)
    for (const selection of ['two', 'three', 'perimeter', 'disconnected', 'all'] as const) {
      const scene = new NativeScene(), replay = new NativeScene(), loaded = new NativeScene()
      let state: TreatmentState | null = null
      const committed = vi.fn(), group = new THREE.Group()
      const tool = new EdgeTreatmentTool(scene as Scene, group, committed, s => { state = s }, vi.fn())
      const current = () => state as unknown as TreatmentState
      try {
        scene.start_recording()
        const sketch = scene.begin_ground_sketch()
        scene.sketch_begin_gesture(sketch)
        const p = [[0, 0], [.3, 0], [.3, .2], [0, .2]]
        for (let i = 0; i < 4; i++) scene.sketch_add_segment(sketch, ...p[i], 0, ...p[(i+1)%4], 0)
        scene.sketch_end_gesture(sketch)
        const object = scene.extrude_region(sketch, scene.sketch_regions(sketch)[0], .1)
        const rays = [[.15,.1,1,0,0,-1], [.15,.1,-1,0,0,1], [-1,.1,.05,1,0,0], [1,.1,.05,-1,0,0], [.15,-1,.05,0,1,0], [.15,1,.05,0,-1,0]]
        const perimeters: bigint[][] = rays.map(ray => {
          const hit = scene.pick_face(...ray)
          try { return Array.from(scene.edge_treatment_face_edges(object, hit.face())) as bigint[] }
          finally { hit.free() }
        })
        const top = perimeters[0]
        const all = [...new Set(perimeters.flat())]
        const origin = all.filter(edge => {
          const a = scene.edge_endpoints(object, edge)
          return (a[0]===0 && a[1]===0 && a[2]===0) || (a[3]===0 && a[4]===0 && a[5]===0)
        })
        const selected = selection === 'two' ? origin.slice(0,2) : selection === 'three' ? origin : selection === 'perimeter' ? top : selection === 'disconnected' ? [top[0],top[2]] : all
        expect(selected.length).toBe(selection === 'all' ? 12 : selection === 'three' ? 3 : selection === 'perimeter' ? 4 : 2)
        if (mode === 'roundover' && selection === 'all') {
          const handles = BigUint64Array.from(selected)
          const coarse = scene.prepare_roundover_edges(object, handles, .00635, .0001)
          const coarseCount = JSON.parse(scene.edge_treatment_info_json(coarse)).segments
          const fine = scene.prepare_roundover_edges(object, handles, .00635, .000025)
          expect(JSON.parse(scene.edge_treatment_info_json(fine)).segments).toBeGreaterThan(coarseCount)
          expect(() => scene.prepare_roundover_edges(object, handles, .00635, 1e-7)).toThrow()
          scene.discard_edge_treatment(fine)
        }
        const before = scene.save(), depth = scene.undo_depth()
        const beforeStl = inspectClosedStl(scene.export('stl', 0))
        tool.activate()
        tool.configure({ mode, distance: '6.35 mm', radius: '6.35 mm', chord: '0.05 mm' })
        for (const edge of selected) {
          tool.setSelectionModifier(true)
          tool.onPointerDown({ x: 0, y: 0, z: 0, kind: 'on-edge', object, element: edge, elementKind: 'edge' }, { origin: [0,0,1], direction: [0,0,-1] })
        }
        tool.generate()
        expect(current().error).toBeNull()
        expect(current().phase).toBe('ready')
        expect(scene.save()).toEqual(before)
        expect(scene.undo_depth()).toBe(depth)
        const candidate = scene.edge_treatment_mesh(current().info!.handle)
        try { expect(candidate.watertight()).toBe(true) } finally { candidate.free() }
        // Cancel one valid preview, then regenerate the identical selection.
        tool.clearSelection()
        expect(scene.save()).toEqual(before)
        for (const edge of selected) tool.onPointerDown({ x:0,y:0,z:0,kind:'on-edge',object,element:edge,elementKind:'edge' }, {origin:[0,0,1],direction:[0,0,-1]})
        tool.generate()
        expect(current().phase).toBe('ready')
        tool.apply()
        expect(committed).toHaveBeenCalledOnce()
        expect(scene.undo_depth()).toBe(depth+1)
        const after = scene.save()
        const stl = scene.export('stl', 0)
        const measured = inspectClosedStl(stl)
        expect(measured.volumeMm3).toBeLessThan(beforeStl.volumeMm3)
        expect(measured.min.every((x,i) => x >= beforeStl.min[i] - .00001)).toBe(true)
        expect(measured.max.every((x,i) => x <= beforeStl.max[i] + .00001)).toBe(true)
        scene.scene_undo().free(); expect(scene.save()).toEqual(before)
        scene.scene_redo().free(); expect(scene.save()).toEqual(after)
        loaded.load(after); expect(loaded.save()).toEqual(after)
        // Persistence preserves exact polygons, but reconstructed loop starts can
        // choose different diagonals. Compare physical coordinates and closed
        // meshes, not incidental triangle record order or diagonals.
        const restoredStl = loaded.export('stl',0)
        expect(stlVertexSet(restoredStl)).toEqual(stlVertexSet(stl))
        expect(Math.abs(inspectClosedStl(restoredStl).volumeMm3 - measured.volumeMm3)).toBeLessThan(beforeStl.volumeMm3 * 1e-6)
        scene.stop_recording()
        const recording = scene.take_recording()
        expect(JSON.parse(recording).calls.filter((c: {method: string}) => c.method === 'edge_treatment_batch')).toHaveLength(1)
        expect(replay.replay(recording)).toBe(scene.state_hash())
        expect(replay.save()).toEqual(after)
        expect(stlVertexSet(replay.export('stl',0))).toEqual(stlVertexSet(stl))
        inspectClosedStl(replay.export('stl',0))
      } finally { tool.cancel(); scene.free(); replay.free(); loaded.free() }
    }
  }, 60_000)
})
