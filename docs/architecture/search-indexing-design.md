# Search and Indexing Design

## Discovery Sources

Applications:
- Start menu shortcuts (`.lnk`)
- Registered installed app entries
- Executables found in user-defined paths

Files and folders:
- User-selected indexed roots
- Include and exclude patterns

Commands:
- Built-in actions such as settings, rebuild index, and clear history

## Indexing Strategy

- Initial bootstrap:
- Scan app sources and configured file roots
- Normalize metadata and write to SQLite cache

- Incremental updates:
- Watch file system changes for configured roots
- Schedule low-priority reconciliation scan periodically
- Recover from missed events by periodic checksum pass

- Startup path:
- Load cache into memory first, then refresh asynchronously
- Ensure search is available before full refresh completes

## Tokenization

- Lowercase
- Remove punctuation
- Split on whitespace, path separators, and camel-case boundaries
- Store original text and normalized tokens

Example:
- `VisualStudioCode.exe` -> `visual`, `studio`, `code`, `visualstudio`, `vscode`

## Matching and Ranking

Candidate generation:
- Prefix and substring checks first
- Fuzzy candidate expansion only on lower-confidence sets

Score components:
- Exact match boost
- Prefix boost
- Fuzzy distance score
- Historical usage score
- Recent usage recency decay
- File-type penalty or boost (configurable)

Composite score (example):

```text
score = exact_boost
      + prefix_boost
      + fuzzy_score
      + usage_weight * log(1 + use_count)
      + recency_weight * recency_decay
```

Tie-breaking:
- Higher usage count
- Most recently used
- Shorter display title

## Latency Budget

- Query parsing and normalization: <= 1ms
- Candidate retrieval from in-memory index: <= 5ms
- Fuzzy scoring and ranking top N: <= 7ms
- In-process overlay result projection and dispatch: <= 2ms

Target P95 end-to-end warm query: <= 15ms

## Operational Controls

- Cap result set size (default 20)
- Limit expensive fuzzy pass above configurable query length
- Use cooperative cancellation when user keeps typing
