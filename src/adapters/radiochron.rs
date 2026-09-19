//! `RadioChron` projections. Scan visibility is not a communication edge.

use crate::model::{
    EntityId, EventKey, EventPhase, IngestError, InteractionEvent, Quality, RelationProfile,
    ScopeId,
};
use alloc::string::String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioKind {
    Association,
    AuthFailure,
    Roam,
    ScanVisibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadioObservation {
    pub producer: String,
    pub boot_epoch: u64,
    pub sequence: u64,
    pub kind: RadioKind,
    pub scope: String,
    pub device: String,
    pub access_point: String,
    pub event_time: u64,
    pub evidence: String,
    pub capture_complete: bool,
    pub scan_policy_changed: bool,
    pub identity_confidence: u8,
}

/// # Errors
/// Rejects zero-weight construction. Scan rows use `radio.visibility`.
pub fn to_event(observation: &RadioObservation) -> Result<InteractionEvent, IngestError> {
    Ok(InteractionEvent {
        event_key: EventKey {
            producer: observation.producer.clone(),
            boot_epoch: observation.boot_epoch,
            sequence: observation.sequence,
            phase: EventPhase::Request,
        },
        scope: ScopeId(observation.scope.clone()),
        relation: RelationProfile::new(relation_name(observation.kind), "count"),
        source: EntityId(observation.device.clone()),
        target: EntityId(observation.access_point.clone()),
        event_time: observation.event_time,
        observed_at: None,
        weight: 1,
        evidence: observation.evidence.clone(),
        quality: Quality {
            sampled: observation.scan_policy_changed,
            identity_confidence: observation.identity_confidence,
            gaps: !observation.capture_complete,
            exposure: 100,
        },
    })
}

#[must_use]
pub fn relation_name(kind: RadioKind) -> &'static str {
    match kind {
        RadioKind::Association | RadioKind::AuthFailure | RadioKind::Roam => "radio.association",
        RadioKind::ScanVisibility => "radio.visibility",
    }
}

#[must_use]
pub fn invents_communication(kind: RadioKind) -> bool {
    matches!(kind, RadioKind::ScanVisibility)
}
