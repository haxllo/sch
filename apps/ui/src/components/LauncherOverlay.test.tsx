import React from 'react'
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { LauncherOverlay } from './LauncherOverlay'

afterEach(() => {
  cleanup()
})

describe('LauncherOverlay', () => {
  it('focuses the search input on open', () => {
    const search = vi.fn().mockResolvedValue([])
    const launch = vi.fn().mockResolvedValue(undefined)

    render(<LauncherOverlay searchCommand={search} launchCommand={launch} />)

    const input = screen.getByRole('textbox', { name: 'Launcher Query' })
    expect(input).toHaveFocus()
  })

  it('executes search command and renders result rows', async () => {
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
        title: 'Codeium',
        path: 'C:\\Codeium.exe',
      },
    ])
    const launch = vi.fn().mockResolvedValue(undefined)

    render(<LauncherOverlay searchCommand={search} launchCommand={launch} />)

    const input = screen.getByRole('textbox', { name: 'Launcher Query' })
    fireEvent.change(input, { target: { value: 'code' } })

    await waitFor(() => {
      expect(search).toHaveBeenCalledWith('code', 20)
      expect(screen.getByText('Visual Studio Code')).toBeInTheDocument()
      expect(screen.getByText('Codeium')).toBeInTheDocument()
    })
  })

  it('supports keyboard selection and enter-to-launch', async () => {
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
        title: 'Codeium',
        path: 'C:\\Codeium.exe',
      },
    ])
    const launch = vi.fn().mockResolvedValue(undefined)

    render(<LauncherOverlay searchCommand={search} launchCommand={launch} />)

    const input = screen.getByRole('textbox', { name: 'Launcher Query' })
    fireEvent.change(input, { target: { value: 'code' } })

    await screen.findByText('Visual Studio Code')
    fireEvent.keyDown(input, { key: 'ArrowDown' })
    fireEvent.keyDown(input, { key: 'Enter' })

    await waitFor(() => {
      expect(launch).toHaveBeenCalledWith({ id: '2', path: 'C:\\Codeium.exe' })
    })
  })

  it('shows launch errors in the UI', async () => {
    const search = vi.fn().mockResolvedValue([
      {
        id: '1',
        kind: 'app',
        title: 'Visual Studio Code',
        path: 'C:\\Code.exe',
      },
    ])
    const launch = vi.fn().mockRejectedValue(new Error('Launch failed: access denied'))

    render(<LauncherOverlay searchCommand={search} launchCommand={launch} />)

    const input = screen.getByRole('textbox', { name: 'Launcher Query' })
    fireEvent.change(input, { target: { value: 'code' } })

    await screen.findByText('Visual Studio Code')
    fireEvent.keyDown(input, { key: 'Enter' })

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('Launch failed: access denied')
    })
  })
})
