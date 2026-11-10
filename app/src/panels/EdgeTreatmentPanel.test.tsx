import { it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { EdgeTreatmentPanel } from './EdgeTreatmentPanel'
import type { EdgeTreatmentTool, TreatmentState } from '../tools/EdgeTreatmentTool'
const state: TreatmentState = {
  mode: 'chamfer',
  distance: '5 mm',
  radius: '5 mm',
  chord: '0.1 mm',
  unit: 'mm',
  target: { object: 1n, edge: 2n },
  phase: 'ready',
  error: null,
  info: { handle: 1, object: '1', edge: '2', operation: { kind: 'chamfer' }, warnings: [] },
  acknowledged: false,
}
const tool = () => ({
  configure: vi.fn(),
  generate: vi.fn(),
  apply: vi.fn(),
  dismiss: vi.fn(),
  acknowledge: vi.fn(),
})
it('renders native parameter controls and forwards changes, Apply and Cancel', () => {
  const t = tool()
  render(<EdgeTreatmentPanel tool={t as unknown as EdgeTreatmentTool} state={state} />)
  fireEvent.change(screen.getByLabelText('Distance'), { target: { value: '0.125 in' } })
  expect(t.configure).toHaveBeenCalledWith({ distance: '0.125 in' })
  fireEvent.change(screen.getByLabelText('Operation'), { target: { value: 'roundover' } })
  expect(t.configure).toHaveBeenCalledWith({ mode: 'roundover' })
  fireEvent.click(screen.getByRole('button', { name: 'Apply' }))
  expect(t.apply).toHaveBeenCalledOnce()
  fireEvent.click(screen.getByRole('button', { name: 'Cancel' }))
  expect(t.dismiss).toHaveBeenCalledOnce()
})
it('discloses annotation identities and blocks Apply until acknowledged; pending/error also block it', () => {
  const t = tool()
  const warned = {
    ...state,
    info: {
      ...state.info!,
      warnings: [{ code: 'AnnotationDetachment', annotations: [{ sid: '42', annotation: '4' }] }],
    },
  }
  const { rerender } = render(
    <EdgeTreatmentPanel tool={t as unknown as EdgeTreatmentTool} state={warned} />,
  )
  expect(screen.getByRole('note')).toHaveTextContent('42')
  expect(screen.getByRole('note')).toHaveTextContent('Undo')
  expect(screen.getByRole('button', { name: 'Apply' })).toBeDisabled()
  fireEvent.click(screen.getByRole('checkbox'))
  expect(t.acknowledge).toHaveBeenCalledWith(true)
  rerender(
    <EdgeTreatmentPanel
      tool={t as unknown as EdgeTreatmentTool}
      state={{ ...warned, acknowledged: true }}
    />,
  )
  expect(screen.getByRole('button', { name: 'Apply' })).toBeEnabled()
  rerender(
    <EdgeTreatmentPanel
      tool={t as unknown as EdgeTreatmentTool}
      state={{ ...state, phase: 'pending' }}
    />,
  )
  expect(screen.getByRole('button', { name: 'Apply' })).toBeDisabled()
})
it('roundover exposes both radius and chord tolerance', () => {
  const t = tool()
  render(
    <EdgeTreatmentPanel
      tool={t as unknown as EdgeTreatmentTool}
      state={{ ...state, mode: 'roundover' }}
    />,
  )
  expect(screen.getByLabelText('Radius')).toBeInTheDocument()
  fireEvent.change(screen.getByLabelText('Maximum chord error'), { target: { value: 'bad' } })
  expect(t.configure).toHaveBeenCalledWith({ chord: 'bad' })
})

it('offers operation-specific help on hover, focus and click without applying changes', () => {
  const t = tool()
  const { rerender } = render(<EdgeTreatmentPanel tool={t as unknown as EdgeTreatmentTool} state={state} />)
  const chamfer = screen.getByRole('button', { name: 'About Chamfer distance' })
  expect(screen.queryByRole('tooltip')).not.toBeInTheDocument()
  fireEvent.mouseEnter(chamfer)
  expect(screen.getByRole('tooltip')).toHaveTextContent('0.354 inches')
  expect(chamfer).toHaveAttribute('aria-describedby', screen.getByRole('tooltip').id)
  fireEvent.mouseLeave(chamfer)
  expect(screen.queryByRole('tooltip')).not.toBeInTheDocument()
  fireEvent.focus(chamfer)
  expect(screen.getByRole('tooltip')).toHaveTextContent('45°')
  fireEvent.keyDown(chamfer, { key: 'Escape' })
  expect(screen.queryByRole('tooltip')).not.toBeInTheDocument()
  fireEvent.click(chamfer)
  expect(screen.getByRole('tooltip')).toBeInTheDocument()
  fireEvent.click(chamfer)
  expect(screen.queryByRole('tooltip')).not.toBeInTheDocument()
  rerender(<EdgeTreatmentPanel tool={t as unknown as EdgeTreatmentTool} state={{ ...state, mode: 'roundover' }} />)
  expect(screen.queryByRole('button', { name: 'About Chamfer distance' })).not.toBeInTheDocument()
  fireEvent.focus(screen.getByRole('button', { name: 'About Roundover radius' }))
  expect(screen.getByRole('tooltip')).toHaveTextContent("arc's center")
  fireEvent.blur(screen.getByRole('button', { name: 'About Roundover radius' }))
  const chord = screen.getByRole('button', { name: 'About Maximum chord error' })
  fireEvent.focus(chord)
  expect(screen.getByRole('tooltip')).toHaveTextContent('roundovers, not chamfers')
  expect(screen.getByRole('tooltip')).toHaveTextContent("model's current units")
  expect(t.apply).not.toHaveBeenCalled()
  expect(t.configure).not.toHaveBeenCalled()
})
