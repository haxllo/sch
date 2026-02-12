# Security and Privacy

## Principles

- Local-first data processing
- Explicit user intent before sensitive actions
- Least-privilege runtime by default

## Data Boundaries

Stored locally:
- Search index metadata
- Usage counters and recent selections
- User configuration

Not stored by default:
- File content
- Keystroke logs
- Remote telemetry

## Threat Model (MVP)

- Malicious or broken shortcut target paths
- Path traversal and unsafe command execution
- Accidental launch of privileged actions
- Leakage of indexed path metadata

## Mitigations

- Validate all launch targets before execution
- Use allowlisted action handlers instead of shell string concatenation
- Require explicit command for run-as-admin behavior
- Restrict logs to non-sensitive operational fields
- Encrypt sensitive optional settings if introduced later

## Telemetry Policy

- Telemetry is off by default
- If enabled, collect aggregate performance counters only
- Do not send raw query text without separate explicit consent

## Secure Development Checklist

- Static analysis and dependency audit on CI
- Fuzz testing for parser and tokenization code paths
- Regression tests for launch action sanitization
- Security review before public release
