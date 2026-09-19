# Changelog

## 0.1.3 - 2026-09-19

- Ingest and merge commit only after every replica, pair, and overflow
  check succeeds. A rejected operation leaves the accepted state unchanged.
- Merge copies identity keys, witnesses, and one watermark. Overlapping
  events are rejected. `restore(checkpoint(S))` keeps the same late/accept
  decisions as the live window.
- Restore checks witness budget, duplicate pair keys, and policy fields.
  Checkpoint version is 2.
- Adapters keep result and radio subtype: `mcp.result.success` /
  `mcp.result.failure`, `radio.auth_failure`, `radio.roam`.
- `group_density` is a separate 2×2 candidate. Peak `heuristic_density`
  stays the baseline. Square-root rounding uses a finer scale.
- README states these are adapters, not a claim that hosts already depend
  on this crate.

## 0.1.2 - 2026-09-19

- Checkpoint/replay and compatible H-CMS pane merge.
- Typed Weavatrix and RadioChron adapters; scan visibility is not communication.
- Candidate explanations stay unverified and are never security findings.
- Forty acceptance tests for identity, time, budgets, and merge.

## 0.1.1 - 2026-09-17

- README states this crate is not a repository engine, MCP host, or
  AnoGraph port.

## 0.1.0 - 2026-09-17

- First `no_std` + `alloc` contract for typed interaction events.
- Exact bounded windows with dedup, watermark, and lateness.
- H-CMS counters with full-key hashing (no modulo-b ID periodicity).
- Heuristic density score plus the AnoGraph matrix witness as a regression.
