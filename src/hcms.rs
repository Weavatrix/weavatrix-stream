use crate::budget::Budget;
use crate::hash::{bucket, mix};
use crate::model::{CHECKPOINT_VERSION, IngestError, InteractionEvent, WindowCheckpoint};
use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct HcmsWindow {
    budget: Budget,
    watermark: u64,
    closed: bool,
    seed: u64,
    counters: Vec<u64>,
}

impl HcmsWindow {
    /// # Errors
    /// Returns [`crate::BudgetError`] when the budget is invalid.
    pub fn new(budget: Budget, seed: u64) -> Result<Self, crate::BudgetError> {
        let budget = budget.validate()?;
        let cells = budget
            .replicas
            .saturating_mul(budget.width)
            .saturating_mul(budget.width);
        Ok(Self {
            budget,
            watermark: 0,
            closed: false,
            seed,
            counters: alloc::vec![0; cells],
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
    /// Returns [`IngestError`] for invalid or late events.
    pub fn ingest(&mut self, event: &InteractionEvent) -> Result<(), IngestError> {
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
        let source = encode(&event.scope.0, &event.relation.name, &event.source.0);
        let target = encode(&event.scope.0, &event.relation.name, &event.target.0);
        for replica in 0..self.budget.replicas {
            let replica_seed = u64::try_from(replica).unwrap_or(0);
            let row = bucket(mix(self.seed ^ replica_seed, &source), self.budget.width);
            let col = bucket(
                mix(self.seed ^ replica_seed ^ 0xD1B5_4A32_D192_ED03, &target),
                self.budget.width,
            );
            let span = self.budget.width.saturating_mul(self.budget.width);
            let index = replica
                .saturating_mul(span)
                .saturating_add(row.saturating_mul(self.budget.width))
                .saturating_add(col);
            let slot = self
                .counters
                .get_mut(index)
                .ok_or(IngestError::Budget("sketch"))?;
            *slot = slot
                .checked_add(event.weight)
                .ok_or(IngestError::Overflow)?;
        }
        Ok(())
    }

    #[must_use]
    pub fn replica(&self, replica: usize) -> Option<&[u64]> {
        let span = self.budget.width * self.budget.width;
        let start = replica.checked_mul(span)?;
        self.counters.get(start..start + span)
    }

    #[must_use]
    pub fn width(&self) -> usize {
        self.budget.width
    }

    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    #[must_use]
    pub fn watermark(&self) -> u64 {
        self.watermark
    }

    /// # Errors
    /// Returns [`IngestError::Checkpoint`] when dimensions or seed disagree.
    pub fn try_merge(&mut self, other: &Self) -> Result<(), IngestError> {
        if self.seed != other.seed
            || self.budget.width != other.budget.width
            || self.budget.replicas != other.budget.replicas
            || self.counters.len() != other.counters.len()
        {
            return Err(IngestError::Checkpoint("incompatible-merge"));
        }
        for (slot, extra) in self.counters.iter_mut().zip(other.counters.iter()) {
            *slot = slot.checked_add(*extra).ok_or(IngestError::Overflow)?;
        }
        if other.watermark > self.watermark {
            self.watermark = other.watermark;
        }
        Ok(())
    }

    #[must_use]
    pub fn checkpoint(&self) -> WindowCheckpoint {
        WindowCheckpoint {
            version: CHECKPOINT_VERSION,
            seed: self.seed,
            watermark: self.watermark,
            closed: self.closed,
            width: self.budget.width,
            replicas: self.budget.replicas,
            pairs: Vec::new(),
            keys: Vec::new(),
            witnesses: Vec::new(),
            counters: Some(self.counters.clone()),
        }
    }

    /// # Errors
    /// Returns [`IngestError::Checkpoint`] when the snapshot does not match this sketch.
    pub fn restore(
        budget: Budget,
        seed: u64,
        snapshot: WindowCheckpoint,
    ) -> Result<Self, IngestError> {
        if snapshot.version != CHECKPOINT_VERSION {
            return Err(IngestError::Checkpoint("version"));
        }
        if snapshot.seed != seed
            || snapshot.width != budget.width
            || snapshot.replicas != budget.replicas
        {
            return Err(IngestError::Checkpoint("incompatible-restore"));
        }
        let counters = snapshot
            .counters
            .ok_or(IngestError::Checkpoint("missing-counters"))?;
        let expected = budget
            .replicas
            .saturating_mul(budget.width)
            .saturating_mul(budget.width);
        if counters.len() != expected {
            return Err(IngestError::Checkpoint("counter-shape"));
        }
        let budget = budget
            .validate()
            .map_err(|_| IngestError::Checkpoint("budget"))?;
        Ok(Self {
            budget,
            watermark: snapshot.watermark,
            closed: snapshot.closed,
            seed,
            counters,
        })
    }
}

fn encode(scope: &str, relation: &str, entity: &str) -> Vec<u8> {
    let mut out = Vec::from(scope.as_bytes());
    out.push(0);
    out.extend_from_slice(relation.as_bytes());
    out.push(0);
    out.extend_from_slice(entity.as_bytes());
    out
}
