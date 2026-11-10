import { createRequire } from 'node:module'
import { it, expect, vi } from 'vitest'
import * as THREE from 'three'
import { EdgeTreatmentTool, type TreatmentState } from './EdgeTreatmentTool'
import type { Scene } from '../wasm/loader'
import { setLengthUnit } from '../settings/units'
import { inspectClosedStl, stlVertexSet } from '../../e2e/stlIntegrity'
const { Scene: NativeScene } = createRequire(import.meta.url)(new URL('../../../tools/replay-runner/pkg-node/wasm_api.js', import.meta.url).pathname)
const radius=0.05, height=0.04, sides=64, chord=0.0003
function removal(size:number,segments:number) {
  const area=sides/2*radius*radius*Math.sin(2*Math.PI/sides)
  const samples=segments ? Array.from({length:segments+1},(_,i)=>{const t=Math.PI*i/2/segments;return [radius-size+size*Math.sin(t),size*(1-Math.cos(t))]}) : [[radius-size,0],[radius,size]]
  return samples.slice(1).reduce((sum,b,i)=>{const a=samples[i];return sum+area*(b[1]-a[1])*(1-(a[0]*a[0]+a[0]*b[0]+b[0]*b[0])/(3*radius*radius))},0)*1e9
}
it.each([
  {mode:'chamfer' as const,firstUpper:true},{mode:'roundover' as const,firstUpper:true},
  {mode:'chamfer' as const,firstUpper:false},{mode:'roundover' as const,firstUpper:false},
])('cylinder $mode firstUpper=$firstUpper: STL, selection, persistence and replay', ({mode,firstUpper}) => {
  setLengthUnit('mm')
  const scene:Scene=new NativeScene(), loaded:Scene=new NativeScene(), replay:Scene=new NativeScene()
  let state:TreatmentState|null=null
  const tool=new EdgeTreatmentTool(scene,new THREE.Group(),vi.fn(),s=>{state=s},vi.fn())
  const current=()=>state as unknown as TreatmentState
  try {
    scene.start_recording()
    const sketch=scene.begin_ground_sketch();scene.sketch_begin_gesture(sketch);scene.sketch_begin_curve_with(sketch,0,0,0,radius)
    for(let i=0;i<sides;i++){const a=2*Math.PI*i/sides,b=2*Math.PI*(i+1)/sides;scene.sketch_add_segment(sketch,radius*Math.cos(a),radius*Math.sin(a),0,radius*Math.cos(b),radius*Math.sin(b),0)}
    scene.sketch_end_curve(sketch);scene.sketch_end_gesture(sketch)
    const object=scene.extrude_region(sketch,scene.sketch_regions(sketch)[0],height)
    tool.activate()
    for(const [index,upper] of [firstUpper,!firstUpper].entries()) {
      const size=index===0?0.003:0.002, operation=index===0?mode:'roundover'
      tool.configure({mode:operation,distance:`${size*1000} mm`,radius:`${size*1000} mm`,chord:'0.3 mm'})
      const hit=scene.pick_face(0,0,upper?1:-1,0,0,upper?-1:1)!
      tool.selectFacePerimeter();tool.onPointerDown({x:0,y:0,z:upper?height:0,kind:'on-face',object,element:hit.face(),elementKind:'face'},{origin:[0,0,1],direction:[0,0,-1]});hit.free()
      const before=scene.save(), depth=scene.undo_depth(), beforeMesh=inspectClosedStl(scene.export('stl',0)!)
      tool.generate();expect(current().phase,current().error??'').toBe('ready')
      expect(current().limits?.certainty).toBe('conservative')
      expect(scene.save()).toEqual(before);expect(scene.undo_depth()).toBe(depth)
      const expected=removal(size,operation==='roundover'?current().info!.segments!:0)
      tool.apply();const after=scene.save(),bytes=scene.export('stl',0)!,mesh=inspectClosedStl(bytes)
      expect(Math.abs(beforeMesh.volumeMm3-mesh.volumeMm3-expected)).toBeLessThan(beforeMesh.volumeMm3*2e-6)
      // UI export defaults to a nonzero curve resolution. Trimmed cylinder
      // bands must fall back to stored facets rather than refit old walls
      // independently from their new transition seams.
      for(const resolution of [8,48,128]) {
        const exported=scene.export('stl',resolution)!
        inspectClosedStl(exported)
        expect(stlVertexSet(exported)).toEqual(stlVertexSet(bytes))
      }
      const ring=stlVertexSet(bytes).map(s=>s.split(',').map(Number)).filter(p=>Math.abs(p[2]-(upper?40:0))<1e-5)
      const actualRadius=Math.max(...ring.map(p=>Math.hypot(p[0],p[1])))
      expect(Math.abs(actualRadius-(radius-size)*1000)).toBeLessThan(0.00002)
      // Independent samples are supplementary to the native whole-triangle certificate.
      if(operation==='roundover') {
        const view=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength);let maximum=0
        for(let i=0;i<view.getUint32(80,true);i++) {
          const at=84+50*i,nz=view.getFloat32(at+8,true)
          if(Math.abs(nz)<1e-5||Math.abs(nz)>.99999) continue
          const p=Array.from({length:3},(_,j)=>Array.from({length:3},(_,k)=>view.getFloat32(at+12+12*j+4*k,true)/1000))
          if(!p.every(v=>upper?v[2]>=height-size-1e-8:v[2]<=size+1e-8)) continue
          for(let a=0;a<=4;a++)for(let b=0;b<=4-a;b++){
            const q=[0,1,2].map(k=>(a*p[0][k]+b*p[1][k]+(4-a-b)*p[2][k])/4)
            const x=Math.hypot(q[0],q[1])-(radius-size), y=(q[2]-(upper?height-size:size))*(upper?1:-1)
            const error=x<0?Math.hypot(x,y-size):y<0?Math.hypot(x-size,y):Math.abs(Math.hypot(x,y)-size)
            maximum=Math.max(maximum,error)
          }
        }
        expect(maximum).toBeLessThanOrEqual(chord)
      }
      scene.scene_undo().free();expect(scene.save()).toEqual(before);scene.scene_redo().free();expect(scene.save()).toEqual(after)
    }
    scene.stop_recording();const recording=scene.take_recording(),final=scene.save()
    loaded.load(final);expect(loaded.save()).toEqual(final);inspectClosedStl(loaded.export('stl',0)!)
    replay.replay(recording);expect(replay.save()).toEqual(final)
  }finally{tool.cancel();scene.free();loaded.free();replay.free()}
})
