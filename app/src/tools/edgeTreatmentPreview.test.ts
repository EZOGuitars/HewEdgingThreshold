import { it, expect, vi } from 'vitest'
import { EdgeTreatmentPreviewRunner, type PreviewResult } from './edgeTreatmentPreview'
it('terminates obsolete computations and rejects late results after supersession/cancel', async () => {
  const workers: { terminate: ReturnType<typeof vi.fn>; postMessage: ReturnType<typeof vi.fn>; onmessage: ((event: unknown) => void) | null; onerror: (() => void) | null }[] = []
  const runner = new EdgeTreatmentPreviewRunner(() => {
    const worker = { terminate: vi.fn(), postMessage: vi.fn(), onmessage: null, onerror: null }
    workers.push(worker)
    return worker as unknown as Worker
  })
  const source = Uint8Array.from([1, 2, 3])
  const first = runner.run({ source, request: 'old', limits: true }).catch(e => e.message)
  const late = workers[0]
  const second = runner.run({ source, request: 'new', limits: false })
  expect(await first).toBe('PreviewSuperseded')
  expect(late.terminate).toHaveBeenCalledOnce()
  const result = { info: 'new' } as PreviewResult
  late.onmessage!({ data: { result: { info: 'old' } } })
  workers[1].onmessage!({ data: { result } })
  expect(await second).toEqual(result)
  expect(source).toEqual(Uint8Array.from([1, 2, 3]))
  const canceled = runner.run({ source, request: 'cancel', limits: false }).catch(e => e.message)
  runner.cancel()
  expect(await canceled).toBe('PreviewSuperseded')
  expect(workers[2].terminate).toHaveBeenCalledOnce()
})
it('reports worker startup failure without leaking a pending worker', async () => {
  const runner=new EdgeTreatmentPreviewRunner(()=>{throw new Error('blocked by policy')})
  await expect(runner.run({source:new Uint8Array(),request:'unused',limits:false})).rejects.toThrow('PreviewWorkerFailed')
  runner.cancel()
})
