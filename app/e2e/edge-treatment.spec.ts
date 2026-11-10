import { test, expect } from '@playwright/test'
import { inspectClosedStl } from './stlIntegrity'

declare global {
  interface Window {
    __hew_test?: import('../src/test/harness').HewTestHarness
  }
}

test('Edge Treatment: real pointer selection, preview, Apply, undo/redo, invalid input and Cancel', async ({
  page,
}, info) => {
  test.setTimeout(120_000)
  const errors: string[] = []
  page.on('pageerror', (e) => errors.push(e.message))
  await page.goto('/')
  await page.waitForFunction(() => window.__hew_test?.isReady() === true, null, { timeout: 30000 })
  const object = await page.evaluate(() => {
    const h = window.__hew_test!
    h.setLengthUnit('mm')
    const object = h.drawBox([0, 0, 0], [2, 3, 0], 4)
    h.setCamera({ position: [7, -8, 7], target: [1, 1.5, 2], up: [0, 0, 1], fovDeg: 45 })
    return object
  })
  await page.evaluate(() => window.__hew_test!.saveWithCameraState())
  await page.getByRole('radio', { name: 'Edge Treatment', exact: true }).click()
  const panel = page.getByRole('region', { name: 'Edge Treatment' })
  await expect(panel).toBeVisible()
  await panel.getByLabel('Distance', { exact: true }).fill('250 mm')
  const canvas = page.locator('canvas').first()
  const rect = await canvas.boundingBox()
  if (!rect) throw Error('missing canvas')
  const point = await page.evaluate(() => window.__hew_test!.worldToScreen([2, 0, 1.5]))
  await page.mouse.move(rect.x + point.x - 8, rect.y + point.y - 8)
  await page.mouse.move(rect.x + point.x, rect.y + point.y, { steps: 3 })
  await page.mouse.click(rect.x + point.x, rect.y + point.y)
  await expect(panel.getByRole('button', { name: 'Apply', exact: true })).toBeEnabled()
  // App recovery autosave persists camera state outside undo. Compare actual
  // committed geometry here; the Node smoke verifies canonical document bytes.
  const before = await page.evaluate(() => window.__hew_test!.exportStl(0)!.bytes)
  await panel.getByLabel('Operation').selectOption('roundover')
  await panel.getByLabel('Radius', { exact: true }).fill('250 mm')
  await panel.getByLabel('Maximum chord error', { exact: true }).fill('5 mm')
  await expect(panel.getByRole('button', { name: 'Apply', exact: true })).toBeEnabled()
  expect(await page.evaluate(() => window.__hew_test!.exportStl(0)!.bytes)).toEqual(before)
  await page.screenshot({ path: info.outputPath('edge-treatment-preview.png') })
  await panel.getByRole('button', { name: 'Apply', exact: true }).click()
  const after = await page.evaluate(() => window.__hew_test!.exportStl(0)!.bytes)
  expect(after).not.toEqual(before)
  expect(await page.evaluate(() => window.__hew_test!.getObjectIds())).toEqual([object])
  await expect(panel.getByRole('button', { name: 'Apply', exact: true })).toBeDisabled()
  await page.evaluate(() => window.__hew_test!.undo())
  expect(await page.evaluate(() => window.__hew_test!.exportStl(0)!.bytes)).toEqual(before)
  await page.evaluate(() => window.__hew_test!.redo())
  expect(await page.evaluate(() => window.__hew_test!.exportStl(0)!.bytes)).toEqual(after)
  await panel.getByLabel('Radius', { exact: true }).fill('-1')
  await expect(panel.getByRole('alert')).toBeVisible()
  await expect(panel.getByRole('button', { name: 'Apply', exact: true })).toBeDisabled()
  await panel.getByRole('button', { name: 'Cancel', exact: true }).click()
  await expect(panel).not.toBeVisible()
  expect(await page.evaluate(() => window.__hew_test!.exportStl(0)!.bytes)).toEqual(after)
  expect(errors).toEqual([])
})

test('a renderer refresh failure preserves the committed model, recording, and dirty state', async ({
  page,
}) => {
  test.setTimeout(120_000)
  await page.goto('/')
  await page.waitForFunction(() => window.__hew_test?.isReady() === true)
  await page.evaluate(() => {
    const h = window.__hew_test!
    h.setLengthUnit('mm')
    h.drawBox([0, 0, 0], [2, 3, 0], 4)
    h.setCamera({ position: [7, -8, 7], target: [1, 1.5, 2], up: [0, 0, 1], fovDeg: 45 })
    h.saveWithCameraState()
  })
  await page.getByRole('radio', { name: 'Edge Treatment', exact: true }).click()
  const panel = page.getByRole('region', { name: 'Edge Treatment' })
  await panel.getByLabel('Distance', { exact: true }).fill('250 mm')
  const rect = await page.locator('canvas').first().boundingBox()
  if (!rect) throw Error('missing canvas')
  const point = await page.evaluate(() => window.__hew_test!.worldToScreen([2, 0, 1.5]))
  await page.mouse.move(rect.x + point.x - 8, rect.y + point.y - 8)
  await page.mouse.move(rect.x + point.x, rect.y + point.y, { steps: 3 })
  await page.mouse.click(rect.x + point.x, rect.y + point.y)
  await expect(panel.getByRole('button', { name: 'Apply', exact: true })).toBeEnabled()
  const before = await page.evaluate(() => {
    window.__hew_test!.startRecording()
    return window.__hew_test!.exportStl(0)!.bytes
  })
  await page.evaluate(async () => {
    // Test-only fault injection at the renderer boundary, not in the model.
    const { SceneRenderer } = await import('/src/viewport/SceneRenderer.ts')
    const original = SceneRenderer.prototype.refreshTouched
    SceneRenderer.prototype.refreshTouched = function () {
      SceneRenderer.prototype.refreshTouched = original
      throw new Error('injected renderer refresh failure')
    }
  })
  await panel.getByRole('button', { name: 'Apply', exact: true }).click()
  await expect(panel.getByRole('alert')).toContainText('was applied')
  await expect(panel.getByRole('button', { name: 'Apply', exact: true })).toBeDisabled()
  expect(await page.evaluate(() => window.__hew_test!.exportStl(0)!.bytes)).not.toEqual(before)
  await expect(page.getByText('Edited just now', { exact: true })).toBeVisible()
  const recording = await page.evaluate(() => {
    window.__hew_test!.stopRecording()
    return window.__hew_test!.takeRecording()
  })
  expect((recording.match(/"method":"edge_treatment"/g) ?? []).length).toBe(1)
  await page.evaluate(() => window.__hew_test!.undo())
  expect(await page.evaluate(() => window.__hew_test!.exportStl(0)!.bytes)).toEqual(before)
})

test('metric roundover retains 6.35 mm across unit switches and exports measured dimensions', async ({
  page,
}) => {
  test.setTimeout(120_000)
  await page.goto('/')
  await page.waitForFunction(() => window.__hew_test?.isReady() === true)
  await page.evaluate(() => {
    const h = window.__hew_test!
    h.setLengthUnit('mm')
    h.drawBox([0, 0, 0], [0.3, 0.3, 0], 0.3)
    h.setCamera({
      position: [0.9, -0.9, 0.9],
      target: [0.15, 0.15, 0.15],
      up: [0, 0, 1],
      fovDeg: 45,
    })
  })
  await page.getByRole('radio', { name: 'Edge Treatment', exact: true }).click()
  const panel = page.getByRole('region', { name: 'Edge Treatment' })
  await panel.getByLabel('Operation').selectOption('roundover')
  await panel.getByLabel('Radius', { exact: true }).fill('6.35')
  await panel.getByLabel('Maximum chord error', { exact: true }).fill('0.01')
  const rect = await page.locator('canvas').first().boundingBox()
  if (!rect) throw Error('missing canvas')
  const p = await page.evaluate(() => window.__hew_test!.worldToScreen([0.3, 0, 0.15]))
  await page.mouse.move(rect.x + p.x - 8, rect.y + p.y - 8)
  await page.mouse.move(rect.x + p.x, rect.y + p.y, { steps: 3 })
  await page.mouse.click(rect.x + p.x, rect.y + p.y)
  await expect(panel.getByRole('button', { name: 'Apply', exact: true })).toBeEnabled()
  const before = await page.evaluate(() => window.__hew_test!.exportStl(0)!.bytes)
  await page.evaluate(() => window.__hew_test!.setLengthUnit('dec_in'))
  await expect(panel.getByLabel('Radius', { exact: true })).toHaveValue('0.25"')
  await page.evaluate(() => {
    for (let i = 0; i < 100; i++) {
      window.__hew_test!.setLengthUnit('mm')
      window.__hew_test!.setLengthUnit('dec_in')
    }
    window.__hew_test!.setLengthUnit('mm')
  })
  await expect(panel.getByLabel('Radius', { exact: true })).toHaveValue('6.35 mm')
  await expect(panel.getByLabel('Maximum chord error', { exact: true })).toHaveValue('0.01 mm')
  await expect(panel.getByRole('button', { name: 'Apply', exact: true })).toBeEnabled()
  expect(await page.evaluate(() => window.__hew_test!.exportStl(0)!.bytes)).toEqual(before)
  await panel.getByRole('button', { name: 'Apply', exact: true }).click()
  const measured = await page.evaluate(() => {
    const bytes = new Uint8Array(window.__hew_test!.exportStl(0)!.bytes)
    const view = new DataView(bytes.buffer)
    const points: number[][] = []
    for (let i = 0; i < view.getUint32(80, true); i++)
      for (let j = 0; j < 3; j++)
        points.push([0, 1, 2].map((k) => view.getFloat32(84 + 50 * i + 12 + 12 * j + 4 * k, true)))
    return {
      dimensions: [0, 1, 2].map(
        (k) => Math.max(...points.map((p) => p[k])) - Math.min(...points.map((p) => p[k])),
      ),
      x: 300 - Math.max(...points.filter((p) => Math.abs(p[1]) < 1e-6).map((p) => p[0])),
      y: Math.min(...points.filter((p) => Math.abs(p[0] - 300) < 1e-6).map((p) => p[1])),
    }
  })
  for (const dimension of measured.dimensions)
    expect(Math.abs(dimension - 300)).toBeLessThan(0.0003)
  expect(Math.abs(measured.x - 6.35)).toBeLessThan(0.0003)
  expect(Math.abs(measured.y - 6.35)).toBeLessThan(0.0003)
})

for (const mode of ['roundover', 'chamfer'] as const) {
  for (const count of [2, 3, 4]) {
    test(`connected ${mode}: ${count} edges, cancellation, one Apply and history`, async ({ page }) => {
      test.setTimeout(120_000)
      await page.goto('/')
      await page.waitForFunction(() => window.__hew_test?.isReady() === true)
      await page.evaluate(() => {
        const h = window.__hew_test!
        h.setLengthUnit('mm')
        h.drawBox([0,0,0], [2,3,0], 4)
        h.setCamera({ position: [7,-8,7], target: [1,1.5,2], up: [0,0,1], fovDeg: 45 })
        h.saveWithCameraState()
      })
      const before = await page.evaluate(() => window.__hew_test!.exportStl(0)!.bytes)
      await page.getByRole('radio', { name: 'Edge Treatment', exact: true }).click()
      const panel = page.getByRole('region', { name: 'Edge Treatment' })
      await panel.getByLabel('Operation').selectOption(mode)
      await panel.getByLabel(mode === 'roundover' ? 'Radius' : 'Distance', { exact: true }).fill('250 mm')
      if (mode === 'roundover') await panel.getByLabel('Maximum chord error', { exact: true }).fill('5 mm')
      const rect = await page.locator('canvas').first().boundingBox()
      if (!rect) throw Error('missing canvas')
      const pick = async (point: [number,number,number], additive = false) => {
        const p = await page.evaluate(point => window.__hew_test!.worldToScreen(point), point)
        await page.mouse.move(rect.x+p.x-10, rect.y+p.y-10)
        await page.mouse.move(rect.x+p.x, rect.y+p.y, { steps: 3 })
        if (additive) await page.keyboard.down('Control')
        try { await page.mouse.click(rect.x+p.x, rect.y+p.y) }
        finally { if (additive) await page.keyboard.up('Control') }
      }
      const select = async () => {
        if (count === 4) {
          await panel.getByRole('button', { name: 'Select face perimeter', exact: true }).click()
          await pick([1,1.5,4])
        } else {
          await pick([2,0,1.5])
          await pick([1,0,4], true)
          if (count === 3) await pick([2,1.5,4], true)
        }
        await expect(panel.getByText(`Selected edges: ${count}`, { exact: true })).toBeVisible()
        await expect(panel.getByRole('button', { name: 'Apply', exact: true })).toBeEnabled()
      }
      await select()
      if (count === 4) {
        // Keyboard activation must cancel the old candidate, not trigger the
        // viewport's Enter-to-Apply shortcut before the button's own click.
        await panel.getByRole('button', { name: 'Select face perimeter', exact: true }).press('Enter')
        await expect(panel.getByRole('button', { name: 'Apply', exact: true })).toBeDisabled()
        expect(await page.evaluate(() => window.__hew_test!.exportStl(0)!.bytes)).toEqual(before)
        await pick([1,1.5,4])
        await expect(panel.getByRole('button', { name: 'Apply', exact: true })).toBeEnabled()
      }
      expect(await page.evaluate(() => window.__hew_test!.exportStl(0)!.bytes)).toEqual(before)
      await panel.getByRole('button', { name: 'Clear', exact: true }).click()
      await expect(panel.getByRole('button', { name: 'Apply', exact: true })).toBeDisabled()
      expect(await page.evaluate(() => window.__hew_test!.exportStl(0)!.bytes)).toEqual(before)
      await select()
      await page.evaluate(() => window.__hew_test!.startRecording())
      await panel.getByRole('button', { name: 'Apply', exact: true }).click()
      const after = await page.evaluate(() => window.__hew_test!.exportStl(0)!.bytes)
      expect(after).not.toEqual(before)
      const measured = inspectClosedStl(after)
      expect(measured.volumeMm3).toBeLessThan(inspectClosedStl(before).volumeMm3)
      const recording = await page.evaluate(() => {
        window.__hew_test!.stopRecording()
        return window.__hew_test!.takeRecording()
      })
      expect(JSON.parse(recording).calls.filter((c: {method: string}) => c.method === 'edge_treatment_batch')).toHaveLength(1)
      await page.evaluate(() => window.__hew_test!.undo())
      expect(await page.evaluate(() => window.__hew_test!.exportStl(0)!.bytes)).toEqual(before)
      await page.evaluate(() => window.__hew_test!.redo())
      expect(await page.evaluate(() => window.__hew_test!.exportStl(0)!.bytes)).toEqual(after)
      await panel.getByRole('button', { name: 'Cancel', exact: true }).click()
      expect(await page.evaluate(() => window.__hew_test!.exportStl(0)!.bytes)).toEqual(after)
    })
  }
}

test('parameter help supports hover, keyboard and click without applying geometry', async ({ page }) => {
  await page.goto('/')
  await page.waitForFunction(() => window.__hew_test?.isReady() === true)
  await page.getByRole('radio', { name: 'Edge Treatment', exact: true }).click()
  const panel = page.getByRole('region', { name: 'Edge Treatment' })
  await panel.getByLabel('Operation').selectOption('roundover')
  for (const [label, text] of [['Roundover radius', "arc's center"], ['Maximum chord error', 'roundovers, not chamfers']]) {
    const help = panel.getByRole('button', { name: `About ${label}`, exact: true })
    await help.focus()
    await expect(panel.getByRole('tooltip')).toContainText(text)
    await help.press('Escape')
    await expect(panel.getByRole('tooltip')).toHaveCount(0)
    await help.click()
    await expect(panel.getByRole('tooltip')).toBeVisible()
    await help.click()
    await expect(panel.getByRole('tooltip')).toHaveCount(0)
  }
  await panel.getByLabel('Operation').selectOption('chamfer')
  await expect(panel.getByRole('button', { name: 'About Roundover radius' })).toHaveCount(0)
  const chamfer = panel.getByRole('button', { name: 'About Chamfer distance' })
  await chamfer.hover()
  await expect(panel.getByRole('tooltip')).toContainText('0.354 inches')
  await page.mouse.move(1000, 800)
  await expect(panel.getByRole('tooltip')).toHaveCount(0)
  await expect(panel.getByRole('button', { name: 'Apply', exact: true })).toBeDisabled()
  expect(await page.evaluate(() => window.__hew_test!.getObjectIds())).toEqual([])
})
