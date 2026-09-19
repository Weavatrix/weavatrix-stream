mod common;

use common::event;
use weavatrix_stream::{Budget, EventPhase, IngestError, StreamWindow};

fn tight() -> Budget {
    Budget {
        max_pairs: 1,
        max_dedup: 8,
        max_witnesses: 2,
        lateness: 0,
        replicas: 2,
        width: 4,
    }
}

#[test]
fn rejected_exact_ingest_must_not_poison_dedup() {
    let mut window = StreamWindow::exact(Budget {
        max_pairs: 4,
        max_dedup: 8,
        max_witnesses: 2,
        lateness: 1_000,
        replicas: 2,
        width: 4,
    })
    .unwrap();
    let mut first = event("proj", "a", "b", 10, 1, 1, EventPhase::Request);
    first.weight = u64::MAX;
    window.ingest(&first).unwrap();
    let before = window.checkpoint();
    let mut overflow = event("proj", "a", "b", 11, 2, 1, EventPhase::Request);
    overflow.weight = 1;
    assert!(matches!(
        window.ingest(&overflow),
        Err(IngestError::Overflow)
    ));
    assert_eq!(window.checkpoint(), before);
    assert!(window.ingest(&overflow).is_err());
}

#[test]
fn rejected_merge_leaves_accepted_state() {
    let mut left = StreamWindow::hcms(tight(), 9).unwrap();
    let mut right = StreamWindow::hcms(tight(), 9).unwrap();
    left.ingest(&event("proj", "a", "b", 10, 1, 1, EventPhase::Request))
        .unwrap();
    right
        .ingest(&event("proj", "c", "d", 11, 2, 1, EventPhase::Request))
        .unwrap();
    let before = left.checkpoint();
    assert!(left.try_merge(&right).is_err());
    assert_eq!(left.checkpoint(), before);
}

#[test]
fn merge_then_restore_rejects_the_same_late_event() {
    let budget = Budget {
        max_pairs: 4,
        max_dedup: 8,
        max_witnesses: 4,
        lateness: 0,
        replicas: 2,
        width: 4,
    };
    let mut left = StreamWindow::hcms(budget, 9).unwrap();
    let mut right = StreamWindow::hcms(budget, 9).unwrap();
    left.ingest(&event("proj", "a", "b", 0, 1, 1, EventPhase::Request))
        .unwrap();
    right
        .ingest(&event("proj", "c", "d", 100, 2, 1, EventPhase::Request))
        .unwrap();
    left.advance_watermark(0);
    right.advance_watermark(100);
    left.try_merge(&right).unwrap();
    let late = event("proj", "e", "f", 50, 3, 1, EventPhase::Request);
    let live = left.ingest(&late);
    let mut restored = StreamWindow::restore_hcms(budget, 9, left.checkpoint()).unwrap();
    let from_checkpoint = restored.ingest(&late);
    assert!(matches!(live, Err(IngestError::Late { .. })));
    assert!(matches!(from_checkpoint, Err(IngestError::Late { .. })));
}

#[test]
fn merged_right_event_is_not_accepted_again() {
    let mut left = StreamWindow::hcms(Budget::small(), 9).unwrap();
    let mut right = StreamWindow::hcms(Budget::small(), 9).unwrap();
    let first = event("proj", "a", "b", 10, 1, 1, EventPhase::Request);
    let second = event("proj", "c", "d", 11, 2, 1, EventPhase::Request);
    left.ingest(&first).unwrap();
    right.ingest(&second).unwrap();
    left.try_merge(&right).unwrap();
    assert!(!left.ingest(&second).unwrap());
    assert_eq!(
        left.exact_count(
            &second.scope,
            &second.relation,
            &second.source,
            &second.target
        ),
        Some(1)
    );
}

#[test]
fn restore_rejects_an_oversize_witness_list() {
    let budget = Budget::small();
    let mut snap = StreamWindow::exact(budget).unwrap().checkpoint();
    snap.witnesses = (0..=budget.max_witnesses)
        .map(|index| {
            (
                weavatrix_stream::EntityId(index.to_string()),
                weavatrix_stream::EntityId("t".into()),
                "w".into(),
            )
        })
        .collect();
    assert!(matches!(
        StreamWindow::restore_exact(budget, snap),
        Err(IngestError::Budget("checkpoint"))
    ));
}
