import { test, expect } from '@playwright/test'
import { writeFile } from 'node:fs/promises'
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

test('generalized trapezoid: native-bound slider, successive opposite perimeters and undo', async ({ page }) => {
  test.setTimeout(120_000)
  await page.goto('/')
  await page.waitForFunction(() => window.__hew_test?.isReady() === true)
  await page.evaluate(() => {
    const h=window.__hew_test!
    h.setLengthUnit('mm')
    const p: [number,number,number][]=[[0,0,0],[0.1,0,0],[0.07,0.06,0],[0.01,0.06,0],[0,0,0]]
    const sketch=h.drawLineChain(p)
    h.extrudeRegion(sketch.sketch,sketch.regions[0],0.04)
    h.setCamera({position:[0.25,-0.25,0.25],target:[0.05,0.03,0.02],up:[0,0,1],fovDeg:45})
    h.saveWithCameraState()
  })
  const original=await page.evaluate(()=>window.__hew_test!.exportStl(0)!.bytes)
  await page.getByRole('radio',{name:'Edge Treatment',exact:true}).click()
  const panel=page.getByRole('region',{name:'Edge Treatment'})
  await panel.getByLabel('Operation').selectOption('roundover')
  await panel.getByLabel('Maximum chord error',{exact:true}).fill('0.02 mm')
  for(const upper of [true,false]) {
    if(!upper) await page.evaluate(()=>{
      const h=window.__hew_test!
      h.setCamera({position:[0.25,-0.25,-0.25],target:[0.05,0.03,0.02],up:[0,0,1],fovDeg:45})
      h.saveWithCameraState()
    })
    await panel.getByLabel('Radius',{exact:true}).fill(upper?'3 mm':'0.0125"')
    await panel.getByRole('button',{name:'Select face perimeter',exact:true}).click()
    const rect=await page.locator('canvas').first().boundingBox()
    if(!rect) throw Error('missing canvas')
    const point=await page.evaluate(z=>window.__hew_test!.worldToScreen([0.05,0.03,z]),upper?0.04:0)
    await page.mouse.move(rect.x+point.x-10,rect.y+point.y-10)
    await page.mouse.move(rect.x+point.x,rect.y+point.y,{steps:3})
    await page.mouse.click(rect.x+point.x,rect.y+point.y)
    await expect(panel.getByText('Selected edges: 4',{exact:true})).toBeVisible()
    await expect(panel.getByRole('button',{name:'Apply',exact:true})).toBeEnabled()
    const slider=panel.getByRole('slider',{name:'Adjust roundover radius'})
    await expect(slider).toBeVisible()
    if(upper) {
      const before=await panel.getByLabel('Radius',{exact:true}).inputValue()
      await slider.focus();await slider.press('ArrowRight')
      await expect(panel.getByLabel('Radius',{exact:true})).not.toHaveValue(before)
      await expect(panel.getByRole('button',{name:'Apply',exact:true})).toBeEnabled()
    }
    await panel.getByRole('button',{name:'Apply',exact:true}).click()
    inspectClosedStl(await page.evaluate(()=>window.__hew_test!.exportStl(0)!.bytes))
  }
  const final=await page.evaluate(()=>window.__hew_test!.exportStl(0)!.bytes)
  expect(final).not.toEqual(original)
  await page.evaluate(()=>{window.__hew_test!.undo();window.__hew_test!.undo()})
  expect(await page.evaluate(()=>window.__hew_test!.exportStl(0)!.bytes)).toEqual(original)
})

for (const first of ['chamfer','roundover'] as const) test(`Threshold cylinder: ${first}, opposite roundover and isolated history`, async ({page}) => {
  test.setTimeout(120_000)
  await page.goto('/'); await page.waitForFunction(()=>window.__hew_test?.isReady()===true)
  await page.evaluate(()=>{
    const h=window.__hew_test!;h.setLengthUnit('mm')
    const circle=h.drawCircle([0,0,0],0.05,64);h.extrudeRegion(circle.sketch,circle.region,0.04)
    h.setCamera({position:[0.25,-0.25,0.25],target:[0,0,0.02],up:[0,0,1],fovDeg:45});h.saveWithCameraState()
  })
  const original=await page.evaluate(()=>window.__hew_test!.exportStl(0)!.bytes)
  await page.getByRole('radio',{name:'Edge Treatment',exact:true}).click()
  const panel=page.getByRole('region',{name:'Edge Treatment'})
  for(const upper of [true,false]) {
    if(!upper) await page.evaluate(()=>{const h=window.__hew_test!;h.setCamera({position:[0.25,-0.25,-0.25],target:[0,0,0.02],up:[0,0,1],fovDeg:45});h.saveWithCameraState()})
    const mode=upper?first:'roundover'
    await panel.getByLabel('Operation').selectOption(mode)
    await panel.getByLabel(mode==='roundover'?'Radius':'Distance',{exact:true}).fill(upper?'3 mm':'2 mm')
    if(mode==='roundover') await panel.getByLabel('Maximum chord error',{exact:true}).fill('0.3 mm')
    await panel.getByRole('button',{name:'Select face perimeter',exact:true}).click()
    const rect=await page.locator('canvas').first().boundingBox();if(!rect)throw Error('missing canvas')
    const p=await page.evaluate(z=>window.__hew_test!.worldToScreen([0,0,z]),upper?0.04:0)
    await page.mouse.move(rect.x+p.x-10,rect.y+p.y-10);await page.mouse.move(rect.x+p.x,rect.y+p.y,{steps:3});await page.mouse.click(rect.x+p.x,rect.y+p.y)
    await expect(panel.getByText('Selected edges: 64',{exact:true})).toBeVisible()
    await expect(panel.getByRole('button',{name:'Apply',exact:true})).toBeEnabled({timeout:60000})
    await expect(panel.getByRole('slider')).toBeVisible()
    await panel.getByRole('button',{name:'Apply',exact:true}).click()
    const measured=inspectClosedStl(await page.evaluate(()=>window.__hew_test!.exportStl(0)!.bytes))
    expect(Math.abs(measured.max[2]-40)).toBeLessThan(0.00002)
    expect(Math.abs(measured.min[2])).toBeLessThan(0.00002)
  }
  await page.evaluate(()=>{window.__hew_test!.undo();window.__hew_test!.undo()})
  expect(await page.evaluate(()=>window.__hew_test!.exportStl(0)!.bytes)).toEqual(original)
})

test('Threshold worker: fine all-edge previews keep the main thread responsive and cancel cleanly',async({page},info)=>{
  test.setTimeout(120_000)
  await page.goto('/');await page.waitForFunction(()=>window.__hew_test?.isReady()===true)
  await page.evaluate(async()=>{
    const {loadKernel}=await import('/src/wasm/loader.ts')
    const {EdgeTreatmentTool}=await import('/src/tools/EdgeTreatmentTool.ts')
    const THREE=await import('/node_modules/.vite/deps/three.js')
    const scene=(await loadKernel()).newScene(),sketch=scene.begin_ground_sketch()
    scene.sketch_begin_gesture(sketch);const p=[[0,0],[.1,0],[.1,.06],[0,.06]]
    p.forEach((a,i)=>{const b=p[(i+1)%4];scene.sketch_add_segment(sketch,...a,0,...b,0)})
    scene.sketch_end_gesture(sketch);const object=scene.extrude_region(sketch,scene.sketch_regions(sketch)[0],.02)
    const rays=[[.05,.03,1,0,0,-1],[.05,.03,-1,0,0,1],[-1,.03,.01,1,0,0],[1,.03,.01,-1,0,0],[.05,-1,.01,0,1,0],[.05,1,.01,0,-1,0]]
    const edges=[...new Set(rays.flatMap(r=>{const hit=scene.pick_face(...r)!;try{return [...scene.edge_treatment_face_edges(object,hit.face())]}finally{hit.free()}}))]
    // Measure modeling responsiveness after the initial application paint;
    // do not attribute cold software-renderer startup to a geometry job.
    await new Promise<void>(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve())))
    const longTasks:{start:number;duration:number}[]=[]
    const observer=new PerformanceObserver(list=>{for(const entry of list.getEntries())longTasks.push({start:entry.startTime,duration:entry.duration})})
    observer.observe({type:'longtask'})
    const NativeWorker=window.Worker;let active=0,peak=0,created=0
    window.Worker=class extends NativeWorker {
      private live=true
      constructor(...args:ConstructorParameters<typeof Worker>){super(...args);active++;created++;peak=Math.max(peak,active)}
      terminate(){if(this.live){this.live=false;active--}super.terminate()}
    }
    let state:any=null,ticks=0,frame=0,lastFrame=performance.now(),maxFrameGapMs=0
    const heartbeat=()=>{const now=performance.now();maxFrameGapMs=Math.max(maxFrameGapMs,now-lastFrame);lastFrame=now;ticks++;frame=requestAnimationFrame(heartbeat)};frame=requestAnimationFrame(heartbeat)
    const tool=new EdgeTreatmentTool(scene,new THREE.Group(),()=>{},s=>{state=s},()=>{})
    tool.activate();tool.configure({mode:'roundover',radius:'6.35 mm',chord:'0.01 mm'})
    const select=()=>edges.forEach((edge,i)=>{tool.setSelectionModifier(i>0);tool.onPointerDown({x:0,y:0,z:0,kind:'on-edge',object,element:edge,elementKind:'edge'},{origin:[0,0,1],direction:[0,0,-1]})})
    const selectionStart=performance.now();select();const selectionSetupMs=performance.now()-selectionStart
    // The fixture selects all 12 edges in one synthetic task. Start the
    // preview-frame gate after that setup; report its cost separately.
    lastFrame=performance.now();maxFrameGapMs=0;ticks=0
    const before=scene.save(),depth=scene.undo_depth(),started=performance.now(),startTicks=ticks
    const dispatchStart=performance.now();tool.generate();const dispatchMs=performance.now()-dispatchStart
    const input=document.createElement('input');input.setAttribute('aria-label','Threshold heartbeat');input.style.cssText='position:fixed;right:20px;top:80px;z-index:1000';document.body.append(input);input.focus()
    ;(window as any).__thresholdPerf={
      read:()=>({phase:state?.phase,error:state?.error,selectionSetupMs,dispatchMs,longTasks:longTasks.slice(-10),maxFrameGapMs,ticks:ticks-startTicks,elapsedMs:performance.now()-started,active,peak,created,geometryPure:(()=>{const after=scene.save();return before.length===after.length&&before.every((v,i)=>v===after[i])})(),metrics:state?.previewMetrics,radius:state?.info?.operation?.radius,pure:(()=>{const after=scene.save();return depth===scene.undo_depth()&&before.length===after.length&&before.every((v,i)=>v===after[i])})()}),
      supersede:()=>{if(!state?.target)select();tool.configure({radius:'6.35 mm',chord:'0.01 mm'});tool.generate();setTimeout(()=>tool.configure({radius:'2 mm',chord:'0.05 mm'}),100)},
      cancel:()=>{for(let i=0;i<10;i++){select();tool.configure({radius:'6.35 mm',chord:'0.01 mm'});tool.generate();tool.clearSelection()}},
      reloadDuringPreview:()=>{select();tool.configure({radius:'6.35 mm',chord:'0.01 mm'});tool.generate();setTimeout(()=>scene.load(scene.save()),100)},
      apply:()=>{const t=performance.now();tool.apply();const elapsed=performance.now()-t;scene.scene_undo().free();return elapsed},
      dispose:()=>{tool.cancel();scene.free();cancelAnimationFrame(frame);observer.disconnect();input.remove();window.Worker=NativeWorker},
    }
  })
  try {
    const inputStarted=Date.now()
    await page.keyboard.insertText('UI remains responsive')
    const inputLatencyMs=Date.now()-inputStarted
    await expect(page.getByLabel('Threshold heartbeat')).toHaveValue('UI remains responsive')
    await page.waitForFunction(()=>(window as any).__thresholdPerf.read().phase==='ready',null,{timeout:60000})
    const first=await page.evaluate(()=>(window as any).__thresholdPerf.read())
    await writeFile(info.outputPath('threshold-worker-first.json'),JSON.stringify({...first,inputLatencyMs},null,2))
    expect(first.pure).toBe(true);expect(first.ticks).toBeGreaterThan(5);expect(first.peak).toBe(1);expect(first.active).toBe(0);expect(inputLatencyMs).toBeLessThan(400);expect(first.dispatchMs).toBeLessThan(100);expect(first.longTasks.every((task:{duration:number})=>task.duration<100)).toBe(true)
    const applyMs=await page.evaluate(()=>(window as any).__thresholdPerf.apply())
    await page.evaluate(()=>(window as any).__thresholdPerf.supersede())
    await page.waitForFunction(()=>(window as any).__thresholdPerf.read().phase==='ready'&&(window as any).__thresholdPerf.read().radius===0.002,null,{timeout:60000})
    await page.evaluate(()=>(window as any).__thresholdPerf.cancel())
    await page.waitForTimeout(300)
    const final=await page.evaluate(()=>(window as any).__thresholdPerf.read())
    expect(final.active).toBe(0);expect(final.phase).toBe('select');expect(final.created).toBeGreaterThanOrEqual(12)
    await page.evaluate(()=>(window as any).__thresholdPerf.reloadDuringPreview())
    await page.waitForFunction(()=>(window as any).__thresholdPerf.read().phase==='error',null,{timeout:60000})
    const stale=await page.evaluate(()=>(window as any).__thresholdPerf.read())
    expect(stale.error).toContain('no longer current');expect(stale.active).toBe(0);expect(stale.geometryPure).toBe(true)
    const path=info.outputPath('threshold-worker-timing.json')
    await writeFile(path,JSON.stringify({first,inputLatencyMs,applyMs,final,stale},null,2))
    await info.attach('threshold-worker-timing.json',{path,contentType:'application/json'})
  }finally{await page.evaluate(()=>(window as any).__thresholdPerf.dispose())}
})
