use crate::budget::Budget;
use crate::exact::ExactWindow;
use crate::hcms::HcmsWindow;
use crate::model::{
    EntityId, IngestError, InteractionEvent, RelationProfile, ScopeId, WindowCheckpoint,
};
use crate::score::{DensityScore, group_density, heuristic_density};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Exact,
    Hcms,
}

#[derive(Debug, Clone)]
pub enum StreamWindow {
    Exact(ExactWindow),
    Hcms {
        exact: ExactWindow,
        sketch: HcmsWindow,
    },
}

impl StreamWindow {
    /// # Errors
    /// Returns [`crate::BudgetError`] when the budget is invalid.
    pub fn exact(budget: Budget) -> Result<Self, crate::BudgetError> {
        Ok(Self::Exact(ExactWindow::new(budget)?))
    }

    /// # Errors
    /// Returns [`crate::BudgetError`] when the budget is invalid.
    pub fn hcms(budget: Budget, seed: u64) -> Result<Self, crate::BudgetError> {
        Ok(Self::Hcms {
            exact: ExactWindow::new(budget)?,
            sketch: HcmsWindow::new(budget, seed)?,
        })
    }

    pub fn advance_watermark(&mut self, watermark: u64) {
        match self {
            Self::Exact(exact) => exact.advance_watermark(watermark),
            Self::Hcms { exact, sketch } => {
                exact.advance_watermark(watermark);
                sketch.advance_watermark(watermark);
            }
        }
    }

    /// # Errors
    /// Returns [`IngestError`] when the event is rejected.
    pub fn ingest(&mut self, event: &InteractionEvent) -> Result<bool, IngestError> {
        match self {
            Self::Exact(exact) => exact.ingest(event),
            Self::Hcms { exact, sketch } => {
                let rollback = exact.clone();
                let accepted = exact.ingest(event)?;
                if accepted && let Err(error) = sketch.ingest(event) {
                    *exact = rollback;
                    return Err(error);
                }
                Ok(accepted)
            }
        }
    }

    #[must_use]
    pub fn exact_count(
        &self,
        scope: &ScopeId,
        relation: &RelationProfile,
        source: &EntityId,
        target: &EntityId,
    ) -> Option<u64> {
        match self {
            Self::Exact(exact) | Self::Hcms { exact, .. } => {
                exact.count(scope, relation, source, target)
            }
        }
    }

    #[must_use]
    pub fn kind(&self) -> BackendKind {
        match self {
            Self::Exact(_) => BackendKind::Exact,
            Self::Hcms { .. } => BackendKind::Hcms,
        }
    }

    pub fn close(&mut self) {
        match self {
            Self::Exact(exact) => exact.close(),
            Self::Hcms { exact, sketch } => {
                exact.close();
                sketch.close();
            }
        }
    }

    #[must_use]
    pub fn checkpoint(&self) -> WindowCheckpoint {
        match self {
            Self::Exact(exact) => exact.checkpoint(),
            Self::Hcms { exact, sketch } => {
                let watermark = exact.watermark().max(sketch.watermark());
                let mut snap = exact.checkpoint();
                let sketch_snap = sketch.checkpoint();
                snap.seed = sketch_snap.seed;
                snap.counters = sketch_snap.counters;
                snap.watermark = watermark;
                snap.closed = snap.closed || sketch_snap.closed;
                snap
            }
        }
    }

    /// # Errors
    /// Returns [`IngestError::Checkpoint`] when the snapshot cannot be replayed.
    pub fn restore_exact(budget: Budget, snapshot: WindowCheckpoint) -> Result<Self, IngestError> {
        Ok(Self::Exact(ExactWindow::restore(budget, snapshot)?))
    }

    /// # Errors
    /// Returns [`IngestError::Checkpoint`] when the snapshot cannot be replayed.
    pub fn restore_hcms(
        budget: Budget,
        seed: u64,
        snapshot: WindowCheckpoint,
    ) -> Result<Self, IngestError> {
        let sketch = HcmsWindow::restore(budget, seed, snapshot.clone())?;
        let mut exact_snap = snapshot;
        exact_snap.counters = None;
        Ok(Self::Hcms {
            exact: ExactWindow::restore(budget, exact_snap)?,
            sketch,
        })
    }

    /// # Errors
    /// Returns [`IngestError::Checkpoint`] when sketches are incompatible.
    pub fn try_merge(&mut self, other: &Self) -> Result<(), IngestError> {
        let mut next = self.clone();
        match (&mut next, other) {
            (
                Self::Hcms {
                    exact: left_exact,
                    sketch: left,
                },
                Self::Hcms {
                    exact: right_exact,
                    sketch: right,
                },
            ) => {
                left.try_merge(right)?;
                left_exact.try_merge(right_exact)?;
                let watermark = left_exact.watermark().max(left.watermark());
                left_exact.advance_watermark(watermark);
                left.advance_watermark(watermark);
            }
            (Self::Exact(left), Self::Exact(right)) => left.try_merge(right)?,
            _ => return Err(IngestError::Checkpoint("merge-backend")),
        }
        *self = next;
        Ok(())
    }

    #[must_use]
    pub fn score(&self) -> Option<DensityScore> {
        match self {
            Self::Exact(_) => None,
            Self::Hcms { sketch, .. } => sketch
                .replica(0)
                .map(|matrix| heuristic_density(matrix, sketch.width())),
        }
    }

    #[must_use]
    pub fn group_score(&self) -> Option<DensityScore> {
        match self {
            Self::Exact(_) => None,
            Self::Hcms { sketch, .. } => sketch
                .replica(0)
                .map(|matrix| group_density(matrix, sketch.width())),
        }
    }
}
