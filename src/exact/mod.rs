use crate::budget::Budget;
use crate::model::{EntityId, EventKey, IngestError, InteractionEvent, RelationProfile, ScopeId};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

mod store;

type PairKey = (ScopeId, RelationProfile, EntityId, EntityId);

#[derive(Debug, Clone)]
pub struct ExactWindow {
    budget: Budget,
    watermark: u64,
    closed: bool,
    counts: BTreeMap<PairKey, u64>,
    dedup: BTreeSet<EventKey>,
    witnesses: Vec<(EntityId, EntityId, alloc::string::String)>,
}

enum Admission {
    Duplicate,
    Accept { key: PairKey, next: u64 },
}

impl ExactWindow {
    /// # Errors
    /// Returns [`crate::BudgetError`] when the budget is invalid.
    pub fn new(budget: Budget) -> Result<Self, crate::BudgetError> {
        Ok(Self {
            budget: budget.validate()?,
            watermark: 0,
            closed: false,
            counts: BTreeMap::new(),
            dedup: BTreeSet::new(),
            witnesses: Vec::new(),
        })
    }

    pub fn advance_watermark(&mut self, watermark: u64) {
        if watermark > self.watermark {
            self.watermark = watermark;
        }
    }

    pub fn close(&mut self) {
        self.closed = true;
    }

    /// # Errors
    /// Returns [`IngestError`] for late, closed, or over-budget events.
    pub fn ingest(&mut self, event: &InteractionEvent) -> Result<bool, IngestError> {
        match self.admit(event)? {
            Admission::Duplicate => Ok(false),
            Admission::Accept { key, next } => {
                self.commit(event, key, next);
                Ok(true)
            }
        }
    }

    fn admit(&self, event: &InteractionEvent) -> Result<Admission, IngestError> {
        if self.closed {
            return Err(IngestError::ClosedWindow);
        }
        if event.weight == 0 {
            return Err(IngestError::ZeroWeight);
        }
        if event.event_time.saturating_add(self.budget.lateness) < self.watermark {
            return Err(IngestError::Late {
                event_time: event.event_time,
                watermark: self.watermark,
            });
        }
        if self.dedup.contains(&event.event_key) {
            return Ok(Admission::Duplicate);
        }
        if self.dedup.len() >= self.budget.max_dedup {
            return Err(IngestError::Budget("dedup"));
        }
        let key = (
            event.scope.clone(),
            event.relation.clone(),
            event.source.clone(),
            event.target.clone(),
        );
        let current = self.counts.get(&key).copied().unwrap_or(0);
        if current == 0 && self.counts.len() >= self.budget.max_pairs {
            return Err(IngestError::Budget("pairs"));
        }
        let next = current
            .checked_add(event.weight)
            .ok_or(IngestError::Overflow)?;
        Ok(Admission::Accept { key, next })
    }

    fn commit(&mut self, event: &InteractionEvent, key: PairKey, next: u64) {
        self.dedup.insert(event.event_key.clone());
        self.counts.insert(key, next);
        if self.witnesses.len() < self.budget.max_witnesses {
            self.witnesses.push((
                event.source.clone(),
                event.target.clone(),
                event.evidence.clone(),
            ));
        }
    }

    #[must_use]
    pub fn count(
        &self,
        scope: &ScopeId,
        relation: &RelationProfile,
        source: &EntityId,
        target: &EntityId,
    ) -> Option<u64> {
        self.counts
            .get(&(
                scope.clone(),
                relation.clone(),
                source.clone(),
                target.clone(),
            ))
            .copied()
    }

    #[must_use]
    pub fn pairs(&self) -> usize {
        self.counts.len()
    }

    #[must_use]
    pub fn witnesses(&self) -> &[(EntityId, EntityId, alloc::string::String)] {
        &self.witnesses
    }

    #[must_use]
    pub fn watermark(&self) -> u64 {
        self.watermark
    }

    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    #[must_use]
    pub fn checkpoint(&self) -> crate::model::WindowCheckpoint {
        store::checkpoint(self)
    }

    /// # Errors
    /// Returns [`IngestError::Checkpoint`] when the snapshot version or budget is wrong.
    pub fn restore(
        budget: Budget,
        snapshot: crate::model::WindowCheckpoint,
    ) -> Result<Self, IngestError> {
        store::restore(budget, snapshot)
    }

    /// # Errors
    /// Returns [`IngestError`] when panes overlap or the merge does not fit.
    pub fn try_merge(&mut self, other: &Self) -> Result<(), IngestError> {
        store::try_merge(self, other)
    }

    #[must_use]
    pub fn exported_pairs(&self) -> Vec<(ScopeId, RelationProfile, EntityId, EntityId, u64)> {
        store::exported_pairs(self)
    }
}
