//! Map `Weavatrix`/`GrantTap` observations to ingest events.

use crate::model::{
    EntityId, EventKey, EventPhase, IngestError, InteractionEvent, Quality, RelationProfile,
    ScopeId,
};
use alloc::string::String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostObservation {
    pub producer: String,
    pub boot_epoch: u64,
    pub sequence: u64,
    pub phase: EventPhase,
    pub scope: String,
    pub source: String,
    pub target: String,
    pub event_time: u64,
    pub weight: u64,
    pub evidence: String,
    pub result: ObservationResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationResult {
    Reported,
    Success,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    SuccessIsNotEffect,
    Ingest(IngestError),
}

/// Invocation counts use the request phase only. A reported success is not an effect.
#[must_use]
pub fn invocation_event(observation: &HostObservation) -> Option<InteractionEvent> {
    if observation.phase != EventPhase::Request {
        return None;
    }
    to_event(observation).ok()
}

/// # Errors
/// Rejects a success result labeled as a verified effect.
pub fn to_event(observation: &HostObservation) -> Result<InteractionEvent, AdapterError> {
    if observation.result == ObservationResult::Success
        && observation.phase == EventPhase::VerifiedEffect
    {
        return Err(AdapterError::SuccessIsNotEffect);
    }
    if observation.weight == 0 {
        return Err(AdapterError::Ingest(IngestError::ZeroWeight));
    }
    Ok(InteractionEvent {
        event_key: EventKey {
            producer: observation.producer.clone(),
            boot_epoch: observation.boot_epoch,
            sequence: observation.sequence,
            phase: observation.phase,
        },
        scope: ScopeId(observation.scope.clone()),
        relation: relation_for(observation.phase),
        source: EntityId(observation.source.clone()),
        target: EntityId(observation.target.clone()),
        event_time: observation.event_time,
        observed_at: None,
        weight: observation.weight,
        evidence: observation.evidence.clone(),
        quality: Quality::default(),
    })
}

fn relation_for(phase: EventPhase) -> RelationProfile {
    match phase {
        EventPhase::Request => RelationProfile::new("mcp.invocation", "count"),
        EventPhase::ReportedResult => RelationProfile::new("mcp.result", "count"),
        EventPhase::VerifiedEffect => RelationProfile::new("mcp.effect", "count"),
    }
}
