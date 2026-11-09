import { test, expect } from '@playwright/test'

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
  await panel.getByLabel('Maximum chord error').fill('5 mm')
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
