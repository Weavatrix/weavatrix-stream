mod common;

use common::event;
use weavatrix_stream::adapters::radiochron::{RadioKind, RadioObservation, to_event};
use weavatrix_stream::{Budget, EventPhase, IngestError, Quality, StreamWindow};

#[test]
fn late_event_within_lateness_is_accepted() {
    let mut window = StreamWindow::exact(Budget::small()).unwrap();
    window.advance_watermark(1_500);
    let item = event("proj", "a", "b", 600, 1, 1, EventPhase::Request);
    assert!(window.ingest(&item).is_ok());
}

#[test]
fn too_late_event_is_reported() {
    let mut window = StreamWindow::exact(Budget::small()).unwrap();
    window.advance_watermark(5_000);
    let item = event("proj", "a", "b", 10, 1, 1, EventPhase::Request);
    assert!(matches!(
        window.ingest(&item),
        Err(IngestError::Late { .. })
    ));
}

#[test]
fn watermark_does_not_roll_back() {
    let mut window = StreamWindow::exact(Budget::small()).unwrap();
    window.advance_watermark(4_000);
    window.advance_watermark(1_000);
    let item = event("proj", "a", "b", 10, 1, 1, EventPhase::Request);
    assert!(matches!(
        window.ingest(&item),
        Err(IngestError::Late { .. })
    ));
}

#[test]
fn timestamp_gap_keeps_exact_counts() {
    let mut window = StreamWindow::exact(Budget::small()).unwrap();
    let early = event("proj", "a", "b", 10, 1, 1, EventPhase::Request);
    let late = event("proj", "a", "b", 10_000, 2, 1, EventPhase::Request);
    window.ingest(&early).unwrap();
    window.ingest(&late).unwrap();
    assert_eq!(
        window.exact_count(&early.scope, &early.relation, &early.source, &early.target),
        Some(2)
    );
}

#[test]
fn missing_capture_forbids_strong_absence() {
    let observation = RadioObservation {
        producer: "sensor".into(),
        boot_epoch: 1,
        sequence: 1,
        kind: RadioKind::ScanVisibility,
        scope: "site".into(),
        device: "phone".into(),
        access_point: "ap-1".into(),
        event_time: 10,
        evidence: "scan-1".into(),
        capture_complete: false,
        scan_policy_changed: false,
        identity_confidence: 100,
    };
    let event = to_event(&observation).unwrap();
    assert!(event.quality.forbids_strong_absence());
}

#[test]
fn scan_policy_change_is_quality_context() {
    let observation = RadioObservation {
        producer: "sensor".into(),
        boot_epoch: 1,
        sequence: 2,
        kind: RadioKind::ScanVisibility,
        scope: "site".into(),
        device: "phone".into(),
        access_point: "ap-1".into(),
        event_time: 20,
        evidence: "scan-2".into(),
        capture_complete: true,
        scan_policy_changed: true,
        identity_confidence: 100,
    };
    let event = to_event(&observation).unwrap();
    assert!(event.quality.sampled);
}

#[test]
fn zero_weight_is_rejected() {
    let mut window = StreamWindow::exact(Budget::small()).unwrap();
    let mut item = event("proj", "a", "b", 10, 1, 1, EventPhase::Request);
    item.weight = 0;
    assert!(matches!(window.ingest(&item), Err(IngestError::ZeroWeight)));
}

#[test]
fn invalid_budget_is_rejected() {
    let mut budget = Budget::small();
    budget.width = 3;
    assert!(budget.validate().is_err());
    budget = Budget::small();
    budget.replicas = 0;
    assert!(budget.validate().is_err());
}

#[test]
fn seed_survives_checkpoint() {
    let mut window = StreamWindow::hcms(Budget::small(), 0xFEED_FACE).unwrap();
    let item = event("proj", "a", "b", 10, 1, 1, EventPhase::Request);
    window.ingest(&item).unwrap();
    assert_eq!(window.checkpoint().seed, 0xFEED_FACE);
}

#[test]
fn idle_sensor_quality_is_not_healthy_zero() {
    let quality = Quality {
        sampled: false,
        identity_confidence: 100,
        gaps: true,
        exposure: 0,
    };
    assert!(quality.forbids_strong_absence());
}
