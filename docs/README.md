# Windows Fuzzy Finder Documentation

This repository contains planning and architecture documentation for a lightweight Windows fuzzy-finder launcher.

## Project Summary

Build a keyboard-first launcher that appears as a floating search bar on a global hotkey and can quickly find and run:
- Applications
- Files and folders
- User-defined actions

Primary goals:
- Faster and cleaner than Windows built-in search for local workflows
- Low CPU and RAM overhead
- Modern, minimal, keyboard-centric UI

## Documentation Map

Product:
- `docs/product/project-charter.md`: vision, goals, non-goals, personas, KPIs
- `docs/product/requirements.md`: functional and non-functional requirements
- `docs/product/user-flows.md`: primary user journeys and acceptance criteria

Architecture:
- `docs/architecture/system-architecture.md`: component design and runtime data flow
- `docs/architecture/search-indexing-design.md`: indexing, fuzzy matching, ranking, latency strategy
- `docs/architecture/configuration-spec.md`: settings model and schema
- `docs/architecture/security-privacy.md`: threat model, local-data boundaries, safe execution

Engineering:
- `docs/engineering/testing-and-quality.md`: testing strategy and release gates
- `docs/engineering/roadmap.md`: milestones from MVP to extensibility
- `docs/engineering/risk-register.md`: key technical and delivery risks
- `docs/engineering/windows-security-release-checklist.md`: Windows-specific security and release blocking checklist

Decisions:
- `docs/decisions/ADR-001-core-stack.md`: initial architecture decision record

Plan:
- `docs/plans/2026-02-11-windows-fuzzy-finder-mvp.md`: detailed implementation sequence

## Recommended Reading Order

1. `docs/product/project-charter.md`
2. `docs/product/requirements.md`
3. `docs/architecture/system-architecture.md`
4. `docs/architecture/search-indexing-design.md`
5. `docs/plans/2026-02-11-windows-fuzzy-finder-mvp.md`
