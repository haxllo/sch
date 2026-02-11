import React from 'react'
import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import { LauncherOverlay } from '../../apps/ui/src/components/LauncherOverlay'

describe('hotkey-search-launch mvp smoke', () => {
  it('renders query and ranked result rows', () => {
    render(
      React.createElement(LauncherOverlay, {
        query: 'code',
        results: [
          { id: '1', title: 'Visual Studio Code' },
          { id: '2', title: 'Codeium' },
        ],
      }),
    )

    expect(screen.getByRole('textbox')).toHaveValue('code')
    expect(screen.getByText('Visual Studio Code')).toBeInTheDocument()
    expect(screen.getByText('Codeium')).toBeInTheDocument()
  })
})
