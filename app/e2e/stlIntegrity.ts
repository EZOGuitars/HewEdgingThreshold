/** Independent binary-STL checks. Coordinates are millimeters, as written by Hew. */
export function inspectClosedStl(bytes: Uint8Array | number[]) {
  const data = Uint8Array.from(bytes)
  const view = new DataView(data.buffer)
  const count = view.getUint32(80, true)
  if (data.length !== 84 + count * 50 || !count) throw Error('Invalid STL size')
  const edges = new Map<string, { count: number; balance: number }>()
  let volume = 0
  const min = [Infinity, Infinity, Infinity], max = [-Infinity, -Infinity, -Infinity]
  for (let i = 0; i < count; i++) {
    const p = [0, 1, 2].map(j => [0, 1, 2].map(k => view.getFloat32(84 + i * 50 + 12 + j * 12 + k * 4, true)))
    for (const point of p) for (let k = 0; k < 3; k++) {
      if (!Number.isFinite(point[k])) throw Error('Nonfinite STL vertex')
      min[k] = Math.min(min[k], point[k]); max[k] = Math.max(max[k], point[k])
    }
    const [a, b, c] = p
    const u = b.map((x, k) => x - a[k]), v = c.map((x, k) => x - a[k])
    const cross = [u[1]*v[2]-u[2]*v[1], u[2]*v[0]-u[0]*v[2], u[0]*v[1]-u[1]*v[0]]
    if (Math.hypot(...cross) === 0) throw Error('Degenerate STL triangle')
    volume += (a[0]*(b[1]*c[2]-b[2]*c[1]) + a[1]*(b[2]*c[0]-b[0]*c[2]) + a[2]*(b[0]*c[1]-b[1]*c[0])) / 6
    const keys = p.map(point => point.join(','))
    for (let j = 0; j < 3; j++) {
      const x = keys[j], y = keys[(j + 1) % 3]
      const forward = x < y
      const key = forward ? `${x}|${y}` : `${y}|${x}`
      const record = edges.get(key) ?? { count: 0, balance: 0 }
      record.count++; record.balance += forward ? 1 : -1
      edges.set(key, record)
    }
  }
  if ([...edges.values()].some(e => e.count !== 2 || e.balance !== 0)) throw Error('STL has open, nonmanifold or inconsistently wound edges')
  if (!(volume > 0)) throw Error('Nonpositive STL volume')
  return { triangles: count, volumeMm3: volume, min, max }
}

/** Compare decoded coordinates independently of face order and triangulation. */
export function stlVertexSet(bytes: Uint8Array | number[]) {
  const data = Uint8Array.from(bytes), view = new DataView(data.buffer)
  const vertices = new Set<string>()
  for (let i = 0; i < view.getUint32(80, true); i++) for (let j = 0; j < 3; j++) {
    vertices.add([0,1,2].map(k => view.getFloat32(84+i*50+12+j*12+k*4,true)).join(','))
  }
  return [...vertices].sort()
}
