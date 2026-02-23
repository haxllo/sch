import { describe, expect, it } from 'vitest'
import { existsSync } from 'node:fs'

describe('scaffold', () => {
  it('has native runtime entry points and bundled fonts', () => {
    expect(existsSync('apps/core/src/main.rs')).toBe(true)
    expect(existsSync('apps/core/src/windows_overlay.rs')).toBe(true)
    expect(existsSync('apps/assets/fonts/Geist/otf/Geist-Regular.otf')).toBe(true)
  })
})
