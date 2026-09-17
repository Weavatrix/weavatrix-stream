# weavatrix-stream

Bounded streaming aggregates for typed `source → target` interactions.

This crate is **not Weavatrix**. It does not parse repositories, speak MCP,
or compete with Serena, Repomix, or ripgrep. It is a `no_std` + `alloc`
counter/sketch library. Hosts (Weavatrix, RadioChron, later a service
wrapper) supply time, identity, I/O, and persistence. The engine must not
take a runtime dependency that pulls this crate into language adapters.

Two backends share one ingest contract:

- **Exact** windows store pair counts with hard cardinality limits.
- **H-CMS** stores replica matrices with full-key hashing. Sketch buckets are
  not entity IDs.

Scores are heuristic candidates. They do not create canonical graph edges and
do not prove an attack. The shipped density helper does **not** claim the
AnoGraph 2-approximation.

## Status

S0–S3 of the streaming RFC: event/profile contract, exact windows, H-CMS
counters, and a checked heuristic scorer.
