use crate::budget::Budget;
use crate::model::{EntityId, EventKey, IngestError, InteractionEvent, RelationProfile, ScopeId};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct ExactWindow {
    budget: Budget,
    watermark: u64,
    closed: bool,
    counts: BTreeMap<(ScopeId, RelationProfile, EntityId, EntityId), u64>,
    dedup: BTreeSet<EventKey>,
    witnesses: Vec<(EntityId, EntityId, alloc::string::String)>,
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
        if self.closed {
            return Err(IngestError::ClosedWindow);
        }
        if event.weight == 0 {
            return Err(IngestError::ZeroWeight);
        }
        if event.event_time + self.budget.lateness < self.watermark {
            return Err(IngestError::Late {
                event_time: event.event_time,
                watermark: self.watermark,
            });
        }
        if !self.dedup.insert(event.event_key.clone()) {
            return Ok(false);
        }
        if self.dedup.len() > self.budget.max_dedup {
            self.dedup.remove(&event.event_key);
            return Err(IngestError::Budget("dedup"));
        }
        let key = (
            event.scope.clone(),
            event.relation.clone(),
            event.source.clone(),
            event.target.clone(),
        );
        if !self.counts.contains_key(&key) && self.counts.len() >= self.budget.max_pairs {
            self.dedup.remove(&event.event_key);
            return Err(IngestError::Budget("pairs"));
        }
        let slot = self.counts.entry(key).or_insert(0);
        *slot = slot
            .checked_add(event.weight)
            .ok_or(IngestError::Overflow)?;
        if self.witnesses.len() < self.budget.max_witnesses {
            self.witnesses.push((
                event.source.clone(),
                event.target.clone(),
                event.evidence.clone(),
            ));
        }
        Ok(true)
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
}
