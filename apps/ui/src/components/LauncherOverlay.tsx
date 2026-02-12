import React from 'react'
import type { LaunchRequest, SearchResultDto } from '../core-contract'

export type SearchCommand = (
  query: string,
  limit: number,
) => Promise<SearchResultDto[]>

export type LaunchCommand = (payload: LaunchRequest) => Promise<void>

type Props = {
  searchCommand: SearchCommand
  launchCommand: LaunchCommand
  resultLimit?: number
}

function messageFromUnknown(error: unknown): string {
  if (error instanceof Error) {
    return error.message
  }

  return 'Unknown launcher error'
}

export function LauncherOverlay({
  searchCommand,
  launchCommand,
  resultLimit = 20,
}: Props) {
  const [query, setQuery] = React.useState('')
  const [results, setResults] = React.useState<SearchResultDto[]>([])
  const [selectedIndex, setSelectedIndex] = React.useState(0)
  const [error, setError] = React.useState<string | null>(null)

  React.useEffect(() => {
    let active = true

    async function runSearch() {
      const trimmed = query.trim()
      if (!trimmed) {
        if (active) {
          setResults([])
          setSelectedIndex(0)
          setError(null)
        }
        return
      }

      try {
        const next = await searchCommand(trimmed, resultLimit)
        if (!active) {
          return
        }
        setResults(next)
        setSelectedIndex(0)
        setError(null)
      } catch (searchError) {
        if (!active) {
          return
        }
        setResults([])
        setSelectedIndex(0)
        setError(messageFromUnknown(searchError))
      }
    }

    void runSearch()

    return () => {
      active = false
    }
  }, [query, resultLimit, searchCommand])

  async function launchSelected() {
    const selected = results[selectedIndex]
    if (!selected) {
      return
    }

    try {
      await launchCommand({ id: selected.id, path: selected.path })
      setError(null)
    } catch (launchError) {
      setError(messageFromUnknown(launchError))
    }
  }

  function onKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      if (results.length > 0) {
        setSelectedIndex((current) => Math.min(current + 1, results.length - 1))
      }
      return
    }

    if (event.key === 'ArrowUp') {
      event.preventDefault()
      if (results.length > 0) {
        setSelectedIndex((current) => Math.max(current - 1, 0))
      }
      return
    }

    if (event.key === 'Enter') {
      event.preventDefault()
      void launchSelected()
    }
  }

  return (
    <div className="overlay">
      <input
        autoFocus
        aria-label="Launcher Query"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        onKeyDown={onKeyDown}
      />
      <ul role="listbox" aria-label="Search Results">
        {results.map((result, index) => (
          <li
            key={result.id}
            role="option"
            aria-selected={index === selectedIndex}
            data-selected={index === selectedIndex ? 'true' : 'false'}
          >
            {result.title}
          </li>
        ))}
      </ul>
      {error ? <p role="alert">{error}</p> : null}
    </div>
  )
}
