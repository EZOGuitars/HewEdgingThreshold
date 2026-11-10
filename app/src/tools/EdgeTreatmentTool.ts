import * as THREE from 'three'
import { t } from '@lingui/core/macro'
import { parseKernelErrorCode } from '../kernelErrors'
import type { Scene, DocChangeJs } from '../wasm/loader'
import type { Tool, Snap, EditContext, SnapConstraint } from './types'
import { editContextEq } from './types'
import type { Ray } from '../viewport/math'
import { clearPreview, markSessionGhost } from './transformPreview'
import {
  formatLengthIn,
  metersFromUnit,
  getLengthUnit,
  parseLengthToMeters,
  subscribe,
  type LengthFormat,
} from '../settings/units'

// Editable values use numeric precision rather than display rounding policy.
function formatTreatmentInput(meters: number, unit: LengthFormat): string {
  const value = String(meters / metersFromUnit(1, unit))
  return ['m', 'cm', 'mm'].includes(unit) ? `${value} ${unit}` : `${value}"`
}

export type TreatmentMode = 'chamfer' | 'roundover'
export interface TreatmentInfo {
  handle: number
  object: string
  edge: string
  edges?: string[]
  segments?: number
  operation: { kind: TreatmentMode; distance?: number; radius?: number; max_chord_error?: number }
  warnings: { code: string; annotations: { annotation: string; sid: string }[] }[]
}
export interface TreatmentState {
  mode: TreatmentMode
  distance: string
  radius: string
  chord: string
  unit: LengthFormat
  // Keep arrays of IDs serializable for React development instrumentation.
  target: { object: bigint; edge: bigint; edges?: string[] } | null
  pickingFace?: boolean
  phase: 'select' | 'pending' | 'ready' | 'error'
  error: string | null
  info: TreatmentInfo | null
  acknowledged: boolean
  manualPreview?: boolean
  dimensionMeters?: number
  limits?: { maximum: number; certainty: 'conservative'; segments_at_maximum: number } | null
}
export function treatmentError(error: unknown): string {
  const code =
    parseKernelErrorCode(error) ??
    (error instanceof Error ? error.message : String(error)).split(':')[0]
  switch (code) {
    case 'UnknownObject':
      return t`This object is no longer available. Select another edge.`
    case 'UnknownEdge':
      return t`This edge is no longer available. Select the edge again.`
    case 'InvalidDistance':
    case 'InvalidRadius':
      return t`Enter a finite positive distance or radius. Very small cuts may be below modeling tolerance.`
    case 'InvalidChordError':
    case 'ApproximationTooFine':
      return t`Enter a positive chord error above modeling tolerance, or increase it to use fewer segments.`
    case 'StlChordPrecision':
      return t`STL precision cannot preserve this chord tolerance here. Use a finer chord setting or move the model closer to the origin.`
    case 'OwnedSketchesUnsupported':
      return t`This object has an attached sketch. Edge treatment cannot safely preserve it yet.`
    case 'UnsupportedDefinitionMember':
      return t`Component definition edges are not supported. Select a plain solid outside component editing.`
    case 'HiddenTarget':
      return t`Show the target object before selecting its edge.`
    case 'ActiveEditingSession':
    case 'UnsupportedSelectionContext':
      return t`Finish group, component, or drawing editing, then select a plain solid edge.`
    case 'StaleCandidate':
    case 'InvalidCandidateHandle':
      return t`The preview is no longer current. Select the edge again and regenerate it.`
    case 'InsufficientClearance':
      return t`The cut reaches another face. Reduce the distance or radius.`
    case 'UnsupportedCurvedBoundary':
      return t`Circular and mixed curved perimeters need a certified curved evaluator. The source remains unchanged.`
    case 'UnsupportedBox':
      return t`Select a complete supported convex cap perimeter, or edges on an unmodified rectangular solid. Nonconvex solids are not supported.`
    case 'InvalidSelection':
      return t`Select distinct edges on one solid.`
    case 'UnknownFace':
      return t`This face is no longer available. Select the face again.`
    case 'UnsupportedSolid':
    case 'NonPerpendicularFaces':
    case 'UnsupportedTermination':
    case 'InvalidSource':
      return t`Select a straight convex edge between perpendicular planar faces on a closed convex solid. Curved and concave edges are not supported.`
    default:
      return t`The treatment could not be validated. Reduce its dimensions or select another supported straight convex edge.`
  }
}

/** One tool, one Scene candidate, one owned transient subtree. Never writes
 * geometry in JS. Input revisions invalidate Apply BEFORE the debounce. */
export class EdgeTreatmentTool implements Tool {
  readonly name = 'Edge Treatment'
  private root = markSessionGhost(new THREE.Group())
  private meshGroup = new THREE.Group()
  private edgeGroup = new THREE.Group()
  private handle: number | null = null
  private timer: ReturnType<typeof setTimeout> | null = null
  // Display formatting has a finite precision ceiling. Never reparse its
  // rounded text after a unit switch; only an actual field edit replaces this.
  private canonical: Partial<Record<'distance' | 'radius' | 'chord', number>> = { distance: 0.005, radius: 0.005, chord: 0.0001 }
  private limitsKey: string | null = null
  private additive = false
  private active = false
  private unsubscribe: (() => void) | null = null
  private context: EditContext = { kind: 'top' }
  private eligible: (o: bigint, i: bigint | undefined) => boolean = () => true
  private state: TreatmentState = {
    mode: 'chamfer',
    distance: formatLengthIn(0.005, getLengthUnit(), 'exact'),
    radius: formatLengthIn(0.005, getLengthUnit(), 'exact'),
    chord: formatLengthIn(0.0001, getLengthUnit(), 'exact'),
    unit: getLengthUnit(),
    target: null,
    phase: 'select',
    error: null,
    info: null,
    acknowledged: false,
  }
  constructor(
    private scene: Scene,
    private preview: THREE.Group,
    private committed: (object: bigint, change: DocChangeJs) => void,
    private changed: (state: TreatmentState | null) => void,
    private exit: () => void,
  ) {
    this.root.add(this.meshGroup, this.edgeGroup)
  }
  activate(): void {
    if (this.active) return
    this.active = true
    this.preview.add(this.root)
    this.unsubscribe = subscribe((unit) => {
      // Keep canonical meters while formatting the selected display unit.
      const old = this.state.unit
      for (const field of ['distance', 'radius', 'chord'] as const) {
        const value = this.canonical[field] ?? parseLengthToMeters(this.state[field], old)
        if (value !== null) {
          this.canonical[field] = value
          this.state[field] = formatTreatmentInput(value, unit)
        }
      }
      this.state.unit = unit
      this.emit() // Unit display changes do not change a physical request.
    })
    this.emit()
  }
  setFaceEligibility(fn: (o: bigint, i: bigint | undefined) => boolean): void {
    this.eligible = fn
  }
  setEditContext(context: EditContext): void {
    if (!editContextEq(this.context, context)) {
      this.reset()
      this.context = context
    }
  }
  hasArmedGesture(): boolean {
    return this.state.target !== null
  }
  onPointerMove(_snap: Snap | null, _ray: Ray): void {}
  setSelectionModifier(additive: boolean): void { this.additive = additive }
  snapConstraint(): SnapConstraint | null {
    return this.state.pickingFace ? { facesOnly: true } : null
  }
  selectFacePerimeter(): void {
    this.clearTimer()
    this.release()
    this.state.info = null
    this.state.acknowledged = false
    this.state.pickingFace = true
    this.state.phase = 'select'
    this.state.error = null
    this.emit()
  }
  clearSelection(): void { this.reset() }
  onPointerDown(snap: Snap | null, _ray: Ray): void {
    this.clearTimer()
    this.release()
    this.state.info = null
    this.state.acknowledged = false
    // Stable native codes are translated by treatmentError.
    if (snap?.instance !== undefined || this.context.kind === 'instance') {
      // oxlint-disable-next-line lingui/no-unlocalized-strings -- stable native error code
      this.fail('UnsupportedDefinitionMember:')
      return
    }
    if (this.context.kind === 'group') {
      // oxlint-disable-next-line lingui/no-unlocalized-strings -- stable native error code
      this.fail('ActiveEditingSession:')
      return
    }
    const face = this.state.pickingFace
    if (snap?.object === undefined || snap.element === undefined || snap.elementKind !== (face ? 'face' : 'edge')) {
      this.state.error = face ? t`Click a rectangular solid face.` : t`Click the middle of a straight solid edge.`
      this.state.phase = 'error'
      this.emit()
      return
    }
    if (!this.eligible(snap.object, snap.instance)) {
      // oxlint-disable-next-line lingui/no-unlocalized-strings -- stable native error code
      this.fail('UnsupportedSelectionContext:')
      return
    }
    try {
      let edges: bigint[]
      if (face) {
        edges = Array.from(this.scene.edge_treatment_face_edges(snap.object, snap.element))
      } else {
        const target = this.state.target
        if (this.additive && target && target.object !== snap.object) {
          this.state.error = t`Add edges from the same solid, or click without Ctrl/Command to start a new selection.`
          this.state.phase = 'error'
          this.emit()
          return
        }
        edges = this.additive && target ? [...(target.edges?.map(BigInt) ?? [target.edge])] : []
        const at = edges.indexOf(snap.element)
        if (at >= 0) edges.splice(at, 1)
        else edges.push(snap.element)
      }
      edges.sort((a, b) => a < b ? -1 : a > b ? 1 : 0)
      // Resolve every endpoint before replacing the selection or highlights.
      const positions = edges.flatMap(edge => {
        const endpoints = this.scene.edge_endpoints(snap.object!, edge)
        if (!endpoints) throw new Error('UnknownEdge:')
        return Array.from(endpoints)
      })
      clearPreview(this.edgeGroup)
      this.state.pickingFace = false
      this.state.target = edges.length ? { object: snap.object, edge: edges[0], edges: edges.map(String) } : null
      if (positions.length) {
        const geometry = new THREE.BufferGeometry().setAttribute(
          'position', new THREE.Float32BufferAttribute(positions, 3),
        )
        const line = new THREE.LineSegments(geometry, new THREE.LineBasicMaterial({ color: 0xffb74d, depthTest: false }))
        line.renderOrder = 999
        this.edgeGroup.add(line)
      }
      this.requestPreview()
    } catch (error) { this.fail(error) }
  }
  configure(values: Partial<Pick<TreatmentState, 'mode' | 'distance' | 'radius' | 'chord'>>): void {
    for (const field of ['distance', 'radius', 'chord'] as const) {
      if (values[field] !== undefined) delete this.canonical[field]
    }
    Object.assign(this.state, values)
    this.requestPreview()
  }
  /** Slider writes physical meters directly; display precision never feeds back. */
  setDimensionMeters(value: number): void {
    if (!Number.isFinite(value) || value <= 0) return
    const field = this.state.mode === 'chamfer' ? 'distance' : 'radius'
    this.canonical[field] = value
    this.state[field] = formatTreatmentInput(value, this.state.unit)
    this.requestPreview()
  }
  acknowledge(value: boolean): void {
    this.state.acknowledged = value
    this.emit()
  }
  private emit(): void {
    const input = this.input()
    this.state.dimensionMeters = input ? (this.state.mode === 'chamfer' ? input.distance : input.radius) : undefined
    if (this.active) this.changed({ ...this.state })
  }
  private clearTimer(): void {
    if (this.timer !== null) clearTimeout(this.timer)
    this.timer = null
  }
  private release(): void {
    if (this.handle !== null) {
      try {
        this.scene.discard_edge_treatment(this.handle)
      } catch {
        /* load/commit may already have retired it */
      }
      this.handle = null
    }
    clearPreview(this.meshGroup)
  }
  private input(): { distance: number; radius: number; chord: number } | null {
    const parse = (field: 'distance' | 'radius' | 'chord') =>
      this.canonical[field] ?? parseLengthToMeters(this.state[field], this.state.unit)
    const distance = parse('distance'),
      radius = parse('radius'),
      chord = parse('chord')
    const valid = (n: number | null): n is number => n !== null && Number.isFinite(n) && n > 0
    if (this.state.mode === 'chamfer' ? !valid(distance) : !valid(radius) || !valid(chord))
      return null
    return { distance: distance ?? 0, radius: radius ?? 0, chord: chord ?? 0 }
  }
  requestPreview(): void {
    this.clearTimer()
    this.release()
    this.state.info = null
    this.state.acknowledged = false
    this.state.error = null
    const target = this.state.target
    const key = target ? JSON.stringify([String(target.object), target.edges ?? [String(target.edge)], this.state.mode, this.input()?.chord]) : null
    if (this.limitsKey !== key) {
      this.limitsKey = null
      this.state.limits = null
    }
    if (!this.input()) {
      this.release()
      this.state.phase = 'error'
      this.state.error = t`Enter finite positive lengths, with an optional unit suffix.`
      this.emit()
      return
    }
    if (!this.state.target || this.state.pickingFace) {
      this.state.phase = 'select'
      this.emit()
      return
    }
    this.state.phase = 'pending'
    this.emit()
    if (!this.state.manualPreview) this.timer = setTimeout(() => this.generate(), 180)
  }
  generate(): void {
    this.clearTimer()
    if (!this.active || !this.state.target || this.state.pickingFace) return
    const started = performance.now()
    const input = this.input()
    if (!input) {
      this.requestPreview()
      return
    }
    const { object, edge } = this.state.target
    const edges = this.state.target.edges?.map(BigInt) ?? [edge]
    this.state.info = null
    this.state.acknowledged = false
    this.state.phase = 'pending'
    this.state.error = null
    clearPreview(this.meshGroup)
    this.emit()
    try {
      if (this.limitsKey === null) {
        this.limitsKey = JSON.stringify([String(object), edges.map(String), this.state.mode, input.chord])
        try {
          const limits = JSON.parse(this.scene.edge_treatment_limits_json(object, BigUint64Array.from(edges), this.state.mode === 'roundover', input.chord))
          if (limits.certainty === 'conservative' && Number.isFinite(limits.maximum) && limits.maximum > 0)
            this.state.limits = limits
        } catch { this.state.limits = null } // Text entry still uses full native validation.
      }
      const handle =
        this.state.mode === 'chamfer'
          ? edges.length === 1
            ? this.scene.prepare_chamfer(object, edge, input.distance)
            : this.scene.prepare_chamfer_edges(object, BigUint64Array.from(edges), input.distance)
          : edges.length === 1
            ? this.scene.prepare_roundover(object, edge, input.radius, input.chord)
            : this.scene.prepare_roundover_edges(object, BigUint64Array.from(edges), input.radius, input.chord)
      // Success retires the old Scene slot; never discard that old token.
      this.handle = handle
      const info = JSON.parse(this.scene.edge_treatment_info_json(handle)) as TreatmentInfo
      const dimensionsMatch =
        this.state.mode === 'chamfer'
          ? info.operation.distance === input.distance
          : info.operation.radius === input.radius && info.operation.max_chord_error === input.chord
      if (
        !dimensionsMatch ||
        (edges.length > 1 && JSON.stringify(info.edges) !== JSON.stringify(edges.map(String))) ||
        info.handle !== handle ||
        info.object !== String(object) ||
        info.edge !== String(edge) ||
        info.operation.kind !== this.state.mode
      )
        throw new Error('InvalidCandidateHandle: mismatched preview')
      const mesh = this.scene.edge_treatment_mesh(handle)
      const resources: (THREE.BufferGeometry | THREE.Material)[] = []
      let attached = false
      try {
        const positions = mesh.positions(),
          normals = mesh.normals(),
          indices = mesh.indices(),
          edgePositions = mesh.edge_positions()
        const geometry = new THREE.BufferGeometry()
        resources.push(geometry)
        geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3))
        geometry.setAttribute('normal', new THREE.BufferAttribute(normals, 3))
        geometry.setIndex(new THREE.BufferAttribute(indices, 1))
        const material = new THREE.MeshBasicMaterial({
          color: 0x26c6da,
          transparent: true,
          opacity: 0.5,
          depthWrite: false,
          depthTest: false,
          side: THREE.DoubleSide,
        })
        resources.push(material)
        const candidate = new THREE.Mesh(geometry, material)
        candidate.renderOrder = 997
        const edges = new THREE.BufferGeometry()
        resources.push(edges)
        edges.setAttribute('position', new THREE.BufferAttribute(edgePositions, 3))
        const wireMaterial = new THREE.LineBasicMaterial({ color: 0x00e5ff, depthTest: false })
        resources.push(wireMaterial)
        const wire = new THREE.LineSegments(edges, wireMaterial)
        wire.renderOrder = 998
        // Publish only after every buffer/material is ready. Until then,
        // the local list owns even allocations not yet attached to a Mesh.
        this.meshGroup.add(candidate, wire)
        attached = true
      } finally {
        try {
          if (!attached) {
            this.meshGroup.clear()
            for (const resource of resources) resource.dispose()
          }
        } finally {
          mesh.free()
        }
      }
      // Coalescing cannot interrupt synchronous WASM. After a slow evaluation,
      // require explicit Preview rather than repeat blocking work on every drag.
      // A resumable/worker evaluator remains necessary for fully live heavy jobs.
      if (performance.now() - started > 100) this.state.manualPreview = true
      this.state.info = info
      this.state.phase = 'ready'
      this.state.error = null
      this.emit()
    } catch (error) {
      this.release()
      this.fail(error)
    }
  }
  apply(): void {
    if (
      this.state.phase !== 'ready' ||
      this.handle === null ||
      !this.state.target ||
      !this.state.info ||
      this.state.info.handle !== this.handle
    )
      return
    if (this.state.info.warnings.length && !this.state.acknowledged) return
    const handle = this.handle,
      object = this.state.target.object
    this.handle = null // Matching attempts consume the native candidate, even on failure.
    let change: DocChangeJs
    try {
      change = this.scene.commit_edge_treatment(handle)
    } catch (error) {
      this.reset()
      this.fail(error)
      return
    }
    // Native geometry/history/recording have committed. A later cleanup or
    // renderer exception must not be presented as a refused model operation.
    this.state.target = null
    this.state.info = null
    this.state.acknowledged = false
    let refreshFailed = false
    try {
      try {
        this.reset()
      } catch {
        refreshFailed = true
      }
      try {
        this.committed(object, change)
      } catch {
        refreshFailed = true
      }
    } finally {
      change.free()
    }
    if (refreshFailed) {
      this.state.phase = 'error'
      this.state.error = t`The treatment was applied, but the view could not be refreshed. Use Undo or reopen the document before continuing.`
      this.emit()
    }
  }
  private fail(error: unknown): void {
    this.state.info = null
    this.state.phase = 'error'
    this.state.error = treatmentError(error)
    this.emit()
  }
  documentChanged(): void {
    if (this.state.target) {
      this.reset()
      // oxlint-disable-next-line lingui/no-unlocalized-strings -- stable API error code
      this.fail('StaleCandidate:')
    }
  }
  private reset(): void {
    this.clearTimer()
    this.release()
    clearPreview(this.edgeGroup)
    this.state.target = null
    this.state.limits = null
    this.limitsKey = null
    this.state.pickingFace = false
    this.state.info = null
    this.state.phase = 'select'
    this.state.error = null
    this.state.acknowledged = false
    this.emit()
  }
  dismiss(): void {
    this.reset()
    this.exit()
  }
  onKey(event: KeyboardEvent): void {
    if (event.key === 'Escape') this.dismiss()
    else if (event.key === 'Enter') this.apply()
  }
  cancel(): void {
    this.reset()
    this.unsubscribe?.()
    this.unsubscribe = null
    this.active = false
    this.root.removeFromParent()
    this.changed(null)
  }
}
