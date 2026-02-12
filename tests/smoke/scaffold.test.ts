import { describe, expect, it } from 'vitest'
import { existsSync } from 'node:fs'

describe('scaffold', () => {
  it('has core and ui entry points', () => {
    expect(existsSync('apps/core/src/main.rs')).toBe(true)
    expect(existsSync('apps/ui/src/main.tsx')).toBe(true)
  })
})
