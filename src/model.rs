use alloc::string::String;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScopeId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntityId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventPhase {
    Request,
    ReportedResult,
    VerifiedEffect,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventKey {
    pub producer: String,
    pub boot_epoch: u64,
    pub sequence: u64,
    pub phase: EventPhase,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RelationProfile {
    pub name: String,
    pub weight_unit: String,
}

impl RelationProfile {
    #[must_use]
    pub fn new(name: &str, weight_unit: &str) -> Self {
        Self {
            name: name.into(),
            weight_unit: weight_unit.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quality {
    pub sampled: bool,
    pub identity_confidence: u8,
}

impl Default for Quality {
    fn default() -> Self {
        Self {
            sampled: false,
            identity_confidence: 100,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionEvent {
    pub event_key: EventKey,
    pub scope: ScopeId,
    pub relation: RelationProfile,
    pub source: EntityId,
    pub target: EntityId,
    pub event_time: u64,
    pub observed_at: Option<u64>,
    pub weight: u64,
    pub evidence: String,
    pub quality: Quality,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestError {
    ZeroWeight,
    Overflow,
    Late { event_time: u64, watermark: u64 },
    Budget(&'static str),
    ClosedWindow,
}
