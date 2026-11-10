// Portable release-WASM benchmark; no timing thresholds or document writes.
import assert from 'node:assert/strict';
import { performance } from 'node:perf_hooks';
import { loadWasm } from './lib.mjs';
const { Scene } = loadWasm();
function make(kind,n=64){const s=new Scene(),k=s.begin_ground_sketch();s.sketch_begin_gesture(k);let p;
 if(kind==='circle'){s.sketch_begin_curve_with(k,0,0,0,.05);p=Array.from({length:n},(_,i)=>{const t=2*Math.PI*i/n;return[.05*Math.cos(t),.05*Math.sin(t)]})}
 else if(kind==='trapezoid')p=[[0,0],[.1,0],[.07,.06],[.01,.06]];
 else p=[[0,0],[.1,0],[.1,.06],[0,.06]];
 p.forEach((a,i)=>{const b=p[(i+1)%p.length];s.sketch_add_segment(k,...a,0,...b,0)});if(kind==='circle')s.sketch_end_curve(k);s.sketch_end_gesture(k);const object=s.extrude_region(k,s.sketch_regions(k)[0],.04);
 function edges(upper=true){const hit=s.pick_face(kind==='circle'?0:.04,kind==='circle'?0:.02,upper?1:-1,0,0,upper?-1:1);assert.ok(hit);try{return s.edge_treatment_face_edges(object,hit.face())}finally{hit.free()}}
 return{s,object,edges};}
const results=[];
for(const c of[{name:'single edge',kind:'box',single:true,r:.002,e:.00005},{name:'trapezoid perimeter',kind:'trapezoid',r:.003,e:.00002},{name:'cylinder 64 perimeter',kind:'circle',n:64,r:.003,e:.0003},{name:'cylinder 128 fine perimeter',kind:'circle',n:128,r:.003,e:.0001},{name:'sequential cylinder opposite cap',kind:'circle',n:64,r:.002,e:.0003,sequential:true}]){
 const f=make(c.kind,c.n);try{if(c.sequential){const h=f.s.prepare_roundover_edges(f.object,f.edges(),.003,.0003);f.s.commit_edge_treatment(h).free()}
 const edges=c.single?f.edges().slice(0,1):f.edges(!c.sequential),source=f.s.save(),depth=f.s.undo_depth(),times=[];let segments,triangles;
 for(let i=0;i<5;i++){const t=performance.now(),h=f.s.prepare_roundover_edges(f.object,edges,c.r,c.e);const info=JSON.parse(f.s.edge_treatment_info_json(h)),mesh=f.s.edge_treatment_mesh(h);try{mesh.positions();mesh.normals();triangles=mesh.indices().length/3;mesh.edge_positions()}finally{mesh.free()}f.s.discard_edge_treatment(h);if(i>=2)times.push(performance.now()-t);segments=info.segments}
 assert.deepEqual(f.s.save(),source);assert.equal(f.s.undo_depth(),depth);times.sort((a,b)=>a-b);results.push({case:c.name,radiusMm:c.r*1000,chordMm:c.e*1000,sourceSides:c.n??4,segments,triangles,samples:times.length,medianMs:times[1],maxMs:times.at(-1)});
 }finally{f.s.free()}}
console.log(JSON.stringify({environment:'Node release WASM; no GPU; 2 warmups and 3 retained samples; prepare/certification, mesh copy and discard; range query and Apply excluded',results},null,2));
