mod common;

use common::event;
use weavatrix_stream::{Budget, EventPhase, StreamWindow};

#[test]
fn one_event_lands_in_its_scope_and_relation() {
    let mut window = StreamWindow::exact(Budget::small()).unwrap();
    let item = event("proj", "skill", "tool-a", 10, 1, 1, EventPhase::Request);
    assert!(window.ingest(&item).unwrap());
    assert_eq!(
        window.exact_count(&item.scope, &item.relation, &item.source, &item.target),
        Some(1)
    );
}

#[test]
fn same_name_entities_on_two_servers_stay_apart() {
    let mut window = StreamWindow::exact(Budget::small()).unwrap();
    let left = event("server-a", "skill", "search", 10, 1, 1, EventPhase::Request);
    let right = event("server-b", "skill", "search", 10, 2, 1, EventPhase::Request);
    window.ingest(&left).unwrap();
    window.ingest(&right).unwrap();
    assert_eq!(
        window.exact_count(&left.scope, &left.relation, &left.source, &left.target),
        Some(1)
    );
    assert_eq!(
        window.exact_count(&right.scope, &right.relation, &right.source, &right.target),
        Some(1)
    );
}

#[test]
fn other_tenant_count_is_absent() {
    let mut window = StreamWindow::exact(Budget::small()).unwrap();
    let item = event("tenant-a", "dev", "ap", 10, 1, 1, EventPhase::Request);
    window.ingest(&item).unwrap();
    let alien = event("tenant-b", "dev", "ap", 10, 1, 1, EventPhase::Request);
    assert_eq!(
        window.exact_count(&alien.scope, &alien.relation, &alien.source, &alien.target),
        None
    );
}

#[test]
fn duplicate_event_key_does_not_increase_count() {
    let mut window = StreamWindow::exact(Budget::small()).unwrap();
    let item = event("proj", "a", "b", 10, 1, 1, EventPhase::Request);
    assert!(window.ingest(&item).unwrap());
    assert!(!window.ingest(&item).unwrap());
    assert_eq!(
        window.exact_count(&item.scope, &item.relation, &item.source, &item.target),
        Some(1)
    );
}

#[test]
fn same_timestamp_different_sequence_counts_twice() {
    let mut window = StreamWindow::exact(Budget::small()).unwrap();
    let first = event("proj", "a", "b", 10, 1, 1, EventPhase::Request);
    let second = event("proj", "a", "b", 10, 2, 1, EventPhase::Request);
    window.ingest(&first).unwrap();
    window.ingest(&second).unwrap();
    assert_eq!(
        window.exact_count(&first.scope, &first.relation, &first.source, &first.target),
        Some(2)
    );
}

#[test]
fn request_and_result_do_not_share_invocation_count() {
    let mut window = StreamWindow::exact(Budget::small()).unwrap();
    let request = event("proj", "a", "b", 10, 1, 1, EventPhase::Request);
    let mut result = event("proj", "a", "b", 11, 1, 1, EventPhase::ReportedResult);
    result.relation = weavatrix_stream::RelationProfile::new("mcp.result", "count");
    window.ingest(&request).unwrap();
    window.ingest(&result).unwrap();
    assert_eq!(
        window.exact_count(
            &request.scope,
            &request.relation,
            &request.source,
            &request.target
        ),
        Some(1)
    );
}

#[test]
fn retry_uses_its_own_sequence() {
    let mut window = StreamWindow::exact(Budget::small()).unwrap();
    let first = event("proj", "a", "b", 10, 1, 1, EventPhase::Request);
    let retry = event("proj", "a", "b", 12, 2, 1, EventPhase::Request);
    window.ingest(&first).unwrap();
    window.ingest(&retry).unwrap();
    assert_eq!(
        window.exact_count(&first.scope, &first.relation, &first.source, &first.target),
        Some(2)
    );
}

#[test]
fn boot_reset_does_not_collide_with_old_sequence() {
    let mut window = StreamWindow::exact(Budget::small()).unwrap();
    let old = event("proj", "a", "b", 10, 1, 1, EventPhase::Request);
    let reset = event("proj", "a", "b", 20, 1, 2, EventPhase::Request);
    window.ingest(&old).unwrap();
    window.ingest(&reset).unwrap();
    assert_eq!(
        window.exact_count(&old.scope, &old.relation, &old.source, &old.target),
        Some(2)
    );
}

#[test]
fn collision_heavy_input_does_not_invent_exact_pairs() {
    let mut window = StreamWindow::exact(Budget::small()).unwrap();
    for sequence in 1..=8 {
        let item = event(
            "proj",
            "src",
            &format!("dst-{sequence}"),
            10,
            sequence,
            1,
            EventPhase::Request,
        );
        window.ingest(&item).unwrap();
    }
    let ghost = event("proj", "src", "never-seen", 10, 99, 1, EventPhase::Request);
    assert_eq!(
        window.exact_count(&ghost.scope, &ghost.relation, &ghost.source, &ghost.target),
        None
    );
}

#[test]
fn exact_backend_matches_manual_count() {
    let mut window = StreamWindow::exact(Budget::small()).unwrap();
    for sequence in 1..=5 {
        window
            .ingest(&event(
                "proj",
                "a",
                "b",
                10 + sequence,
                sequence,
                1,
                EventPhase::Request,
            ))
            .unwrap();
    }
    let item = event("proj", "a", "b", 10, 1, 1, EventPhase::Request);
    assert_eq!(
        window.exact_count(&item.scope, &item.relation, &item.source, &item.target),
        Some(5)
    );
}
