import init, { Scene } from '../wasm/pkg/wasm_api.js'
import type { PreviewJob, PreviewResult } from './edgeTreatmentPreview'
const host = globalThis as unknown as {
  onmessage: (event: MessageEvent<PreviewJob>) => void
  postMessage(value: { result?: PreviewResult; error?: string }, transfer?: Transferable[]): void
}
host.onmessage = async ({ data }) => {
  let scene: Scene | null = null
  let initialized = false
  try {
    const kernel = await init()
    initialized = true
    scene = new Scene()
    scene.load(data.source)
    const started = performance.now()
    let limits: string | null = null
    if (data.limits) {
      try { limits = scene.edge_treatment_request_limits_json(data.request) } catch { /* precision may constrain a geometric bound */ }
    }
    const handle = scene.prepare_edge_treatment_request_json(data.request)
    const info = scene.edge_treatment_info_json(handle)
    const mesh = scene.edge_treatment_mesh(handle)
    try {
      const result: PreviewResult = {
        info, limits, elapsedMs: performance.now() - started, memoryBytes: kernel.memory.buffer.byteLength,
        mesh: { positions: mesh.positions().slice(), normals: mesh.normals().slice(), indices: mesh.indices().slice(), edges: mesh.edge_positions().slice() },
      }
      host.postMessage({ result }, [result.mesh.positions.buffer, result.mesh.normals.buffer, result.mesh.indices.buffer, result.mesh.edges.buffer])
    } finally { mesh.free() }
  // oxlint-disable-next-line lingui/no-unlocalized-strings -- Stable worker failure code; translated by treatmentError
  } catch (error) { host.postMessage({ error: initialized ? String(error) : `PreviewWorkerFailed: ${String(error)}` }) }
  finally { scene?.free() }
}
