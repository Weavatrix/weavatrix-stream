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

/// Best 2×2 block density, falling back to the peak cell. Candidate only.
#[must_use]
pub fn group_density(matrix: &[u64], width: usize) -> DensityScore {
    let peak = heuristic_density(matrix, width);
    if width < 2 {
        return DensityScore {
            kind: "group",
            ..peak
        };
    }
    let mut best = peak.milli;
    let mut at = (peak.rows, peak.cols);
    for row in 0..width.saturating_sub(1) {
        for col in 0..width.saturating_sub(1) {
            let milli = submatrix_density(matrix, width, &[row, row + 1], &[col, col + 1]);
            if milli > best {
                best = milli;
                at = (row, col);
            }
        }
    }
    DensityScore {
        milli: best,
        kind: "group",
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
    let area = u64::try_from(area).unwrap_or(u64::MAX);
    let scaled = area.saturating_mul(1_000_000);
    let root = isqrt(scaled).max(1);
    sum.saturating_mul(1_000_000) / root
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
    use super::{anograph_counterexample, group_density, heuristic_density, submatrix_density};

    #[test]
    fn a_dense_block_scores_above_a_diagonal_of_the_same_mass() {
        let diagonal = [1, 0, 0, 0, 1, 0, 0, 0, 1];
        let block = [1, 1, 0, 1, 1, 0, 0, 0, 0];
        assert!(
            group_density(&block, 3).milli > group_density(&diagonal, 3).milli,
            "a 2x2 block is denser than a diagonal of ones"
        );
    }

    #[test]
    fn submatrix_density_uses_a_finer_square_root() {
        let matrix = alloc::vec![1_u64; 63];
        let cols = (0..63).collect::<alloc::vec::Vec<_>>();
        let milli = submatrix_density(&matrix, 63, &[0], &cols);
        assert!(
            (7_900..=8_000).contains(&milli),
            "63/sqrt(63) in thousandths is about 7937, got {milli}"
        );
    }

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
