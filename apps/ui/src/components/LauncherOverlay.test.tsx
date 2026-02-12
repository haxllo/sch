import React from 'react'
import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import { LauncherOverlay } from './LauncherOverlay'

describe('LauncherOverlay', () => {
  it('focuses the search input on open', () => {
    render(<LauncherOverlay query="" results={[]} />)
    const input = screen.getByRole('textbox')
    expect(input).toHaveFocus()
  })
})
