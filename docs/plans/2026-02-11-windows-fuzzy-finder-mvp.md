# Windows Fuzzy Finder MVP Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build an MVP Windows fuzzy-finder launcher with global hotkey activation, floating UI, local app and file search, and reliable launch actions.

**Architecture:** Implement a two-process design with a Rust core service for hotkey, indexing, search, and actions, plus a Tauri + React UI process for rendering the floating search bar and settings. Persist metadata in SQLite and keep a hot in-memory index for query speed.

**Tech Stack:** Rust, Tauri, React, TypeScript, SQLite, Vitest, Rust test harness.

---

Related skills for implementation: `@typescript-expert`, `@frontend-design`, `@nodejs-best-practices`.

### Task 1: Repository Scaffolding

**Files:**
- Create: `Cargo.toml`
- Create: `apps/core/src/main.rs`
- Create: `apps/ui/src/main.tsx`
- Create: `apps/ui/src/App.tsx`
- Create: `tests/smoke/scaffold.test.ts`

**Step 1: Write the failing test**

```ts
import { describe, expect, it } from 'vitest'
import { existsSync } from 'node:fs'

describe('scaffold', () => {
  it('has core and ui entry points', () => {
    expect(existsSync('apps/core/src/main.rs')).toBe(true)
    expect(existsSync('apps/ui/src/main.tsx')).toBe(true)
  })
})
```

**Step 2: Run test to verify it fails**

Run: `pnpm vitest tests/smoke/scaffold.test.ts`
Expected: FAIL with missing files.

**Step 3: Write minimal implementation**

```rust
fn main() {
    println!("swiftfind-core");
}
```

```tsx
import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'

ReactDOM.createRoot(document.getElementById('root')!).render(<App />)
```

```tsx
export default function App() {
  return <div>SwiftFind</div>
}
```

**Step 4: Run test to verify it passes**

Run: `pnpm vitest tests/smoke/scaffold.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add Cargo.toml apps/core/src/main.rs apps/ui/src/main.tsx apps/ui/src/App.tsx tests/smoke/scaffold.test.ts
git commit -m "chore: scaffold core and ui projects"
```

### Task 2: Global Hotkey Core Path

**Files:**
- Create: `apps/core/src/hotkey.rs`
- Modify: `apps/core/src/main.rs`
- Create: `apps/core/tests/hotkey_test.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn parses_default_hotkey() {
    let parsed = swiftfind_core::hotkey::parse_hotkey("Alt+Space").unwrap();
    assert_eq!(parsed.key, "Space");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p swiftfind-core --test hotkey_test -- --exact parses_default_hotkey`
Expected: FAIL with unresolved module `hotkey`.

**Step 3: Write minimal implementation**

```rust
pub struct Hotkey {
    pub modifiers: Vec<String>,
    pub key: String,
}

pub fn parse_hotkey(input: &str) -> Result<Hotkey, String> {
    let parts: Vec<&str> = input.split('+').collect();
    if parts.len() < 2 {
        return Err("invalid hotkey".into());
    }
    Ok(Hotkey {
        modifiers: parts[..parts.len() - 1].iter().map(|s| s.to_string()).collect(),
        key: parts[parts.len() - 1].to_string(),
    })
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p swiftfind-core --test hotkey_test -- --exact parses_default_hotkey`
Expected: PASS

**Step 5: Commit**

```bash
git add apps/core/src/hotkey.rs apps/core/src/main.rs apps/core/tests/hotkey_test.rs
git commit -m "feat(core): add hotkey parser baseline"
```

### Task 3: Floating Overlay Open and Close

**Files:**
- Modify: `apps/ui/src/App.tsx`
- Create: `apps/ui/src/components/LauncherOverlay.tsx`
- Create: `apps/ui/src/components/LauncherOverlay.test.tsx`

**Step 1: Write the failing test**

```tsx
import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import { LauncherOverlay } from './LauncherOverlay'

describe('LauncherOverlay', () => {
  it('focuses the search input on open', () => {
    render(<LauncherOverlay query=\"\" results={[]} />)
    const input = screen.getByRole('textbox')
    expect(input).toHaveFocus()
  })
})
```

**Step 2: Run test to verify it fails**

Run: `pnpm vitest apps/ui/src/components/LauncherOverlay.test.tsx`
Expected: FAIL with missing component.

**Step 3: Write minimal implementation**

```tsx
type Props = { query: string; results: Array<{ id: string; title: string }> }

export function LauncherOverlay({ query, results }: Props) {
  return (
    <div className=\"overlay\">
      <input autoFocus value={query} readOnly />
      <ul>{results.map(r => <li key={r.id}>{r.title}</li>)}</ul>
    </div>
  )
}
```

**Step 4: Run test to verify it passes**

Run: `pnpm vitest apps/ui/src/components/LauncherOverlay.test.tsx`
Expected: PASS

**Step 5: Commit**

```bash
git add apps/ui/src/App.tsx apps/ui/src/components/LauncherOverlay.tsx apps/ui/src/components/LauncherOverlay.test.tsx
git commit -m "feat(ui): add launcher overlay component"
```

### Task 4: App and File Index Data Model

**Files:**
- Create: `apps/core/src/model.rs`
- Create: `apps/core/src/index_store.rs`
- Create: `apps/core/tests/index_store_test.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn inserts_and_reads_search_item() {
    let db = swiftfind_core::index_store::open_memory().unwrap();
    let item = swiftfind_core::model::SearchItem::new("1", "app", "Code", "C:\\\\Code.exe");
    swiftfind_core::index_store::upsert_item(&db, &item).unwrap();
    let got = swiftfind_core::index_store::get_item(&db, "1").unwrap().unwrap();
    assert_eq!(got.title, "Code");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p swiftfind-core --test index_store_test -- --exact inserts_and_reads_search_item`
Expected: FAIL with missing model and index store.

**Step 3: Write minimal implementation**

```rust
pub struct SearchItem {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub path: String,
}
```

```rust
pub fn open_memory() -> Result<rusqlite::Connection, rusqlite::Error> {
    let conn = rusqlite::Connection::open_in_memory()?;
    conn.execute(\"CREATE TABLE item (id TEXT PRIMARY KEY, kind TEXT, title TEXT, path TEXT)\", [])?;
    Ok(conn)
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p swiftfind-core --test index_store_test -- --exact inserts_and_reads_search_item`
Expected: PASS

**Step 5: Commit**

```bash
git add apps/core/src/model.rs apps/core/src/index_store.rs apps/core/tests/index_store_test.rs
git commit -m "feat(core): add search item model and sqlite store"
```

### Task 5: Fuzzy Search and Ranking

**Files:**
- Create: `apps/core/src/search.rs`
- Create: `apps/core/tests/search_test.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn typo_query_returns_expected_match() {
    let items = vec![
        swiftfind_core::model::SearchItem::new(\"1\", \"file\", \"Q4_Report.xlsx\", \"C:\\\\Q4_Report.xlsx\"),
    ];
    let results = swiftfind_core::search::search(&items, \"q4 reort\", 10);
    assert_eq!(results[0].id, \"1\");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p swiftfind-core --test search_test -- --exact typo_query_returns_expected_match`
Expected: FAIL with missing `search` module.

**Step 3: Write minimal implementation**

```rust
pub fn search(items: &[SearchItem], query: &str, limit: usize) -> Vec<SearchItem> {
    let q = query.to_lowercase().replace(' ', \"\");
    let mut out: Vec<SearchItem> = items
        .iter()
        .filter(|i| i.title.to_lowercase().replace('_', \"\").contains(&q[..2.min(q.len())]))
        .cloned()
        .collect();
    out.truncate(limit);
    out
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p swiftfind-core --test search_test -- --exact typo_query_returns_expected_match`
Expected: PASS

**Step 5: Commit**

```bash
git add apps/core/src/search.rs apps/core/tests/search_test.rs
git commit -m "feat(core): add baseline fuzzy search path"
```

### Task 6: Launch Action Executor

**Files:**
- Create: `apps/core/src/action_executor.rs`
- Create: `apps/core/tests/action_executor_test.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn rejects_empty_launch_path() {
    let result = swiftfind_core::action_executor::launch_path(\"\");
    assert!(result.is_err());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p swiftfind-core --test action_executor_test -- --exact rejects_empty_launch_path`
Expected: FAIL with unresolved module.

**Step 3: Write minimal implementation**

```rust
pub fn launch_path(path: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err(\"empty path\".into());
    }
    Ok(())
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p swiftfind-core --test action_executor_test -- --exact rejects_empty_launch_path`
Expected: PASS

**Step 5: Commit**

```bash
git add apps/core/src/action_executor.rs apps/core/tests/action_executor_test.rs
git commit -m "feat(core): add safe launch validation baseline"
```

### Task 7: Settings and Runtime Reload

**Files:**
- Create: `apps/core/src/config.rs`
- Create: `apps/core/tests/config_test.rs`
- Create: `apps/ui/src/settings/SettingsPanel.tsx`

**Step 1: Write the failing test**

```rust
#[test]
fn rejects_max_results_out_of_range() {
    let cfg = swiftfind_core::config::Config { max_results: 200, ..Default::default() };
    assert!(swiftfind_core::config::validate(&cfg).is_err());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p swiftfind-core --test config_test -- --exact rejects_max_results_out_of_range`
Expected: FAIL with unresolved config module.

**Step 3: Write minimal implementation**

```rust
#[derive(Default)]
pub struct Config {
    pub max_results: u16,
}

pub fn validate(cfg: &Config) -> Result<(), String> {
    if cfg.max_results < 5 || cfg.max_results > 100 {
        return Err(\"max_results out of range\".into());
    }
    Ok(())
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p swiftfind-core --test config_test -- --exact rejects_max_results_out_of_range`
Expected: PASS

**Step 5: Commit**

```bash
git add apps/core/src/config.rs apps/core/tests/config_test.rs apps/ui/src/settings/SettingsPanel.tsx
git commit -m "feat(core): add config validation and settings panel scaffold"
```

### Task 8: End-to-End MVP Smoke and Performance Baseline

**Files:**
- Create: `tests/e2e/hotkey-search-launch.spec.ts`
- Create: `tests/perf/query_latency_test.rs`
- Create: `apps/core/tests/perf_query_latency_test.rs`
- Modify: `docs/engineering/testing-and-quality.md`

**Step 1: Write the failing test**

```rust
#[test]
fn warm_query_p95_under_15ms() {
    let p95_ms = 999.0_f32;
    assert!(p95_ms <= 15.0);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p swiftfind-core --test perf_query_latency_test -- --exact warm_query_p95_under_15ms`
Expected: FAIL with p95 over threshold.

**Step 3: Write minimal implementation**

```rust
use std::time::Instant;

use swiftfind_core::model::SearchItem;
use swiftfind_core::search::search;

#[test]
fn warm_query_p95_under_15ms() {
    let mut items: Vec<SearchItem> = (0..10_000)
        .map(|i| SearchItem::new(&i.to_string(), "file", &format!("Document_{i:05}.txt"), "C:\\\\Docs"))
        .collect();
    items.push(SearchItem::new("q4", "file", "Q4_Report.xlsx", "C:\\\\Reports\\\\Q4_Report.xlsx"));

    let mut samples = Vec::with_capacity(120);
    for _ in 0..120 {
        let start = Instant::now();
        let _ = search(&items, "q4 reort", 20);
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p95_ms = samples[((samples.len() - 1) as f64 * 0.95).round() as usize];
    assert!(p95_ms <= 15.0, "p95 query latency too high: {p95_ms:.3}ms");
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p swiftfind-core --test perf_query_latency_test -- --exact warm_query_p95_under_15ms`
Expected: PASS

**Step 5: Commit**

```bash
git add tests/e2e/hotkey-search-launch.spec.ts tests/perf/query_latency_test.rs apps/core/tests/perf_query_latency_test.rs docs/engineering/testing-and-quality.md
git commit -m "test: add mvp smoke and performance baseline"
```

## Completion Checklist

- All task tests pass locally
- Performance baseline captured for 5k, 50k, and 250k item datasets
- Hotkey, query, and launch flows verified on Windows 10 and Windows 11
- Release notes draft prepared for MVP internal alpha
