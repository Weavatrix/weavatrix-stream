use crate::budget::Budget;
use crate::hash::{bucket, mix};
use crate::model::{IngestError, InteractionEvent};
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
}

fn encode(scope: &str, relation: &str, entity: &str) -> Vec<u8> {
    let mut out = Vec::from(scope.as_bytes());
    out.push(0);
    out.extend_from_slice(relation.as_bytes());
    out.push(0);
    out.extend_from_slice(entity.as_bytes());
    out
}
