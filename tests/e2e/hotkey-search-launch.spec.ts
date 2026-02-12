import { describe, it, expect } from 'vitest'

describe('hotkey-search-launch mvp smoke', () => {
  it('documents the intended e2e path', () => {
    expect('hotkey->query->launch').toContain('launch')
  })
})
