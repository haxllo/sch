import React from 'react'
import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import { LauncherOverlay } from './LauncherOverlay'

describe('LauncherOverlay', () => {
  it('renders input with autofocus', () => {
    render(<LauncherOverlay query="" results={[]} />)
    const input = screen.getByRole('textbox')
    expect(input).toHaveAttribute('autofocus')
  })
})
