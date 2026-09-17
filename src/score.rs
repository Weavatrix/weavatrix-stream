//! Heuristic dense-region scores. No approximation ratio is claimed.

use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DensityScore {
    pub milli: u64,
    pub kind: &'static str,
    pub rows: usize,
    pub cols: usize,
}

/// Peak cell over sqrt(width), in thousandths. Candidate score, not OPT/2.
#[must_use]
pub fn heuristic_density(matrix: &[u64], width: usize) -> DensityScore {
    if width == 0 || matrix.len() < width.saturating_mul(width) {
        return DensityScore {
            milli: 0,
            kind: "heuristic",
            rows: 0,
            cols: 0,
        };
    }
    let mut peak = 0_u64;
    let mut at = (0, 0);
    for (index, value) in matrix.iter().enumerate().take(width * width) {
        if *value > peak {
            peak = *value;
            at = (index / width, index % width);
        }
    }
    DensityScore {
        milli: density_milli(peak, width),
        kind: "heuristic",
        rows: at.0,
        cols: at.1,
    }
}

/// Exact density of one supplied submatrix, in thousandths.
#[must_use]
pub fn submatrix_density(matrix: &[u64], width: usize, rows: &[usize], cols: &[usize]) -> u64 {
    if rows.is_empty() || cols.is_empty() {
        return 0;
    }
    let mut sum = 0_u64;
    for row in rows {
        for col in cols {
            if let Some(value) = matrix.get(row.saturating_mul(width).saturating_add(*col)) {
                sum = sum.saturating_add(*value);
            }
        }
    }
    density_milli(sum, rows.len().saturating_mul(cols.len()))
}

#[must_use]
pub fn anograph_counterexample(width: usize) -> Vec<u64> {
    let mut matrix = alloc::vec![0_u64; width.saturating_mul(width)];
    if width < 2 {
        return matrix;
    }
    for cell in matrix.iter_mut().take(width).skip(1) {
        *cell = 1;
    }
    for index in 1..width {
        if let Some(cell) = matrix.get_mut(index.saturating_mul(width).saturating_add(index)) {
            *cell = 2;
        }
    }
    matrix
}

fn density_milli(sum: u64, area: usize) -> u64 {
    let root = isqrt(u64::try_from(area).unwrap_or(u64::MAX)).max(1);
    sum.saturating_mul(1000) / root
}

fn isqrt(value: u64) -> u64 {
    if value <= 1 {
        return value;
    }
    let mut high = value;
    let mut low = value.saturating_add(1) / 2;
    while low < high {
        high = low;
        low = high.saturating_add(value / high) / 2;
    }
    high
}

#[cfg(test)]
mod tests {
    use super::{anograph_counterexample, heuristic_density, submatrix_density};

    #[test]
    fn counterexample_witness_is_stronger_than_a_false_two_approx() {
        let width = 64;
        let matrix = anograph_counterexample(width);
        let rows = [0_usize];
        let cols = (1..width).collect::<alloc::vec::Vec<_>>();
        let witness = submatrix_density(&matrix, width, &rows, &cols);
        let heuristic = heuristic_density(&matrix, width);
        assert!(
            witness > 7_000,
            "row-0 band density is a known feasible witness"
        );
        assert_eq!(heuristic.kind, "heuristic");
        assert!(
            witness > heuristic.milli.saturating_mul(2) || heuristic.milli <= witness,
            "do not advertise a guarantee the witness can refute"
        );
    }
}
