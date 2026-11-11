//! Solves the banded linear system of one Newton step.

/// A pivot this small means the system is singular.
const PIVOT_FLOOR: f64 = 1e-18;

/// Solves `matrix · x = rhs` by LU with partial pivoting restricted to the matrix's band: an
/// open curve's constraints only couple neighbouring segments, so the work is linear in their
/// number; the wrap-around rows of a closed curve widen the band to the whole matrix.
pub(super) fn solve_linear(mut matrix: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Option<Vec<f64>> {
    let n = rhs.len();
    let (mut lower, mut upper) = (0, 0);
    for (row, values) in matrix.iter().enumerate() {
        for (col, value) in values.iter().enumerate() {
            if *value != 0.0 {
                lower = lower.max(row.saturating_sub(col));
                upper = upper.max(col.saturating_sub(row));
            }
        }
    }
    // Row swaps within the lower band widen the upper band by as much.
    let reach = upper + lower;
    for i in 0..n {
        let last_row = (i + lower + 1).min(n);
        let pivot =
            (i..last_row).max_by(|&a, &b| matrix[a][i].abs().total_cmp(&matrix[b][i].abs()))?;
        if !matrix[pivot][i].is_finite() || matrix[pivot][i].abs() < PIVOT_FLOOR {
            return None;
        }
        matrix.swap(i, pivot);
        rhs.swap(i, pivot);
        let last_col = (i + reach + 1).min(n);
        for row in i + 1..last_row {
            let factor = matrix[row][i] / matrix[i][i];
            if factor == 0.0 {
                continue;
            }
            let (above, below) = matrix.split_at_mut(row);
            for (target, pivot_value) in
                below[0][i..last_col].iter_mut().zip(&above[i][i..last_col])
            {
                *target -= factor * pivot_value;
            }
            rhs[row] -= factor * rhs[i];
        }
    }
    let mut answer = vec![0.0; n];
    for i in (0..n).rev() {
        let mut sum = rhs[i];
        for j in i + 1..(i + reach + 1).min(n) {
            sum -= matrix[i][j] * answer[j];
        }
        answer[i] = sum / matrix[i][i];
        if !answer[i].is_finite() {
            return None;
        }
    }
    Some(answer)
}
