# weavatrix-stream

[![CI](https://github.com/Weavatrix/weavatrix-stream/actions/workflows/ci.yml/badge.svg)](https://github.com/Weavatrix/weavatrix-stream/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/weavatrix-stream.svg)](https://crates.io/crates/weavatrix-stream)
[![MSRV](https://img.shields.io/badge/MSRV-1.89.0-orange.svg)](https://www.rust-lang.org/)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**See where interactions concentrate — then hand the candidates back to a
graph that already knows the real entities.**

`weavatrix-stream` is a `no_std` + `alloc` crate for typed `source → target`
windows. It **provides adapters** for Weavatrix observations and RadioChron
captures. It does not pull those products in, and those hosts do not have to
depend on this crate to ship. It is **not** a repository parser, **not** an
MCP server, and **not** a port of `AnoGraph`.

```toml
[dependencies]
weavatrix-stream = "0.1.3"
```

## What you get

| Mode | Use it when | What it will not pretend |
| --- | --- | --- |
| Exact window | Small projects, tests, replay oracles | A pair you never ingested does not appear |
| H-CMS sketch | Large streams that cannot store every pair | A bucket is not a tool name or an AP |
| Checkpoint / merge | Restart, host/embedded replay, disjoint panes | A rejected merge or ingest leaves state unchanged. Overlap is an error. Restore keeps the next late/accept decision |
| Host adapters | `Weavatrix` observations, `RadioChron` captures | A reported success is not a verified effect. A Wi-Fi scan is `radio.visibility`, not communication |

Scores are heuristic candidates. They do not write canonical graph edges
and do not prove an attack. The density helper does **not** claim the
`AnoGraph` 2-approximation; the shipped counterexample matrix is a
regression against that claim.

## Ingest contract

Callers supply event time, watermark, identity, and persistence. The crate
does not open files, sockets, or a wall clock.

- Request, reported result, and verified effect are different relations.
- Duplicate `producer / boot / sequence / phase` keys do not increment.
- Late events after the watermark are errors, not silent drops.
- Zero and overflowing weights are rejected.
- Missing capture coverage forbids a “nothing happened” claim.

Acceptance tests under `tests/accept_*.rs` cover identity, time, budgets,
merge, and compositional invariants: a rejected operation leaves accepted
state unchanged; `restore(checkpoint(S))` keeps the next late/accept
decision; merge of disjoint panes matches a single replay. S6 — a
network service wrapper — stays out until a host needs shared state.

## Embed

```rust
use weavatrix_stream::{Budget, EventPhase, StreamWindow};

let mut window = StreamWindow::exact(Budget::small())?;
# let event = weavatrix_stream::InteractionEvent {
#     event_key: weavatrix_stream::EventKey {
#         producer: "demo".into(),
#         boot_epoch: 1,
#         sequence: 1,
#         phase: EventPhase::Request,
#     },
#     scope: weavatrix_stream::ScopeId("proj".into()),
#     relation: weavatrix_stream::RelationProfile::new("mcp.invocation", "count"),
#     source: weavatrix_stream::EntityId("skill".into()),
#     target: weavatrix_stream::EntityId("tool".into()),
#     event_time: 10,
#     observed_at: None,
#     weight: 1,
#     evidence: "evt-1".into(),
#     quality: weavatrix_stream::Quality::default(),
# };
window.ingest(&event)?;
# Ok::<(), weavatrix_stream::IngestError>(())
```

`RadioChron` and `Weavatrix` adapters live in `adapters`. They convert typed
host observations; they do not pull `weavatrix-rust` or a radio stack into
this crate.

## License

MIT.
