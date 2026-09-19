mod common;

use common::event;
use weavatrix_stream::adapters::radiochron::{RadioKind, invents_communication, relation_name};
use weavatrix_stream::adapters::weavatrix::{
    HostObservation, ObservationResult, invocation_event, to_event,
};
use weavatrix_stream::explain::{from_witnesses, is_security_finding};
use weavatrix_stream::hash::{bucket, mix};
use weavatrix_stream::{
    Budget, EntityId, EventPhase, IngestError, Quality, StreamWindow, anograph_counterexample,
    heuristic_density, submatrix_density,
};

#[test]
fn ids_offset_by_width_do_not_collide_in_every_replica() {
    let width = 64_usize;
    let left = 7_u64.to_le_bytes();
    let right = (7_u64 + width as u64).to_le_bytes();
    let collisions = (0..8)
        .filter(|replica| {
            let seed = 0xA5A5_A5A5_A5A5_A5A5 ^ (*replica * 0x1111_1111_1111_1111);
            bucket(mix(seed, &left), width) == bucket(mix(seed, &right), width)
        })
        .count();
    assert!(collisions < 8);
}

#[test]
fn sketch_is_reproducible_with_the_same_seed() {
    let item = event("proj", "a", "b", 10, 1, 1, EventPhase::Request);
    let mut first = StreamWindow::hcms(Budget::small(), 7).unwrap();
    let mut second = StreamWindow::hcms(Budget::small(), 7).unwrap();
    first.ingest(&item).unwrap();
    second.ingest(&item).unwrap();
    assert_eq!(first.checkpoint().counters, second.checkpoint().counters);
}

#[test]
fn matrix_counterexample_stays_heuristic() {
    let width = 64;
    let matrix = anograph_counterexample(width);
    let rows = [0_usize];
    let cols = (1..width).collect::<Vec<_>>();
    let witness = submatrix_density(&matrix, width, &rows, &cols);
    let heuristic = heuristic_density(&matrix, width);
    assert!(witness > 7_000);
    assert_eq!(heuristic.kind, "heuristic");
    assert!(witness > heuristic.milli.saturating_mul(2) || heuristic.milli <= witness);
}

#[test]
fn empty_and_asymmetric_matrices_are_safe() {
    assert_eq!(heuristic_density(&[], 0).milli, 0);
    assert_eq!(heuristic_density(&[1, 2, 3], 2).milli, 0);
}

#[test]
fn dense_hub_is_not_a_security_finding() {
    let candidate = from_witnesses(
        "proj",
        "mcp.invocation",
        None,
        &[(
            EntityId("hub".into()),
            EntityId("spoke".into()),
            "evt".into(),
        )],
        Quality::default(),
    );
    assert!(!is_security_finding(&candidate));
}

#[test]
fn closed_window_rejects_ingest_and_still_scores() {
    let mut window = StreamWindow::hcms(Budget::small(), 1).unwrap();
    let item = event("proj", "a", "b", 10, 1, 1, EventPhase::Request);
    window.ingest(&item).unwrap();
    window.close();
    assert!(matches!(
        window.ingest(&item),
        Err(IngestError::ClosedWindow)
    ));
    assert!(window.score().is_some());
}

#[test]
fn pair_and_witness_registries_are_bounded() {
    let mut budget = Budget::small();
    budget.max_pairs = 2;
    budget.max_witnesses = 1;
    budget.max_dedup = 8;
    let mut window = StreamWindow::exact(budget).unwrap();
    window
        .ingest(&event("proj", "a", "b", 10, 1, 1, EventPhase::Request))
        .unwrap();
    window
        .ingest(&event("proj", "a", "c", 11, 2, 1, EventPhase::Request))
        .unwrap();
    let overflow = window.ingest(&event("proj", "a", "d", 12, 3, 1, EventPhase::Request));
    assert!(matches!(overflow, Err(IngestError::Budget("pairs"))));
}

#[test]
fn candidate_is_not_a_verified_effect() {
    let observation = HostObservation {
        producer: "host".into(),
        boot_epoch: 1,
        sequence: 1,
        phase: EventPhase::VerifiedEffect,
        scope: "proj".into(),
        source: "skill".into(),
        target: "tool".into(),
        event_time: 10,
        weight: 1,
        evidence: "evt".into(),
        result: ObservationResult::Success,
    };
    assert!(to_event(&observation).is_err());
    assert!(invocation_event(&observation).is_none());
}

#[test]
fn radio_scan_is_visibility_not_communication() {
    assert_eq!(relation_name(RadioKind::ScanVisibility), "radio.visibility");
    assert!(invents_communication(RadioKind::ScanVisibility));
    assert!(!invents_communication(RadioKind::Association));
}

#[test]
fn low_identity_confidence_is_not_one_device() {
    let quality = Quality {
        sampled: false,
        identity_confidence: 20,
        gaps: false,
        exposure: 100,
    };
    assert!(quality.forbids_strong_absence());
}
