mod common;

use common::event;
use weavatrix_stream::{
    Budget, CHECKPOINT_VERSION, EventPhase, IngestError, StreamWindow, WindowCheckpoint,
};

#[test]
fn checkpoint_replay_neither_loses_nor_duplicates() {
    let mut window = StreamWindow::exact(Budget::small()).unwrap();
    let first = event("proj", "a", "b", 10, 1, 1, EventPhase::Request);
    window.ingest(&first).unwrap();
    let snap = window.checkpoint();
    let mut restored = StreamWindow::restore_exact(Budget::small(), snap).unwrap();
    assert!(!restored.ingest(&first).unwrap());
    let second = event("proj", "a", "b", 11, 2, 1, EventPhase::Request);
    assert!(restored.ingest(&second).unwrap());
    assert_eq!(
        restored.exact_count(&first.scope, &first.relation, &first.source, &first.target),
        Some(2)
    );
}

#[test]
fn old_checkpoint_version_is_rejected() {
    let mut snap = WindowCheckpoint {
        version: CHECKPOINT_VERSION + 1,
        seed: 0,
        watermark: 0,
        closed: false,
        width: 64,
        replicas: 4,
        pairs: Vec::new(),
        keys: Vec::new(),
        witnesses: Vec::new(),
        counters: None,
    };
    assert!(matches!(
        StreamWindow::restore_exact(Budget::small(), snap.clone()),
        Err(IngestError::Checkpoint("version"))
    ));
    snap.version = 0;
    assert!(StreamWindow::restore_exact(Budget::small(), snap).is_err());
}

#[test]
fn merge_of_disjoint_panes_matches_aggregate() {
    let left_event = event("proj", "a", "b", 10, 1, 1, EventPhase::Request);
    let right_event = event("proj", "c", "d", 11, 2, 1, EventPhase::Request);
    let mut left = StreamWindow::hcms(Budget::small(), 9).unwrap();
    let mut right = StreamWindow::hcms(Budget::small(), 9).unwrap();
    let mut both = StreamWindow::hcms(Budget::small(), 9).unwrap();
    left.ingest(&left_event).unwrap();
    right.ingest(&right_event).unwrap();
    both.ingest(&left_event).unwrap();
    both.ingest(&right_event).unwrap();
    left.try_merge(&right).unwrap();
    assert_eq!(
        left.exact_count(
            &left_event.scope,
            &left_event.relation,
            &left_event.source,
            &left_event.target
        ),
        Some(1)
    );
    assert_eq!(
        left.exact_count(
            &right_event.scope,
            &right_event.relation,
            &right_event.source,
            &right_event.target
        ),
        Some(1)
    );
    assert_eq!(left.checkpoint().counters, both.checkpoint().counters);
}

#[test]
fn merge_of_a_different_seed_is_rejected() {
    let mut left = StreamWindow::hcms(Budget::small(), 1).unwrap();
    let right = StreamWindow::hcms(Budget::small(), 2).unwrap();
    assert!(matches!(
        left.try_merge(&right),
        Err(IngestError::Checkpoint("incompatible-merge"))
    ));
}

#[test]
fn local_scores_are_not_used_as_a_global_score() {
    let mut left = StreamWindow::hcms(Budget::small(), 3).unwrap();
    let mut right = StreamWindow::hcms(Budget::small(), 3).unwrap();
    left.ingest(&event("proj", "a", "b", 10, 1, 1, EventPhase::Request))
        .unwrap();
    right
        .ingest(&event("proj", "c", "d", 11, 2, 1, EventPhase::Request))
        .unwrap();
    let left_score = left.score().unwrap().milli;
    let right_score = right.score().unwrap().milli;
    left.try_merge(&right).unwrap();
    let merged = left.score().unwrap().milli;
    assert_ne!(merged, left_score.saturating_add(right_score));
}

#[test]
fn repeated_repository_scan_is_not_a_burst() {
    let mut window = StreamWindow::exact(Budget::small()).unwrap();
    let item = event("proj", "file", "symbol", 10, 1, 1, EventPhase::Request);
    window.ingest(&item).unwrap();
    window.ingest(&item).unwrap();
    window.ingest(&item).unwrap();
    assert_eq!(
        window.exact_count(&item.scope, &item.relation, &item.source, &item.target),
        Some(1)
    );
}

#[test]
fn static_edge_removal_is_not_a_negative_increment() {
    let mut item = event("proj", "a", "b", 10, 1, 1, EventPhase::Request);
    item.weight = 0;
    let mut window = StreamWindow::hcms(Budget::small(), 4).unwrap();
    assert!(matches!(window.ingest(&item), Err(IngestError::ZeroWeight)));
}

#[test]
fn explanation_does_not_write_graph_edges() {
    let mut window = StreamWindow::exact(Budget::small()).unwrap();
    let item = event("proj", "a", "b", 10, 1, 1, EventPhase::Request);
    window.ingest(&item).unwrap();
    let snap = window.checkpoint();
    assert!(snap.pairs.len() <= 1);
}

#[test]
fn invocation_adapter_skips_reported_results() {
    use weavatrix_stream::adapters::weavatrix::{
        HostObservation, ObservationResult, invocation_event,
    };
    let observation = HostObservation {
        producer: "host".into(),
        boot_epoch: 1,
        sequence: 4,
        phase: EventPhase::ReportedResult,
        scope: "proj".into(),
        source: "skill".into(),
        target: "tool".into(),
        event_time: 10,
        weight: 1,
        evidence: "evt".into(),
        result: ObservationResult::Success,
    };
    assert!(invocation_event(&observation).is_none());
}

#[test]
fn host_and_embedded_replay_the_same_counts() {
    let item = event("proj", "a", "b", 10, 1, 1, EventPhase::Request);
    let mut host = StreamWindow::exact(Budget::small()).unwrap();
    host.ingest(&item).unwrap();
    let embedded = StreamWindow::restore_exact(Budget::small(), host.checkpoint()).unwrap();
    assert_eq!(
        host.exact_count(&item.scope, &item.relation, &item.source, &item.target),
        embedded.exact_count(&item.scope, &item.relation, &item.source, &item.target)
    );
}
