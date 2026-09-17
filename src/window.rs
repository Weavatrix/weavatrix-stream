use crate::budget::Budget;
use crate::exact::ExactWindow;
use crate::hcms::HcmsWindow;
use crate::model::{EntityId, IngestError, InteractionEvent, RelationProfile, ScopeId};

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
}
