import { readFileSync } from 'node:fs'
import { describe, it, expect } from 'vitest'

const config = (name: string) =>
  JSON.parse(readFileSync(new URL(`../../shells/tauri/src-tauri/${name}`, import.meta.url), 'utf8'))
const installed = config('tauri.conf.json')
const alpha = config('tauri.edge-alpha.conf.json')

describe('experimental desktop isolation', () => {
  it('uses a distinct reverse-domain identity and visible product name', () => {
    expect(alpha.identifier).not.toBe(installed.identifier)
    expect(alpha.identifier).toMatch(/^[A-Za-z][\w]*(\.[A-Za-z][\w]*)+$/)
    expect(alpha.productName).not.toBe(installed.productName)
    expect(alpha.productName).toContain('Alpha')
  })
  it('disables packaging and update endpoints without changing the installed application config', () => {
    expect(alpha.bundle.active).toBe(false)
    expect(alpha.plugins.updater.endpoints).toEqual([])
    expect(installed.bundle.active).toBe(true)
    expect(installed.plugins.updater.endpoints.length).toBeGreaterThan(0)
  })
  it('inherits the normal frontend hooks, sidecar, and security policy', () => {
    expect(alpha.build).toBeUndefined()
    expect(alpha.app).toBeUndefined()
    expect(alpha.bundle.externalBin).toBeUndefined()
    expect(installed.build.beforeBuildCommand).toBe('pnpm --dir ../../app build')
    expect(installed.bundle.externalBin).toContain('binaries/hew-cli')
    expect(installed.app.security.csp).toContain("object-src 'none'")
  })
})
