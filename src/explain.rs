//! Candidate explanations. Scores never become graph edges or attacks.

use crate::model::{EntityId, Quality};
use crate::score::DensityScore;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub scope: String,
    pub relation: String,
    pub score: Option<DensityScore>,
    pub participants: Vec<(EntityId, EntityId)>,
    pub witnesses: Vec<String>,
    pub verification: Verification,
    pub quality: Quality,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verification {
    Corroborated,
    Partial,
    NotPerformed,
}

#[must_use]
pub fn from_witnesses(
    scope: &str,
    relation: &str,
    density: Option<DensityScore>,
    witnesses: &[(EntityId, EntityId, String)],
    quality: Quality,
) -> Candidate {
    Candidate {
        scope: scope.into(),
        relation: relation.into(),
        score: density,
        participants: witnesses
            .iter()
            .map(|(source, target, _)| (source.clone(), target.clone()))
            .collect(),
        witnesses: witnesses
            .iter()
            .map(|(_, _, evidence)| evidence.clone())
            .collect(),
        verification: Verification::NotPerformed,
        quality,
    }
}

#[must_use]
pub fn is_security_finding(candidate: &Candidate) -> bool {
    let _ = candidate;
    false
}
