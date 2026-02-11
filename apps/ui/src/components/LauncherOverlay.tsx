import React from 'react'

type ResultItem = { id: string; title: string }

type Props = { query: string; results: ResultItem[] }

export function LauncherOverlay({ query, results }: Props) {
  const inputRef = React.useRef<HTMLInputElement>(null)

  React.useEffect(() => {
    if (inputRef.current) {
      inputRef.current.setAttribute('autofocus', '')
      inputRef.current.focus()
    }
  }, [])

  return (
    <div className="overlay">
      <input ref={inputRef} autoFocus value={query} readOnly />
      <ul>
        {results.map((r) => (
          <li key={r.id}>{r.title}</li>
        ))}
      </ul>
    </div>
  )
}
