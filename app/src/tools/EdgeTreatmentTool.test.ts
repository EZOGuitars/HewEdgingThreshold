import { beforeEach, afterEach, describe, it, expect, vi } from 'vitest'
import * as THREE from 'three'
import { EdgeTreatmentTool, type TreatmentState, treatmentError } from './EdgeTreatmentTool'
import { ToolController } from './ToolController'
import { setLengthUnit } from '../settings/units'
import type { Scene } from '../wasm/loader'
const ray = {
  origin: [3, -1, 1] as [number, number, number],
  direction: [-1, 1, 0] as [number, number, number],
}
const snap = { x: 2, y: 0, z: 1, kind: 'on-edge', object: 1n, element: 2n, elementKind: 'edge' }
function fixture() {
  let next = 0,
    current = 0
  const infos = new Map<number, unknown>()
  const meshes: { free: ReturnType<typeof vi.fn> }[] = []
  const scene = {
    edge_endpoints: vi.fn(() => new Float64Array([2, 0, 0, 2, 0, 4])),
    prepare_chamfer: vi.fn((o: bigint, e: bigint, d: number) =>
      prepare(o, e, { kind: 'chamfer', distance: d }),
    ),
    prepare_roundover: vi.fn((o: bigint, e: bigint, r: number, c: number) =>
      prepare(o, e, { kind: 'roundover', radius: r, max_chord_error: c }),
    ),
    edge_treatment_info_json: vi.fn((h: number) => JSON.stringify(infos.get(h))),
    edge_treatment_mesh: vi.fn(() => {
      const m = {
        positions: () => new Float32Array([0, 0, 0, 1, 0, 0, 0, 1, 0]),
        normals: () => new Float32Array(9),
        indices: () => new Uint32Array([0, 1, 2]),
        edge_positions: () => new Float32Array([0, 0, 0, 1, 0, 0]),
        free: vi.fn(),
      }
      meshes.push(m)
      return m
    }),
    discard_edge_treatment: vi.fn((h: number) => {
      if (h !== current) throw 'InvalidCandidateHandle:'
      current = 0
    }),
    commit_edge_treatment: vi.fn((h: number) => {
      if (h !== current) throw 'InvalidCandidateHandle:'
      current = 0
      return { free: vi.fn() }
    }),
  }
  function prepare(o: bigint, e: bigint, operation: unknown) {
    current = ++next
    infos.set(current, {
      handle: current,
      object: String(o),
      edge: String(e),
      operation,
      warnings: [],
    })
    return current
  }
  let state: TreatmentState | null = null
  const group = new THREE.Group(),
    commit = vi.fn(),
    exit = vi.fn()
  const tool = new EdgeTreatmentTool(
    scene as unknown as Scene,
    group,
    commit,
    (s) => {
      state = s
    },
    exit,
  )
  tool.activate()
  return { tool, scene, group, commit, exit, meshes, infos, state: () => state! }
}
beforeEach(() => {
  vi.useFakeTimers()
  setLengthUnit('mm')
})
afterEach(() => vi.useRealTimers())
describe('EdgeTreatmentTool', () => {
  it('debounces rapid edits, passes BigInt IDs and exact units, frees the real mesh wrapper', () => {
    const f = fixture()
    f.tool.onPointerDown(snap, ray)
    f.tool.configure({ distance: '0.1234567890123 mm' })
    f.tool.configure({ distance: '0.2345678901234 mm' })
    expect(f.state().phase).toBe('pending')
    expect(f.scene.prepare_chamfer).not.toHaveBeenCalled()
    vi.advanceTimersByTime(180)
    expect(f.scene.prepare_chamfer).toHaveBeenCalledTimes(1)
    expect(f.scene.prepare_chamfer).toHaveBeenCalledWith(1n, 2n, 0.2345678901234 * 0.001)
    expect(f.state().phase).toBe('ready')
    expect(f.meshes[0].free).toHaveBeenCalledOnce()
    f.tool.cancel()
  })
  it('switches modes and disposes replaced GPU geometry/materials', () => {
    const f = fixture()
    f.tool.onPointerDown(snap, ray)
    vi.advanceTimersByTime(180)
    const candidate = f.group.children[0].children[0].children[0] as THREE.Mesh
    const dispose = vi.spyOn(candidate.geometry, 'dispose')
    const material = vi.spyOn(candidate.material as THREE.Material, 'dispose')
    f.tool.configure({ mode: 'roundover', radius: '2 mm', chord: '0.1 mm' })
    expect(dispose).toHaveBeenCalledOnce()
    expect(material).toHaveBeenCalledOnce()
    expect(f.state().phase).toBe('pending')
    f.tool.apply()
    expect(f.commit).not.toHaveBeenCalled()
    vi.advanceTimersByTime(180)
    expect(f.scene.prepare_roundover).toHaveBeenCalledWith(1n, 2n, 0.002, 0.0001)
    f.tool.cancel()
  })
  it.each(['0', '-1', 'NaN', 'Infinity', 'not a length', '1e999 m'])(
    'refuses invalid input %s without a native call or old Apply',
    (value) => {
      const f = fixture()
      f.tool.onPointerDown(snap, ray)
      vi.advanceTimersByTime(180)
      f.scene.prepare_chamfer.mockClear()
      f.tool.configure({ distance: value })
      f.tool.apply()
      vi.runAllTimers()
      expect(f.scene.prepare_chamfer).not.toHaveBeenCalled()
      expect(f.commit).not.toHaveBeenCalled()
      expect(f.state().phase).toBe('error')
      expect(f.scene.discard_edge_treatment).toHaveBeenCalledOnce()
      f.tool.cancel()
    },
  )
  it('failed preparation discards the retained old native slot and disables Apply', () => {
    const f = fixture()
    f.tool.onPointerDown(snap, ray)
    vi.advanceTimersByTime(180)
    f.scene.prepare_chamfer.mockImplementation(() => {
      throw 'InsufficientClearance:'
    })
    f.tool.configure({ distance: '20 mm' })
    vi.advanceTimersByTime(180)
    expect(f.scene.discard_edge_treatment).toHaveBeenCalledWith(1)
    expect(f.state().info).toBeNull()
    expect(f.state().error).toContain('Reduce')
    f.tool.apply()
    expect(f.commit).not.toHaveBeenCalled()
    f.tool.cancel()
  })
  it('mesh errors release the candidate and wrapper', () => {
    const f = fixture()
    const free = vi.fn()
    f.scene.edge_treatment_mesh.mockReturnValue({
      positions: () => {
        throw Error('mesh')
      },
      normals: () => new Float32Array(),
      indices: () => new Uint32Array(),
      edge_positions: () => new Float32Array(),
      free,
    })
    f.tool.onPointerDown(snap, ray)
    vi.advanceTimersByTime(180)
    expect(free).toHaveBeenCalledOnce()
    expect(f.scene.discard_edge_treatment).toHaveBeenCalledOnce()
    expect(f.state().phase).toBe('error')
    f.tool.cancel()
  })
  it('requires warning acknowledgment for this precise candidate and invalidates selection after Apply', () => {
    const f = fixture()
    f.tool.onPointerDown(snap, ray)
    f.infos.set(1, {
      handle: 1,
      object: '1',
      edge: '2',
      operation: { kind: 'chamfer', distance: 0.005 },
      warnings: [{ code: 'AnnotationDetachment', annotations: [{ sid: '42' }] }],
    })
    // Inject warning via the boundary accessor, after the mock prepares.
    f.scene.edge_treatment_info_json.mockImplementation(() =>
      JSON.stringify({
        handle: 1,
        object: '1',
        edge: '2',
        operation: { kind: 'chamfer', distance: 0.005 },
        warnings: [{ code: 'AnnotationDetachment', annotations: [{ sid: '42' }] }],
      }),
    )
    vi.advanceTimersByTime(180)
    f.tool.apply()
    expect(f.commit).not.toHaveBeenCalled()
    f.tool.acknowledge(true)
    f.tool.apply()
    expect(f.commit).toHaveBeenCalledOnce()
    expect(f.state().target).toBeNull()
    f.tool.apply()
    expect(f.commit).toHaveBeenCalledOnce()
    f.tool.cancel()
  })
  it('manual regeneration requires warning acknowledgment again', () => {
    const f = fixture()
    f.scene.edge_treatment_info_json.mockImplementation((h: number) =>
      JSON.stringify({
        handle: h,
        object: '1',
        edge: '2',
        operation: { kind: 'chamfer', distance: 0.005 },
        warnings: [{ code: 'AnnotationDetachment', annotations: [{ sid: '42' }] }],
      }),
    )
    f.tool.onPointerDown(snap, ray)
    vi.advanceTimersByTime(180)
    f.tool.acknowledge(true)
    f.tool.generate()
    expect(f.state().acknowledged).toBe(false)
    f.tool.apply()
    expect(f.commit).not.toHaveBeenCalled()
    f.tool.cancel()
  })
  it('refuses a candidate whose frozen dimensions differ from the input', () => {
    const f = fixture()
    f.scene.edge_treatment_info_json.mockImplementation(() =>
      JSON.stringify({
        handle: 1,
        object: '1',
        edge: '2',
        operation: { kind: 'chamfer', distance: 0.004 },
        warnings: [],
      }),
    )
    f.tool.onPointerDown(snap, ray)
    vi.advanceTimersByTime(180)
    expect(f.state().phase).toBe('error')
    f.tool.apply()
    expect(f.commit).not.toHaveBeenCalled()
    expect(f.scene.discard_edge_treatment).toHaveBeenCalledWith(1)
    f.tool.cancel()
  })
  it('disposes an allocation when preview setup throws before attachment', () => {
    const f = fixture()
    f.tool.onPointerDown(snap, ray)
    const dispose = vi.spyOn(THREE.BufferGeometry.prototype, 'dispose')
    const setup = vi
      .spyOn(THREE.BufferGeometry.prototype, 'setAttribute')
      .mockImplementationOnce(() => {
        throw new Error('preview buffer setup')
      })
    try {
      f.tool.generate()
      expect(f.state().phase).toBe('error')
      expect(f.scene.commit_edge_treatment).not.toHaveBeenCalled()
      expect(f.meshes[0].free).toHaveBeenCalledOnce()
      expect(f.scene.discard_edge_treatment).toHaveBeenCalledOnce()
      expect(dispose).toHaveBeenCalledOnce()
    } finally {
      f.tool.cancel()
      setup.mockRestore()
      dispose.mockRestore()
    }
  })

  it('disposes both allocations if edge setup fails after the surface is assembled', () => {
    const f = fixture()
    f.tool.onPointerDown(snap, ray)
    const dispose = vi.spyOn(THREE.BufferGeometry.prototype, 'dispose')
    const original = THREE.BufferGeometry.prototype.setAttribute
    let attributes = 0
    const setup = vi
      .spyOn(THREE.BufferGeometry.prototype, 'setAttribute')
      .mockImplementation(function (this: THREE.BufferGeometry, name, attribute) {
        if (++attributes === 3) throw new Error('edge buffer setup')
        return original.call(this, name, attribute)
      })
    try {
      f.tool.generate()
      expect(f.state().phase).toBe('error')
      expect(f.meshes[0].free).toHaveBeenCalledOnce()
      expect(f.scene.discard_edge_treatment).toHaveBeenCalledOnce()
      expect(dispose).toHaveBeenCalledTimes(2)
    } finally {
      f.tool.cancel()
      setup.mockRestore()
      dispose.mockRestore()
    }
  })

  it('reports a committed treatment accurately when document refresh throws', () => {
    const f = fixture()
    f.tool.onPointerDown(snap, ray)
    f.tool.generate()
    const change = { free: vi.fn() }
    f.scene.commit_edge_treatment.mockReturnValue(change)
    f.commit.mockImplementation(() => {
      throw new Error('renderer refresh')
    })
    try {
      f.tool.apply()
      expect(f.scene.commit_edge_treatment).toHaveBeenCalledOnce()
      expect(change.free).toHaveBeenCalledOnce()
      expect(f.state().target).toBeNull()
      expect(f.state().phase).toBe('error')
      expect(f.state().error).toContain('was applied')
      f.tool.apply()
      expect(f.scene.commit_edge_treatment).toHaveBeenCalledOnce()
    } finally {
      f.tool.cancel()
    }
  })

  it('always frees the commit result and notifies the host if preview cleanup throws', () => {
    const f = fixture()
    f.tool.onPointerDown(snap, ray)
    f.tool.generate()
    const candidate = f.group.children[0].children[0].children[0] as THREE.Mesh
    let thrown = false
    candidate.geometry.addEventListener('dispose', () => {
      if (!thrown) {
        thrown = true
        throw new Error('dispose listener')
      }
    })
    const change = { free: vi.fn() }
    f.scene.commit_edge_treatment.mockReturnValue(change)
    try {
      f.tool.apply()
      expect(f.scene.commit_edge_treatment).toHaveBeenCalledOnce()
      expect(f.commit).toHaveBeenCalledOnce()
      expect(change.free).toHaveBeenCalledOnce()
      expect(f.state().target).toBeNull()
      expect(f.state().error).toContain('was applied')
    } finally {
      f.tool.cancel()
    }
  })

  it('consumes stale failures without document refresh', () => {
    const f = fixture()
    f.tool.onPointerDown(snap, ray)
    vi.advanceTimersByTime(180)
    f.scene.commit_edge_treatment.mockImplementation(() => {
      throw 'StaleCandidate:'
    })
    f.tool.apply()
    expect(f.commit).not.toHaveBeenCalled()
    expect(f.state().target).toBeNull()
    expect(f.state().phase).toBe('error')
    expect(f.scene.discard_edge_treatment).not.toHaveBeenCalled()
    f.tool.cancel()
  })
  it('changing selection, context, or document invalidates topology and pending work', () => {
    const f = fixture()
    f.tool.onPointerDown(snap, ray)
    vi.advanceTimersByTime(180)
    f.tool.onPointerDown({ ...snap, element: 3n }, ray)
    expect(f.scene.discard_edge_treatment).toHaveBeenCalledWith(1)
    f.tool.setEditContext({ kind: 'object', id: 2n })
    vi.runAllTimers()
    expect(f.state().target).toBeNull()
    f.tool.onPointerDown(snap, ray)
    vi.advanceTimersByTime(180)
    f.tool.documentChanged()
    expect(f.state().target).toBeNull()
    f.tool.cancel()
  })
  it('rejects instance picks, nonsolid picks and ineligible grouped edges', () => {
    const f = fixture()
    f.tool.onPointerDown({ ...snap, instance: 8n }, ray)
    expect(f.state().error).toContain('Component')
    f.tool.onPointerDown(null, ray)
    expect(f.state().error).toContain('middle')
    f.tool.setFaceEligibility(() => false)
    f.tool.onPointerDown(snap, ray)
    vi.runAllTimers()
    expect(f.scene.prepare_chamfer).not.toHaveBeenCalled()
    f.tool.cancel()
  })
  it('tool switches cancel timers, release candidates and remove the owned preview subtree', () => {
    const f = fixture()
    const controller = new ToolController(f.scene as unknown as Scene, vi.fn())
    controller.setTool(f.tool)
    f.tool.onPointerDown(snap, ray)
    vi.advanceTimersByTime(180)
    controller.resetToSelect()
    expect(f.scene.discard_edge_treatment).toHaveBeenCalledOnce()
    expect(f.group.children).toHaveLength(0)
    expect(f.state()).toBeNull()
    vi.runAllTimers()
    expect(f.scene.prepare_chamfer).toHaveBeenCalledOnce()
  })
  it('Escape and Cancel request normal tool exit and leave no preview', () => {
    const f = fixture()
    f.tool.onPointerDown(snap, ray)
    f.tool.onKey({ key: 'Escape' } as KeyboardEvent)
    expect(f.exit).toHaveBeenCalledOnce()
    vi.runAllTimers()
    expect(f.scene.prepare_chamfer).not.toHaveBeenCalled()
    f.tool.cancel()
  })
  it('keeps exact parsed values when display units change', () => {
    const f = fixture()
    f.tool.configure({ distance: '0.123456789123456 mm' })
    setLengthUnit('cm')
    expect(f.state().unit).toBe('cm')
    expect(f.state().distance).toBe(`${0.123456789123456 * 0.001} m`)
    f.tool.cancel()
  })
  it('maps actionable structured errors without exposing raw panic text', () => {
    expect(treatmentError(new Error('StaleCandidate: regenerate'))).toContain('no longer current')
    for (const c of [
      'UnknownObject',
      'UnknownEdge',
      'HiddenTarget',
      'InvalidChordError',
      'OwnedSketchesUnsupported',
      'UnsupportedDefinitionMember',
      'InvalidResult',
    ])
      expect(treatmentError(`${c}: raw panic`)).not.toContain('raw panic')
  })
})
