use weavatrix_stream::{
    EntityId, EventKey, EventPhase, InteractionEvent, Quality, RelationProfile, ScopeId,
};

pub fn event(
    scope: &str,
    source: &str,
    target: &str,
    time: u64,
    sequence: u64,
    boot: u64,
    phase: EventPhase,
) -> InteractionEvent {
    InteractionEvent {
        event_key: EventKey {
            producer: "test".into(),
            boot_epoch: boot,
            sequence,
            phase,
        },
        scope: ScopeId(scope.into()),
        relation: RelationProfile::new("mcp.invocation", "count"),
        source: EntityId(source.into()),
        target: EntityId(target.into()),
        event_time: time,
        observed_at: None,
        weight: 1,
        evidence: format!("evt-{sequence}"),
        quality: Quality::default(),
    }
}
