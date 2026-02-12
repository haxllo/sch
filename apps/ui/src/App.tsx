import React from 'react'
import { launchCommand, searchCommand } from './core-client'
import { LauncherOverlay } from './components/LauncherOverlay'

export default function App() {
  return (
    <LauncherOverlay
      searchCommand={searchCommand}
      launchCommand={launchCommand}
      resultLimit={20}
    />
  )
}
