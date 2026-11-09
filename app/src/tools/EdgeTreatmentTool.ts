import * as THREE from 'three'
import { t } from '@lingui/core/macro'
import { parseKernelErrorCode } from '../kernelErrors'
import type { Scene, DocChangeJs } from '../wasm/loader'
import type { Tool, Snap, EditContext } from './types'
import { editContextEq } from './types'
import type { Ray } from '../viewport/math'
import { clearPreview, markSessionGhost } from './transformPreview'
import {
  formatLengthIn,
  getLengthUnit,
  parseLengthToMeters,
  subscribe,
  type LengthFormat,
} from '../settings/units'

export type TreatmentMode = 'chamfer' | 'roundover'
export interface TreatmentInfo {
  handle: number
  object: string
  edge: string
  operation: { kind: TreatmentMode; distance?: number; radius?: number; max_chord_error?: number }
  warnings: { code: string; annotations: { annotation: string; sid: string }[] }[]
}
export interface TreatmentState {
  mode: TreatmentMode
  distance: string
  radius: string
  chord: string
  unit: LengthFormat
  target: { object: bigint; edge: bigint } | null
  phase: 'select' | 'pending' | 'ready' | 'error'
  error: string | null
  info: TreatmentInfo | null
  acknowledged: boolean
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
      // Preserve values exactly when display units change, including values
      // beyond display precision. Explicit meters round-trip the same f64.
      const old = this.state.unit
      for (const field of ['distance', 'radius', 'chord'] as const) {
        const value = parseLengthToMeters(this.state[field], old)
        if (value !== null) this.state[field] = `${value} m`
      }
      this.state.unit = unit
      this.requestPreview()
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
  onPointerDown(snap: Snap | null, _ray: Ray): void {
    this.reset()
    // oxlint-disable lingui/no-unlocalized-strings -- stable API error codes, never displayed
    if (snap?.instance !== undefined || this.context.kind === 'instance') {
      this.fail('UnsupportedDefinitionMember:')
      return
    }
    if (this.context.kind === 'group') {
      this.fail('ActiveEditingSession:')
      return
    }
    if (snap?.object === undefined || snap.element === undefined || snap.elementKind !== 'edge') {
      this.state.error = t`Click the middle of a straight solid edge.`
      this.state.phase = 'error'
      this.emit()
      return
    }
    if (!this.eligible(snap.object, snap.instance)) {
      this.fail('UnsupportedSelectionContext:')
      return
    }
    // oxlint-enable lingui/no-unlocalized-strings
    const endpoints = this.scene.edge_endpoints(snap.object, snap.element)
    if (!endpoints) {
      // oxlint-disable-next-line lingui/no-unlocalized-strings -- stable API error code
      this.fail('UnknownEdge:')
      return
    }
    this.state.target = { object: snap.object, edge: snap.element }
    const geometry = new THREE.BufferGeometry().setAttribute(
      'position',
      new THREE.Float32BufferAttribute(endpoints, 3),
    )
    const line = new THREE.LineSegments(
      geometry,
      new THREE.LineBasicMaterial({ color: 0xffb74d, depthTest: false }),
    )
    line.renderOrder = 999
    this.edgeGroup.add(line)
    this.requestPreview()
  }
  configure(values: Partial<Pick<TreatmentState, 'mode' | 'distance' | 'radius' | 'chord'>>): void {
    Object.assign(this.state, values)
    this.requestPreview()
  }
  acknowledge(value: boolean): void {
    this.state.acknowledged = value
    this.emit()
  }
  private emit(): void {
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
    const parse = (value: string) => parseLengthToMeters(value, this.state.unit)
    const distance = parse(this.state.distance),
      radius = parse(this.state.radius),
      chord = parse(this.state.chord)
    const valid = (n: number | null): n is number => n !== null && Number.isFinite(n) && n > 0
    if (this.state.mode === 'chamfer' ? !valid(distance) : !valid(radius) || !valid(chord))
      return null
    return { distance: distance ?? 0, radius: radius ?? 0, chord: chord ?? 0 }
  }
  requestPreview(): void {
    this.clearTimer()
    clearPreview(this.meshGroup)
    this.state.info = null
    this.state.acknowledged = false
    this.state.error = null
    if (!this.input()) {
      this.release()
      this.state.phase = 'error'
      this.state.error = t`Enter finite positive lengths, with an optional unit suffix.`
      this.emit()
      return
    }
    if (!this.state.target) {
      this.state.phase = 'select'
      this.emit()
      return
    }
    this.state.phase = 'pending'
    this.emit()
    this.timer = setTimeout(() => this.generate(), 180)
  }
  generate(): void {
    this.clearTimer()
    if (!this.active || !this.state.target) return
    const input = this.input()
    if (!input) {
      this.requestPreview()
      return
    }
    const { object, edge } = this.state.target
    this.state.info = null
    this.state.acknowledged = false
    this.state.phase = 'pending'
    this.state.error = null
    clearPreview(this.meshGroup)
    this.emit()
    try {
      const handle =
        this.state.mode === 'chamfer'
          ? this.scene.prepare_chamfer(object, edge, input.distance)
          : this.scene.prepare_roundover(object, edge, input.radius, input.chord)
      // Success retires the old Scene slot; never discard that old token.
      this.handle = handle
      const info = JSON.parse(this.scene.edge_treatment_info_json(handle)) as TreatmentInfo
      const dimensionsMatch =
        this.state.mode === 'chamfer'
          ? info.operation.distance === input.distance
          : info.operation.radius === input.radius && info.operation.max_chord_error === input.chord
      if (
        !dimensionsMatch ||
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
