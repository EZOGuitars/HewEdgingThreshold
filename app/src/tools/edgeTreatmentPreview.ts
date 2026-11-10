/** Feature-local worker ownership. Termination cancels actual synchronous WASM;
 * revision checks additionally prevent late events publishing after supersession. */
export interface PreviewBuffers {
  positions: Float32Array
  normals: Float32Array
  indices: Uint32Array
  edges: Float32Array
}
export interface PreviewJob { source: Uint8Array; request: string; limits: boolean }
export interface PreviewResult { info: string; mesh: PreviewBuffers; limits: string | null; elapsedMs: number; memoryBytes: number }
export class EdgeTreatmentPreviewRunner {
  private worker: Worker | null = null
  private revision = 0
  private reject: ((error: Error) => void) | null = null
  // oxlint-disable-next-line lingui/no-unlocalized-strings -- Worker module URL, not user-facing copy
  constructor(private create: () => Worker = () => new Worker(new URL('./edgeTreatmentPreview.worker.ts', import.meta.url), { type: 'module' })) {}
  cancel(): void {
    ++this.revision
    this.worker?.terminate()
    this.worker = null
    this.reject?.(new Error('PreviewSuperseded'))
    this.reject = null
  }
  run(job: PreviewJob): Promise<PreviewResult> {
    this.cancel()
    const revision = this.revision
    return new Promise((resolve, reject) => {
      this.reject = reject
      let worker: Worker
      try { worker = this.create() } catch { this.reject = null; reject(new Error('PreviewWorkerFailed')); return }
      this.worker = worker
      const finish = () => { worker.terminate(); if (this.worker === worker) { this.worker = null; this.reject = null } }
      worker.onmessage = (event: MessageEvent<{ result?: PreviewResult; error?: string }>) => {
        if (revision !== this.revision) return
        finish()
        if (event.data.result) resolve(event.data.result)
        else reject(new Error(event.data.error ?? 'PreviewFailed'))
      }
      worker.onerror = () => { if (revision === this.revision) { finish(); reject(new Error('PreviewWorkerFailed')) } }
      // Structured clone keeps the caller's source snapshot intact for Apply.
      worker.postMessage(job)
    })
  }
}
