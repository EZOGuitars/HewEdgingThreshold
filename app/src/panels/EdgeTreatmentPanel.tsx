import type { CSSProperties } from 'react'
import { t } from '@lingui/core/macro'
import type { EdgeTreatmentTool, TreatmentState } from '../tools/EdgeTreatmentTool'
import { getLengthUnitSuffix } from '../settings/units'

const control: CSSProperties = {
  font: 'inherit',
  padding: '5px 8px',
  borderRadius: 4,
  border: '1px solid var(--border-default)',
  color: 'var(--text-primary)',
  background: 'var(--surface-input, var(--surface-panel))',
}
const field: CSSProperties = { display: 'grid', gap: 4, marginBottom: 10 }

export function EdgeTreatmentPanel({
  tool,
  state,
}: {
  tool: EdgeTreatmentTool
  state: TreatmentState
}) {
  const unit = getLengthUnitSuffix(state.unit)
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
    >
      <h3 style={{ marginTop: 0 }}>{t`Edge Treatment`}</h3>
      <p>{t`Select a straight convex solid edge. Curved and concave edges are not supported.`}</p>
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
          <label style={field}>
            {t`Distance`}{' '}
            <input
              style={control}
              value={state.distance}
              onChange={(e) => tool.configure({ distance: e.target.value })}
            />
          </label>
        ) : (
          <>
            <label style={field}>
              {t`Radius`}{' '}
              <input
                style={control}
                value={state.radius}
                onChange={(e) => tool.configure({ radius: e.target.value })}
              />
            </label>
            <label style={field}>
              {t`Maximum chord error`}{' '}
              <input
                style={control}
                value={state.chord}
                onChange={(e) => tool.configure({ chord: e.target.value })}
              />
            </label>
          </>
        )}
        <p role="status">
          {state.phase === 'select'
            ? t`Select an edge to preview.`
            : state.phase === 'pending'
              ? t`Preparing preview…`
              : state.phase === 'ready'
                ? t`Candidate shown in cyan. The original remains unchanged until Apply.`
                : ''}
        </p>
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
