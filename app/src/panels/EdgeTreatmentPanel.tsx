import { useId, useRef, useState, type CSSProperties } from 'react'
import infoSvg from '@material-symbols/svg-400/outlined/info.svg?raw'
import { InlineIcon } from './InlineIcon'
import { t } from '@lingui/core/macro'
import type { EdgeTreatmentTool, TreatmentState } from '../tools/EdgeTreatmentTool'
import { formatLengthIn, getLengthUnitSuffix } from '../settings/units'

const control: CSSProperties = {
  font: 'inherit',
  padding: '5px 8px',
  borderRadius: 4,
  border: '1px solid var(--border-default)',
  color: 'var(--text-primary)',
  background: 'var(--surface-input, var(--surface-panel))',
}
const field: CSSProperties = { display: 'grid', gap: 4, marginBottom: 10 }

/** Panel-local help: focus/hover for desktop, toggle for touch, Escape to dismiss. */
function ParameterHelp({ label, paragraphs }: { label: string; paragraphs: string[] }) {
  const id = useId()
  const [open, setOpen] = useState(false)
  const [pinned, setPinned] = useState(false)
  const anchor = useRef<HTMLSpanElement>(null)
  const [position, setPosition] = useState({ left: 0, top: 0 })
  const show = () => {
    const row = anchor.current?.parentElement?.getBoundingClientRect()
    if (row) {
      const width = Math.min(280, window.innerWidth - 32)
      // Prefer beside the entire label row, so long help never covers the
      // next parameter's input/info button. Clamp inside narrow viewports.
      const left = row.right + width + 8 <= window.innerWidth - 16
        ? row.right + 8 : Math.max(16, row.left - width - 8)
      setPosition({ left, top: Math.max(16, Math.min(row.top, window.innerHeight - 436)) })
    }
    setOpen(true)
  }
  return (
    <span
      ref={anchor}
      style={{ display: 'inline-flex' }}
      onMouseEnter={show}
      onMouseLeave={(event) => {
        if (!pinned && !event.currentTarget.contains(document.activeElement)) setOpen(false)
      }}
    >
      <button
        type="button"
        aria-label={t`About ${label}`}
        aria-describedby={open ? id : undefined}
        aria-expanded={open}
        style={{ ...control, display: 'inline-flex', padding: 3, cursor: 'help' }}
        onFocus={show}
        onBlur={() => { setOpen(false); setPinned(false) }}
        onClick={() => { setOpen(!pinned); setPinned(!pinned) }}
        onKeyDown={(event) => {
          event.stopPropagation()
          if (event.key === 'Escape') {
            event.preventDefault()
            setOpen(false)
            setPinned(false)
          }
        }}
        onKeyUp={(event) => event.stopPropagation()}
      >
        {/* Keep pointer targeting on the stable button: InlineIcon replaces raw
            SVG contents during rendering, which can synthesize hover re-entry. */}
        <span style={{ pointerEvents: 'none' }}><InlineIcon svg={infoSvg} /></span>
      </button>
      {open && (
        <span
          id={id}
          role="tooltip"
          style={{
            position: 'fixed', ...position, zIndex: 30,
            width: 'min(280px, calc(100vw - 32px))',
            maxHeight: 'min(420px, calc(100vh - 32px))',
            overflowY: 'auto', boxSizing: 'border-box',
            padding: 12, borderRadius: 6, lineHeight: 1.45,
            background: 'var(--surface-panel, #252932)',
            color: 'var(--text-primary, #eee)',
            border: '1px solid var(--border-default)',
            boxShadow: '0 2px 12px #0006', fontWeight: 'normal',
          }}
        >
          {paragraphs.map((paragraph, i) => (
            <span key={paragraph} style={{ display: 'block', marginTop: i ? 8 : 0 }}>{paragraph}</span>
          ))}
        </span>
      )}
    </span>
  )
}

export function EdgeTreatmentPanel({
  tool,
  state,
}: {
  tool: EdgeTreatmentTool
  state: TreatmentState
}) {
  const fieldId = useId()
  const unit = getLengthUnitSuffix(state.unit)
  const selectedCount = state.target ? (state.target.edges?.length ?? 1) : 0
  const sliderValue = state.limits ? Math.max(state.limits.maximum / 1000, Math.min(state.dimensionMeters ?? 0, state.limits.maximum)) : 0
  const boundLabel = state.limits ? formatLengthIn(state.limits.maximum, state.unit, 'exact') : ''
  const segments = state.info?.segments
  const warnings = state.info?.warnings ?? []
  return (
    <section
      aria-label={t`Edge Treatment`}
      style={{
        position: 'absolute',
        top: 56,
        left: 12,
        zIndex: 20,
        width: 280,
        padding: 14,
        fontSize: 13,
        borderRadius: 8,
        background: 'var(--surface-panel, #252932)',
        color: 'var(--text-primary, #eee)',
        boxShadow: '0 2px 12px #0006',
      }}
      onPointerDown={(e) => e.stopPropagation()}
      onKeyDown={(event) => {
        // Keep native button/select activation out of viewport Enter/Space
        // shortcuts. Defaults still click/submit; Ctrl+Z and Escape can bubble.
        if ((event.target instanceof HTMLButtonElement && (event.key === 'Enter' || event.key === ' ')) ||
            event.target instanceof HTMLSelectElement) event.stopPropagation()
      }}
    >
      <h3 style={{ marginTop: 0 }}>{t`Edge Treatment`}</h3>
      <p>{t`Click an edge to replace the selection. Ctrl/Command-click adds or removes edges on the same solid.`}</p>
      <p>{t`For a complete polygonal or circular cap, choose Select face perimeter, then click the cap.`}</p>
      {state.mode === 'roundover' && <p>{t`Roundovers are faceted. Connected box edges use miter seams or faceted spherical corners.`}</p>}
      <div style={{ display: 'flex', gap: 8 }}>
        <button style={control} type="button" onClick={() => tool.selectFacePerimeter()}>{t`Select face perimeter`}</button>
        <button style={control} type="button" onClick={() => tool.clearSelection()}>{t`Clear`}</button>
      </div>
      <p>{t`Selected edges: ${selectedCount}`}</p>
      <form
        onSubmit={(event) => {
          event.preventDefault()
          tool.apply()
        }}
      >
        <label style={field}>
          {t`Operation`}{' '}
          <select
            style={control}
            value={state.mode}
            onChange={(e) => tool.configure({ mode: e.target.value as TreatmentState['mode'] })}
          >
            <option value="chamfer">{t`Chamfer`}</option>
            <option value="roundover">{t`Roundover`}</option>
          </select>
        </label>
        <p>{t`Values without a unit use ${unit}. Explicit units are accepted.`}</p>
        {state.mode === 'chamfer' ? (
          <div style={field}>
            <span style={{ position: 'relative', display: 'flex', alignItems: 'center', gap: 6 }}>
              <label htmlFor={`${fieldId}-distance`}>{t`Distance`}</label>
              <ParameterHelp label={t`Chamfer distance`} paragraphs={[
                t`Chamfer distance controls how far the bevel extends from the original sharp edge along each adjacent face.`,
                t`For a 90° corner, an equal-distance chamfer removes the corner and replaces it with a flat 45° beveled surface.`,
                t`For example, a 0.25-inch chamfer extends 0.25 inches along each adjoining face. The sloped bevel itself is approximately 0.354 inches wide.`,
              ]} />
            </span>
            <input id={`${fieldId}-distance`}
              style={control}
              value={state.distance}
              onChange={(e) => tool.configure({ distance: e.target.value })}
            />
          </div>
        ) : (
          <>
            <div style={field}>
              <span style={{ position: 'relative', display: 'flex', alignItems: 'center', gap: 6 }}>
                <label htmlFor={`${fieldId}-radius`}>{t`Radius`}</label>
                <ParameterHelp label={t`Roundover radius`} paragraphs={[
                  t`Radius determines the size of the rounded corner.`,
                  t`It is the radius of the ideal circular arc used to replace the sharp edge, measured from the arc's center.`,
                  t`A larger radius removes more material and produces a broader curve. A smaller radius creates a tighter roundover.`,
                  t`For a 90° corner, the radius also equals the distance from the original sharp edge to where the roundover meets each adjacent face.`,
                ]} />
              </span>
              <input id={`${fieldId}-radius`}
                style={control}
                value={state.radius}
                onChange={(e) => tool.configure({ radius: e.target.value })}
              />
            </div>
            <div style={field}>
              <span style={{ position: 'relative', display: 'flex', alignItems: 'center', gap: 6 }}>
                <label htmlFor={`${fieldId}-chord`}>{t`Maximum chord error`}</label>
                <ParameterHelp label={t`Maximum chord error`} paragraphs={[
                  t`Maximum chord error controls how closely the flat segments of a roundover approximate a perfect circular curve.`,
                  t`Smaller values produce more segments and smoother geometry, but increase mesh complexity.`,
                  t`Larger values produce fewer segments and a more visibly faceted result.`,
                  t`The value is the maximum permitted deviation between the ideal circular arc and its faceted approximation, expressed in the model's current units.`,
                  t`This setting applies to roundovers, not chamfers.`,
                ]} />
              </span>
              <input id={`${fieldId}-chord`}
                style={control}
                value={state.chord}
                onChange={(e) => tool.configure({ chord: e.target.value })}
              />
            </div>
          </>
        )}
        {state.limits && <div style={field}>
          <label htmlFor={`${fieldId}-size-slider`}>{state.mode === 'chamfer' ? t`Adjust chamfer distance` : t`Adjust roundover radius`}</label>
          <input id={`${fieldId}-size-slider`} type="range" min={state.limits.maximum / 1000}
            max={state.limits.maximum} step={state.limits.maximum / 1000}
            value={sliderValue}
            aria-valuetext={formatLengthIn(sliderValue, state.unit, 'exact')}
            aria-describedby={`${fieldId}-range-help`}
            onChange={(event) => tool.setDimensionMeters(Number(event.target.value))}
            onKeyDown={(event) => event.stopPropagation()} />
          <small id={`${fieldId}-range-help`}>{t`Conservative slider bound: ${boundLabel}. Exact input remains independent; every preview is validated.`}</small>
        </div>}
        <p role="status">
          {state.phase === 'select'
            ? state.pickingFace ? t`Click a supported planar cap face to select its perimeter.` : t`Select an edge to preview.`
            : state.phase === 'pending'
              ? state.manualPreview ? t`Parameters changed. Press Preview to evaluate this expensive operation.` : t`Preparing preview…`
              : state.phase === 'ready'
                ? t`Candidate shown in cyan. The original remains unchanged until Apply.`
                : ''}
        </p>
        {state.mode === 'roundover' && state.phase === 'ready' && segments !== undefined && (
          <p>{t`Segments per quarter arc: ${segments}`}</p>
        )}
        {state.error && <p role="alert">{state.error}</p>}
        {warnings.map((warning, i) => {
          const annotations = warning.annotations.map((a) => a.sid ?? a.annotation).join(', ')
          return (
            <div key={i} role="note">
              <p>{t`Annotations ${annotations} will lose their source attachments. Their recorded points remain. Undo restores the attachments.`}</p>
            </div>
          )
        })}
        {warnings.length > 0 && (
          <label style={field}>
            <input
              type="checkbox"
              checked={state.acknowledged}
              onChange={(e) => tool.acknowledge(e.target.checked)}
            />
            {t`I understand the annotation attachments will be removed.`}
          </label>
        )}
        <div style={{ display: 'flex', gap: 8, marginTop: 12 }}>
          <button
            style={control}
            type="button"
            onClick={() => tool.generate()}
            disabled={!state.target}
          >{t`Preview`}</button>
          <button
            style={control}
            type="submit"
            disabled={state.phase !== 'ready' || (warnings.length > 0 && !state.acknowledged)}
          >{t`Apply`}</button>
          <button style={control} type="button" onClick={() => tool.dismiss()}>{t`Cancel`}</button>
        </div>
      </form>
    </section>
  )
}
