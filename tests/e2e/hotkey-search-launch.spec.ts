import React from 'react'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { LauncherOverlay } from '../../apps/ui/src/components/LauncherOverlay'

afterEach(() => {
  document.body.innerHTML = ''
})

describe('hotkey-search-launch mvp smoke', () => {
  it('searches, selects with keyboard, and launches the selected item', async () => {
    const search = vi.fn().mockResolvedValue([
      {
        id: '1',
        kind: 'app',
        title: 'Visual Studio Code',
        path: 'C:\\Code.exe',
      },
      {
        id: '2',
        kind: 'app',
        title: 'Windows Terminal',
        path: 'C:\\Terminal.exe',
      },
    ])
    const launch = vi.fn().mockResolvedValue(undefined)

    render(React.createElement(LauncherOverlay, { searchCommand: search, launchCommand: launch }))

    const input = screen.getByRole('textbox', { name: 'Launcher Query' })
    fireEvent.change(input, { target: { value: 'term' } })

    await screen.findByText('Windows Terminal')

    fireEvent.keyDown(input, { key: 'ArrowDown' })
    fireEvent.keyDown(input, { key: 'Enter' })

    await waitFor(() => {
      expect(launch).toHaveBeenCalledWith({
        id: '2',
        path: 'C:\\Terminal.exe',
      })
    })
  })
})
