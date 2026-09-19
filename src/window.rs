use crate::budget::Budget;
use crate::exact::ExactWindow;
use crate::hcms::HcmsWindow;
use crate::model::{
    EntityId, IngestError, InteractionEvent, RelationProfile, ScopeId, WindowCheckpoint,
};
use crate::score::{DensityScore, heuristic_density};

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
                let accepted = exact.ingest(event)?;
                if accepted {
                    sketch.ingest(event)?;
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
                let mut snap = exact.checkpoint();
                let sketch_snap = sketch.checkpoint();
                snap.seed = sketch_snap.seed;
                snap.counters = sketch_snap.counters;
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
        match (self, other) {
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
                for (scope, relation, source, target, count) in right_exact.exported_pairs() {
                    left_exact.add_restored(scope, relation, source, target, count)?;
                }
                Ok(())
            }
            _ => Err(IngestError::Checkpoint("merge-backend")),
        }
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
}
