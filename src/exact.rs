use crate::budget::Budget;
use crate::model::{
    CHECKPOINT_VERSION, EntityId, EventKey, IngestError, InteractionEvent, PairCount,
    RelationProfile, ScopeId, WindowCheckpoint,
};
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
    pub fn exported_pairs(&self) -> Vec<(ScopeId, RelationProfile, EntityId, EntityId, u64)> {
        self.counts
            .iter()
            .map(|(key, count)| {
                (
                    key.0.clone(),
                    key.1.clone(),
                    key.2.clone(),
                    key.3.clone(),
                    *count,
                )
            })
            .collect()
    }

    /// # Errors
    /// Returns [`IngestError`] when the restored pair overflows the budget.
    pub fn add_restored(
        &mut self,
        scope: ScopeId,
        relation: RelationProfile,
        source: EntityId,
        target: EntityId,
        count: u64,
    ) -> Result<(), IngestError> {
        let key = (scope, relation, source, target);
        if !self.counts.contains_key(&key) && self.counts.len() >= self.budget.max_pairs {
            return Err(IngestError::Budget("pairs"));
        }
        let slot = self.counts.entry(key).or_insert(0);
        *slot = slot.checked_add(count).ok_or(IngestError::Overflow)?;
        Ok(())
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
    pub fn checkpoint(&self) -> WindowCheckpoint {
        WindowCheckpoint {
            version: CHECKPOINT_VERSION,
            seed: 0,
            watermark: self.watermark,
            closed: self.closed,
            width: self.budget.width,
            replicas: self.budget.replicas,
            pairs: self
                .counts
                .iter()
                .map(|(key, count)| PairCount {
                    scope: key.0.clone(),
                    relation: key.1.clone(),
                    source: key.2.clone(),
                    target: key.3.clone(),
                    count: *count,
                })
                .collect(),
            keys: self.dedup.iter().cloned().collect(),
            witnesses: self.witnesses.clone(),
            counters: None,
        }
    }

    /// # Errors
    /// Returns [`IngestError::Checkpoint`] when the snapshot version or budget is wrong.
    pub fn restore(budget: Budget, snapshot: WindowCheckpoint) -> Result<Self, IngestError> {
        if snapshot.version != CHECKPOINT_VERSION {
            return Err(IngestError::Checkpoint("version"));
        }
        if snapshot.counters.is_some() {
            return Err(IngestError::Checkpoint("exact-cannot-hold-sketch"));
        }
        let budget = budget
            .validate()
            .map_err(|_| IngestError::Checkpoint("budget"))?;
        if snapshot.pairs.len() > budget.max_pairs || snapshot.keys.len() > budget.max_dedup {
            return Err(IngestError::Budget("checkpoint"));
        }
        let mut counts = BTreeMap::new();
        for pair in snapshot.pairs {
            counts.insert(
                (pair.scope, pair.relation, pair.source, pair.target),
                pair.count,
            );
        }
        Ok(Self {
            budget,
            watermark: snapshot.watermark,
            closed: snapshot.closed,
            counts,
            dedup: snapshot.keys.into_iter().collect(),
            witnesses: snapshot.witnesses,
        })
    }
}
