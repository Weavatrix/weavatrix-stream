//! Bounded streaming aggregates for typed source→target interactions.
//!
//! This crate is `no_std` + `alloc`. It does not open files, sockets, or a wall
//! clock. Callers supply event time, watermarks, identity, and persistence.
//! Sketch scores are heuristic candidates, not proven graph edges or attacks.

#![no_std]

extern crate alloc;

pub mod budget;
pub mod exact;
pub mod hash;
pub mod hcms;
pub mod model;
pub mod score;
pub mod window;

pub use budget::{Budget, BudgetError};
pub use exact::ExactWindow;
pub use hcms::HcmsWindow;
pub use model::{
    EntityId, EventKey, EventPhase, IngestError, InteractionEvent, Quality, RelationProfile,
    ScopeId,
};
pub use score::{DensityScore, heuristic_density};
pub use window::{BackendKind, StreamWindow};

#[cfg(test)]
mod tests {
    use super::{
        Budget, EntityId, EventKey, EventPhase, InteractionEvent, Quality, RelationProfile,
        ScopeId, StreamWindow,
    };

    #[test]
    fn duplicate_event_key_does_not_increase_count() {
        let mut window = StreamWindow::exact(Budget::small()).unwrap();
        let event = sample("alpha", "tool-a", 10, 1);
        assert!(window.ingest(&event).is_ok());
        assert!(window.ingest(&event).is_ok());
        assert_eq!(
            window.exact_count(&event.scope, &event.relation, &event.source, &event.target),
            Some(1)
        );
    }

    #[test]
    fn same_name_entities_stay_in_their_scope() {
        let mut window = StreamWindow::exact(Budget::small()).unwrap();
        let mut left = sample("skill", "search", 10, 1);
        left.scope = ScopeId("server-a".into());
        let mut right = sample("skill", "search", 10, 2);
        right.scope = ScopeId("server-b".into());
        assert!(window.ingest(&left).is_ok());
        assert!(window.ingest(&right).is_ok());
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
    fn late_event_is_rejected_after_watermark() {
        let mut window = StreamWindow::exact(Budget::small()).unwrap();
        window.advance_watermark(5_000);
        let event = sample("alpha", "tool-a", 10, 1);
        assert!(matches!(
            window.ingest(&event),
            Err(super::IngestError::Late { .. })
        ));
    }

    fn sample(source: &str, target: &str, time: u64, sequence: u64) -> InteractionEvent {
        InteractionEvent {
            event_key: EventKey {
                producer: "test".into(),
                boot_epoch: 1,
                sequence,
                phase: EventPhase::Request,
            },
            scope: ScopeId("proj".into()),
            relation: RelationProfile::new("mcp.invocation", "count"),
            source: EntityId(source.into()),
            target: EntityId(target.into()),
            event_time: time,
            observed_at: None,
            weight: 1,
            evidence: "evt-1".into(),
            quality: Quality::default(),
        }
    }
}
