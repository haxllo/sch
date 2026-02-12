# Core User Flows

## Flow 1: Launch an App Quickly

1. User presses global hotkey.
2. Floating search bar appears with focused input.
3. User types `code`.
4. Launcher returns ranked app results.
5. User presses `Enter` on `Visual Studio Code`.
6. App starts; launcher closes.

Acceptance criteria:
- UI appears immediately with no visible jitter.
- First relevant app appears in top 3 for common aliases.

## Flow 2: Open a File by Partial Name

1. User invokes launcher.
2. Types partial file name with typo (`q4 reort`).
3. Results show `Q4_Report.xlsx` despite typo.
4. User selects and opens file.

Acceptance criteria:
- Typo tolerance returns likely matches.
- Query latency remains below target for warm index.

## Flow 3: Open Parent Folder

1. User searches a file.
2. User highlights result and triggers secondary action (`Ctrl+Enter`).
3. File Explorer opens parent folder with file selected.

Acceptance criteria:
- Secondary action exposed in UI hint row.
- Action works even if default open command fails.

## Flow 4: Reconfigure Indexed Paths

1. User opens settings from launcher.
2. Adds `D:\Projects` and excludes `node_modules`.
3. Indexer updates incrementally.
4. New files become searchable without full rebuild.

Acceptance criteria:
- Settings save atomically.
- Indexing progress and last-sync status are visible.

## Flow 5: Recover from Missing Target

1. User selects stale shortcut whose target no longer exists.
2. Launch fails gracefully.
3. UI shows clear error and offers to remove stale entry.

Acceptance criteria:
- No app crash.
- Clear message and one-click cleanup option.
