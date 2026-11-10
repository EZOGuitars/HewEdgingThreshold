import { createRequire } from 'node:module'
import { describe, it, expect, vi } from 'vitest'
import * as THREE from 'three'
import { EdgeTreatmentTool, type TreatmentState } from './EdgeTreatmentTool'
import type { Scene } from '../wasm/loader'
import { setLengthUnit } from '../settings/units'
import { inspectClosedStl, stlVertexSet } from '../../e2e/stlIntegrity'
const binding = new URL('../../../tools/replay-runner/pkg-node/wasm_api.js', import.meta.url)
// Mandatory: a missing generated module fails this acceptance suite.
const { Scene: NativeScene } = createRequire(import.meta.url)(binding.pathname)
const shapes = [
  { name: 'triangle', points: [[0,0],[0.1,0],[0.03,0.07]] },
  { name: 'asymmetric trapezoid', points: [[0,0],[0.1,0],[0.07,0.06],[0.01,0.06]] },
  { name: 'rectangle', points: [[0,0],[0.1,0],[0.1,0.06],[0,0.06]] },
  ...[6,8,11].map(n => ({ name: `${n}-gon`, points: Array.from({length:n},(_,i)=>{const t=2*Math.PI*i/n; return [0.05*Math.cos(t),0.04*Math.sin(t)]}) })),
  ...[6,8].map(n => ({ name: `regular ${n}-gon`, points: Array.from({length:n},(_,i)=>{const t=2*Math.PI*i/n; return [0.05*Math.cos(t),0.05*Math.sin(t)]}) })),
  { name: 'irregular hexagon', points: [[0,0],[0.08,0],[0.11,0.02],[0.09,0.06],[0.03,0.08],[-0.01,0.04]] },
]
function expectedRemoval(points: number[][], radius: number, segments: number): number {
  const directions = points.map((p,i) => { const q=points[(i+1)%points.length], length=Math.hypot(q[0]-p[0],q[1]-p[1]); return [(q[0]-p[0])/length,(q[1]-p[1])/length,length] })
  const perimeter=directions.reduce((s,d)=>s+d[2],0)
  const corners=directions.reduce((sum,b,i)=>{const a=directions[(i+directions.length-1)%directions.length];return sum+(a[0]*b[1]-a[1]*b[0])/(1+a[0]*b[0]+a[1]*b[1])},0)
  let removed=0
  for(let i=0;i<segments;i++) {
    const t=i*Math.PI/2/segments, u=(i+1)*Math.PI/2/segments
    const a=radius*(1-Math.sin(t)), b=radius*(1-Math.sin(u))
    removed+=radius*(Math.cos(t)-Math.cos(u))*(perimeter*(a+b)/2-corners*(a*a+a*b+b*b)/3)
  }
  return removed*1e9
}

describe('generalized cap bands through actual WASM and the tool', () => {
  it.each(shapes)('$name: independent STL, exact dimensions, sequential history and replay', ({points}) => {
    setLengthUnit('mm')
    const scene: Scene = new NativeScene(), replay: Scene = new NativeScene(), loaded: Scene = new NativeScene()
    let state: TreatmentState | null = null
    const tool = new EdgeTreatmentTool(scene,new THREE.Group(),vi.fn(),s=>{state=s},vi.fn())
    const current = () => state as unknown as TreatmentState
    try {
      scene.start_recording()
      const sketch=scene.begin_ground_sketch(); scene.sketch_begin_gesture(sketch)
      points.forEach((p,i)=>{const q=points[(i+1)%points.length];scene.sketch_add_segment(sketch,p[0],p[1],0,q[0],q[1],0)})
      scene.sketch_end_gesture(sketch)
      const object=scene.extrude_region(sketch,scene.sketch_regions(sketch)[0],0.04)
      const center=points.reduce((a,p)=>[a[0]+p[0]/points.length,a[1]+p[1]/points.length],[0,0])
      const original=scene.save(), originalStl=inspectClosedStl(scene.export('stl',0)!)
      tool.activate()
      for(const upper of [true,false]) {
        const hit=scene.pick_face(center[0],center[1],upper?1:-1,0,0,upper?-1:1)!
        try {
          tool.configure({mode:'roundover',radius:upper?'3 mm':'0.0125"',chord:upper?'0.02 mm':'0.0039"'})
          tool.selectFacePerimeter()
          tool.onPointerDown({x:center[0],y:center[1],z:upper?0.04:0,kind:'on-face',object,element:hit.face(),elementKind:'face'}, {origin:[0,0,1],direction:[0,0,-1]})
        } finally {hit.free()}
        const before=scene.save(),depth=scene.undo_depth()
        const beforeBytes=scene.export('stl',0)!, beforeMesh=inspectClosedStl(beforeBytes)
        tool.generate()
        expect(current().phase,current().error ?? '').toBe('ready')
        expect(current().limits?.certainty).toBe('conservative')
        expect(current().info!.operation.radius).toBe(upper?0.003:0.0125*0.0254)
        expect(scene.save()).toEqual(before); expect(scene.undo_depth()).toBe(depth)
        const expected=expectedRemoval(points,upper?0.003:0.0125*0.0254,current().info!.segments!)
        tool.apply(); expect(scene.undo_depth()).toBe(depth+1)
        const after=scene.save(), stl=inspectClosedStl(scene.export('stl',0)!)
        expect(stl.volumeMm3).toBeLessThan(originalStl.volumeMm3)
        // Independently decode the exported carrier; do not equate parsed input
        // values or native success with dimensional/volumetric accuracy.
        expect(Math.abs((beforeMesh.volumeMm3-stl.volumeMm3)-expected)).toBeLessThan(originalStl.volumeMm3*2e-6)
        const afterVertices=new Set(stlVertexSet(scene.export('stl',0)!))
        for(const key of stlVertexSet(beforeBytes)) {
          const z=Number(key.split(',')[2])
          if(upper ? z < 36.99 : z > 0.318) expect(afterVertices.has(key)).toBe(true)
        }
        expect(Math.abs(stl.max[2]-40)).toBeLessThan(0.00002)
        expect(Math.abs(stl.min[2])).toBeLessThan(0.00002)
        scene.scene_undo().free(); expect(scene.save()).toEqual(before)
        scene.scene_redo().free(); expect(scene.save()).toEqual(after)
      }
      const final=scene.save(), stl=scene.export('stl',0)!
      loaded.load(final); expect(loaded.save()).toEqual(final)
      expect(stlVertexSet(loaded.export('stl',0)!)).toEqual(stlVertexSet(stl))
      inspectClosedStl(loaded.export('stl',0)!)
      scene.stop_recording();const recording=scene.take_recording()
      replay.replay(recording);expect(replay.save()).toEqual(final)
      expect(final).not.toEqual(original)
    } finally {tool.cancel(); scene.free(); replay.free(); loaded.free()}
  })
})

it('refuses an attributed circular cap without changing the model or history', () => {
  const scene: Scene = new NativeScene()
  try {
    const sketch=scene.begin_ground_sketch()
    scene.sketch_begin_gesture(sketch)
    scene.sketch_begin_curve_with(sketch,0,0,0,0.05)
    for(let i=0;i<24;i++) {
      const t=2*Math.PI*i/24,u=2*Math.PI*(i+1)/24
      scene.sketch_add_segment(sketch,0.05*Math.cos(t),0.05*Math.sin(t),0,0.05*Math.cos(u),0.05*Math.sin(u),0)
    }
    scene.sketch_end_curve(sketch);scene.sketch_end_gesture(sketch)
    const object=scene.extrude_region(sketch,scene.sketch_regions(sketch)[0],0.04)
    const hit=scene.pick_face(0,0,1,0,0,-1)!
    const edges=scene.edge_treatment_face_edges(object,hit.face());hit.free()
    const before=scene.save(),depth=scene.undo_depth()
    let refusal=''
    try {scene.prepare_roundover_edges(object,edges,0.003,0.00002)} catch(error) {refusal=String(error)}
    expect(refusal).toMatch(/^SourceResolution:/)
    expect(scene.save()).toEqual(before);expect(scene.undo_depth()).toBe(depth)
  } finally {scene.free()}
})
