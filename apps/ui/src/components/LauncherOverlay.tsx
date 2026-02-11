import React from 'react'

type ResultItem = { id: string; title: string }

type Props = { query: string; results: ResultItem[] }

export function LauncherOverlay({ query, results }: Props) {
  return (
    <div className="overlay">
      <input autoFocus value={query} readOnly />
      <ul>
        {results.map((r) => (
          <li key={r.id}>{r.title}</li>
        ))}
      </ul>
    </div>
  )
}
