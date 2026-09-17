#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    pub max_pairs: usize,
    pub max_dedup: usize,
    pub max_witnesses: usize,
    pub lateness: u64,
    pub replicas: usize,
    pub width: usize,
}

impl Budget {
    #[must_use]
    pub const fn small() -> Self {
        Self {
            max_pairs: 4_096,
            max_dedup: 16_384,
            max_witnesses: 64,
            lateness: 1_000,
            replicas: 4,
            width: 64,
        }
    }

    #[must_use]
    pub const fn sketch() -> Self {
        Self {
            max_pairs: 256,
            max_dedup: 16_384,
            max_witnesses: 32,
            lateness: 1_000,
            replicas: 4,
            width: 128,
        }
    }

    /// # Errors
    /// Returns [`BudgetError`] when dimensions cannot allocate a sketch.
    pub const fn validate(self) -> Result<Self, BudgetError> {
        if self.replicas == 0 || self.replicas > 16 {
            return Err(BudgetError::Replicas);
        }
        if self.width < 2 || self.width > 512 || !self.width.is_power_of_two() {
            return Err(BudgetError::Width);
        }
        if self.max_pairs == 0 || self.max_dedup == 0 {
            return Err(BudgetError::Cardinality);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetError {
    Replicas,
    Width,
    Cardinality,
}
