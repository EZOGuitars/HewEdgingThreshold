import { createRequire } from 'node:module'
import { existsSync } from 'node:fs'
import { describe, it, expect } from 'vitest'
import * as THREE from 'three'
import { EdgeTreatmentTool, type TreatmentState } from './EdgeTreatmentTool'
import { parseLengthToMeters, setLengthUnit, setPrecision } from '../settings/units'
import type { Scene } from '../wasm/loader'

const path = new URL('../../../tools/replay-runner/pkg-node/wasm_api.js', import.meta.url)
const pairs = [
  [0.125, 3.175],
  [0.25, 6.35],
  [0.375, 9.525],
  [0.5, 12.7],
  [0.75, 19.05],
  [1, 25.4],
  [2, 50.8],
  [4, 101.6],
] as const
const side = 0.3048 // Fresh 12-inch cubes, in canonical meters.
const meshTolerance = side * 8 * 2 ** -23
function fixture() {
  const { Scene: NativeScene } = createRequire(import.meta.url)(path.pathname)
  const scene: Scene = new NativeScene()
  const sketch = scene.begin_ground_sketch()
  scene.sketch_begin_gesture(sketch)
  const p = [
    [0, 0],
    [side, 0],
    [side, side],
    [0, side],
  ]
  for (let i = 0; i < 4; i++)
    scene.sketch_add_segment(
      sketch,
      ...([p[i][0], p[i][1], 0, p[(i + 1) % 4][0], p[(i + 1) % 4][1], 0] as [
        number,
        number,
        number,
        number,
        number,
        number,
      ]),
    )
  scene.sketch_end_gesture(sketch)
  const object = scene.extrude_region(sketch, scene.sketch_regions(sketch)[0], side)
  const picked = scene.snap(side * 1.5, -side / 2, side / 2, -Math.SQRT1_2, Math.SQRT1_2, 0, 0.001)!
  expect(picked.object()).toBe(object)
  expect(picked.element_kind()).toBe('edge')
  const edge = picked.element()
  picked.free()
  let state: TreatmentState | null = null
  const tool = new EdgeTreatmentTool(
    scene,
    new THREE.Group(),
    () => {},
    (s) => {
      state = s
    },
    () => {},
  )
  tool.activate()
  tool.onPointerDown(
    {
      x: side,
      y: 0,
      z: side / 2,
      kind: 'on-edge',
      object,
      element: edge,
      elementKind: 'edge',
    },
    {
      origin: [side * 1.5, -side / 2, side / 2],
      direction: [-Math.SQRT1_2, Math.SQRT1_2, 0],
    },
  )
  return { scene, tool, state: () => state! }
}
function vertices(bytes: Uint8Array, sorted = true): number[][] {
  const v = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  const count = v.getUint32(80, true)
  expect(bytes.length).toBe(84 + 50 * count)
  const points: number[][] = []
  for (let i = 0; i < count; i++)
    for (let j = 0; j < 3; j++)
      points.push(
        [0, 1, 2].map((k) => v.getFloat32(84 + 50 * i + 12 + 12 * j + 4 * k, true) / 1000),
      )
  return sorted ? points.sort((a, b) => a[0] - b[0] || a[1] - b[1] || a[2] - b[2]) : points
}
function measure(bytes: Uint8Array, radius: number, rounded = true, requestedChord = 0.00001) {
  const points = vertices(bytes, false)
  const dims = [0, 1, 2].map(
    (axis) => Math.max(...points.map((p) => p[axis])) - Math.min(...points.map((p) => p[axis])),
  )
  for (const d of dims) expect(Math.abs(d - side)).toBeLessThanOrEqual(meshTolerance)
  // At this independently picked corner (x=side,y=0), recover both setbacks.
  const rx = side - Math.max(...points.filter((p) => Math.abs(p[1]) < 1e-10).map((p) => p[0]))
  const ry = Math.min(
    ...points.filter((p) => Math.abs(p[0] - side) < meshTolerance).map((p) => p[1]),
  )
  const error = Math.max(Math.abs(rx - radius), Math.abs(ry - radius))
  expect(error).toBeLessThanOrEqual(meshTolerance)
  const edges = new Map<string, number>()
  const chords = new Set<string>()
  const center = new THREE.Vector3(side - radius, radius, 0)
  const witness = new THREE.Vector3(side / 2, side / 2, side / 2)
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  let volume = 0,
    maximumChord = 0
  const onArc = (p: THREE.Vector3) =>
    p.x >= side - radius - meshTolerance &&
    p.y <= radius + meshTolerance &&
    Math.abs(Math.hypot(p.x - center.x, p.y - center.y) - radius) <= meshTolerance
  for (let i = 0; i < points.length; i += 3) {
    const t = points.slice(i, i + 3).map((p) => new THREE.Vector3(p[0], p[1], p[2]))
    const cross = t[1].clone().sub(t[0]).cross(t[2].clone().sub(t[0]))
    expect(cross.length()).toBeGreaterThan(0)
    expect(cross.dot(t[0].clone().sub(witness))).toBeGreaterThan(0)
    const at = 84 + 50 * (i / 3)
    const normal = new THREE.Vector3(
      view.getFloat32(at, true),
      view.getFloat32(at + 4, true),
      view.getFloat32(at + 8, true),
    )
    expect(cross.dot(normal)).toBeGreaterThan(0)
    volume += t[0].dot(t[1].clone().cross(t[2])) / 6
    for (let j = 0; j < 3; j++) {
      const a = t[j],
        b = t[(j + 1) % 3]
      const key = JSON.stringify([a.toArray(), b.toArray()])
      edges.set(key, (edges.get(key) ?? 0) + 1)
      // Only actual arc edges on a vertical side facet, not cap diagonals.
      if (
        rounded &&
        Math.max(...t.map((p) => p.z)) - Math.min(...t.map((p) => p.z)) > side / 2 &&
        Math.abs(a.z) < 1e-10 &&
        Math.abs(b.z) < 1e-10 &&
        onArc(a) &&
        onArc(b)
      ) {
        chords.add(key)
        const d = b.clone().sub(a)
        const fraction = Math.max(0, Math.min(1, center.clone().sub(a).dot(d) / d.lengthSq()))
        const closest = a.clone().addScaledVector(d, fraction)
        for (const p of [a, b, closest])
          maximumChord = Math.max(maximumChord, Math.abs(p.distanceTo(center) - radius))
      }
    }
  }
  for (const [key, count] of edges) {
    const pair = JSON.parse(key) as number[][]
    expect(count).toBe(1)
    expect(edges.get(JSON.stringify([pair[1], pair[0]]))).toBe(1)
  }
  const segments = chords.size
  if (rounded) {
    expect(segments).toBeGreaterThanOrEqual(4)
    expect(segments).toBeLessThanOrEqual(128)
    expect(maximumChord).toBeLessThanOrEqual(requestedChord + meshTolerance)
    expect(
      Math.abs(maximumChord - radius * (1 - Math.cos(Math.PI / (4 * segments)))),
    ).toBeLessThanOrEqual(meshTolerance)
  }
  const ideal = rounded ? side * radius ** 2 * (1 - Math.PI / 4) : (side * radius ** 2) / 2
  const expected = rounded
    ? side * radius ** 2 * (1 - (segments * Math.sin(Math.PI / (2 * segments))) / 2)
    : ideal
  const removed = side ** 3 - volume
  expect(Math.abs(removed - expected)).toBeLessThanOrEqual(side ** 3 * 2e-6)
  return {
    requested_radius_in: radius / 0.0254,
    requested_chord_mm: rounded ? requestedChord * 1000 : null,
    strict_stl_chord_compliant: !rounded || maximumChord <= requestedChord,
    stl_chord_excess_mm: rounded ? Math.max(0, maximumChord - requestedChord) * 1000 : 0,
    measured_x_mm: rx * 1000,
    measured_y_mm: ry * 1000,
    absolute_error_mm: error * 1000,
    absolute_error_in: error / 0.0254,
    relative_error: error / radius,
    stl_dimensional_error_mm: Math.max(...dims.map((d) => Math.abs(d - side))) * 1000,
    measured_x_in: rx / 0.0254,
    measured_y_in: ry / 0.0254,
    outer_mm: dims.map((d) => d * 1000),
    outer_in: dims.map((d) => d / 0.0254),
    max_chord_mm: maximumChord * 1000,
    max_chord_in: maximumChord / 0.0254,
    segments,
    ideal_removed_mm3: ideal * 1e9,
    expected_faceted_removed_mm3: expected * 1e9,
    measured_stl_removed_mm3: removed * 1e9,
    faceting_excess_mm3: (expected - ideal) * 1e9,
    stl_removed_volume_error_mm3: Math.abs(removed - expected) * 1e9,
    ideal_removed_in3: ideal / 0.0254 ** 3,
    expected_faceted_removed_in3: expected / 0.0254 ** 3,
    measured_stl_removed_in3: removed / 0.0254 ** 3,
    triangles: points.length / 3,
    stl_manifold_outward: true,
  }
}
function run(mode: 'chamfer' | 'roundover', text: string, unit: 'mm' | 'dec_in') {
  setLengthUnit(unit)
  const f = fixture()
  try {
    f.tool.configure({ mode, distance: text, radius: text, chord: '0.01 mm' })
    f.tool.generate()
    expect(f.state().phase).toBe('ready')
    const operation = f.state().info!.operation as {
      radius?: number
      distance?: number
      max_chord_error?: number
    }
    const value = mode === 'roundover' ? operation.radius! : operation.distance!
    expect(value).toBe(parseLengthToMeters(text, unit))
    if (mode === 'roundover') expect(operation.max_chord_error).toBe(0.00001)
    f.tool.apply()
    return { value, bytes: f.scene.export('stl', 0)! }
  } finally {
    f.tool.cancel()
    f.scene.free()
  }
}
// Real generated bindings are required for the dedicated verification command.
describe.skipIf(!existsSync(path))('metric dimensions across UI and actual WASM', () => {
  for (const mode of ['chamfer', 'roundover'] as const)
    it.each(pairs)(
      `${mode}: %s inches equals %s mm, measured after commit and STL import`,
      (inches, mm) => {
        const metric = run(mode, String(mm), 'mm')
        const imperial = run(mode, String(inches), 'dec_in')
        const conversionError = Math.abs(metric.value - imperial.value)
        expect(conversionError).toBeLessThanOrEqual(2e-17)
        const a = vertices(metric.bytes),
          b = vertices(imperial.bytes)
        expect(a.length).toBe(b.length)
        const difference = Math.max(
          ...a.flatMap((p, i) => p.map((v, axis) => Math.abs(v - b[i][axis]))),
        )
        expect(difference).toBeLessThanOrEqual(meshTolerance)
        const measured = measure(metric.bytes, mm / 1000, mode === 'roundover')
        measure(imperial.bytes, mm / 1000, mode === 'roundover')
        // STL has no physical unit metadata. Exercise both explicitly specified
        // external-unit workflows, including an inch-coordinate STL conversion.
        const roundtrips: { meters_per_stl_unit: number; measured: ReturnType<typeof measure> }[] =
          []
        for (const scale of [0.001, 0.0254]) {
          const bytes = metric.bytes.slice()
          if (scale === 0.0254) {
            const v = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
            for (let i = 0; i < v.getUint32(80, true); i++)
              for (let j = 0; j < 9; j++) {
                const at = 84 + 50 * i + 12 + 4 * j
                v.setFloat32(at, v.getFloat32(at, true) / 25.4, true)
              }
          }
          const { Scene: NativeScene } = createRequire(import.meta.url)(path.pathname)
          const loaded: Scene = new NativeScene()
          try {
            loaded.import_stl(bytes, scale, 'explicit-unit-roundtrip')
            const imported = measure(loaded.export('stl', 0)!, mm / 1000, mode === 'roundover')
            expect(imported.stl_dimensional_error_mm).toBeLessThanOrEqual(meshTolerance * 1000)
            roundtrips.push({ meters_per_stl_unit: scale, measured: imported })
          } finally {
            loaded.free()
          }
        }
        process.stdout.write(
          'METRIC_WASM,' +
            JSON.stringify({
              mode,
              requested_mm: mm,
              ...measured,
              roundtrips,
              conversion_error_mm: conversionError * 1000,
              equivalent_mesh_error_mm: difference * 1000,
            }) +
            '\n',
        )
      },
    )
  it.each([0.1, 0.25, 0.5, 1.25, 2.54, 6.35, 12.7])(
    'decimal %s mm: display and 100 unit cycles retain the actual dimension',
    (mm) => {
      setLengthUnit('mm')
      setPrecision(0, 'mm')
      setPrecision(0, 'dec_in')
      const f = fixture()
      try {
        f.tool.configure({
          mode: 'roundover',
          radius: String(mm),
          chord: '0.01 mm',
        })
        f.tool.generate()
        const exact = (f.state().info!.operation as { radius: number }).radius
        for (let i = 0; i < 100; i++) {
          setLengthUnit('dec_in')
          if (mm === 6.35) expect(f.state().radius).toBe('0.25"')
          setLengthUnit('mm')
        }
        expect(f.state().radius).toBe(`${mm} mm`)
        f.tool.generate()
        expect((f.state().info!.operation as { radius: number }).radius).toBe(exact)
        f.tool.apply()
        const measured = measure(f.scene.export('stl', 0)!, mm / 1000)
        process.stdout.write(
          'METRIC_DECIMAL,' +
            JSON.stringify({
              requested_mm: mm,
              ...measured,
              conversion_drift_mm: 0,
            }) +
            '\n',
        )
      } finally {
        f.tool.cancel()
        f.scene.free()
        setPrecision('exact', 'mm')
        setPrecision('exact', 'dec_in')
      }
    },
  )
  it('display precision ceilings do not replace the canonical WASM dimension', () => {
    setLengthUnit('mm')
    const f = fixture()
    try {
      const text = '6.350000000123456'
      const exact = parseLengthToMeters(text, 'mm')!
      f.tool.configure({ mode: 'roundover', radius: text, chord: '0.01 mm' })
      for (let i = 0; i < 100; i++) {
        setLengthUnit('dec_in')
        setLengthUnit('mm')
      }
      expect(parseLengthToMeters(f.state().radius, 'mm')).not.toBe(exact)
      f.tool.generate()
      expect((f.state().info!.operation as { radius: number }).radius).toBe(exact)
      f.tool.apply()
      measure(f.scene.export('stl', 0)!, exact)
    } finally {
      f.tool.cancel()
      f.scene.free()
    }
  })
  it.each([0.01, 0.025, 0.05, 0.1, 0.25])(
    'passes chord tolerance %s mm through the actual boundary',
    (mm) => {
      setLengthUnit('mm')
      const f = fixture()
      try {
        f.tool.configure({
          mode: 'roundover',
          radius: '50 mm',
          chord: String(mm),
        })
        f.tool.generate()
        expect(f.state().phase).toBe('ready')
        expect((f.state().info!.operation as { max_chord_error: number }).max_chord_error).toBe(
          mm * 0.001,
        )
        f.tool.apply()
        const measured = measure(f.scene.export('stl', 0)!, 0.05, true, mm * 0.001)
        process.stdout.write(
          'METRIC_CHORD,' + JSON.stringify({ requested_chord_mm: mm, ...measured }) + '\n',
        )
      } finally {
        f.tool.cancel()
        f.scene.free()
      }
    },
  )
})
