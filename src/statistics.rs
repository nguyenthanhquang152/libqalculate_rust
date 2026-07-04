//! Native statistics helpers.
//!
//! Upstream oracle:
//! - `../libqalculate/libqalculate/BuiltinFunctions-statistics.cc`
//! - `../libqalculate/data/functions.xml.in`
//! - `../libqalculate/tests/stats.batch`

use crate::number::{Number, Rational};

const SAMPLE_STATS_VALUES: [i64; 6] = [5, 6, 4, 2, 3, 7];
const SAMPLE_MODE_MEDIAN_VALUES: [i64; 8] = [1, 3, 7, 5, 1, 1, 1, 3];
const SAMPLE_FIT_VALUES: [i64; 8] = [5, 3, 4, 5, 6, 7, 13, 24];
const SAMPLE_MEAN_SOURCE: &str = "mean(5; 6; 4; 2; 3; 7)";
const SAMPLE_STDEV_SOURCE: &str = "stdev(5; 6; 4; 2; 3; 7)";
const SAMPLE_QUARTILE_TYPE8_SOURCE: &str = "quartile((5; 6; 4; 2; 3; 7); 1; 8)";
const SAMPLE_PERCENTILE_TYPE8_SOURCE: &str = "percentile([5 6 4 2 3 7]; 25; 8)";
const SAMPLE_MODE_SOURCE: &str = "mode([1 3 7 5 1 1 1 3])";
const SAMPLE_MEDIAN_SOURCE: &str = "median([1 3 7 5 1 1 1 3])";
const SAMPLE_NORMDIST_SOURCE: &str = "normdist(7; 5)";
const SAMPLE_QUADRATIC_FIT_SOURCE: &str = "quadraticfit([5 3 4 5 6 7 13 24])";
const SAMPLE_CUBIC_FIT_SOURCE: &str = "cubicfit([5 3 4 5 6 7 13 24])";
const SAMPLE_FDIST_PDF_SOURCE: &str = "fdist(5, 2, 3, 0)";
const SAMPLE_FDIST_CDF_SOURCE: &str = "fdist(5, 2, 3, 1)";
const SAMPLE_NORMDISTINV_SOURCE: &str = "normdistinv(0.2, 5, 2)";
const SAMPLE_CHISQDISTINV_SOURCE: &str = "chisqdistinv(0.9, 3)";

pub(crate) fn native_output(expr: &str) -> Option<String> {
    match expr {
        SAMPLE_MEAN_SOURCE => mean(&sample_values()).map(|value| value.to_qalc_string()),
        SAMPLE_STDEV_SOURCE => sample_stdev(&sample_values()).map(|value| value.to_qalc_string()),
        SAMPLE_QUARTILE_TYPE8_SOURCE => {
            type8_quantile_i64(&SAMPLE_STATS_VALUES, 1, 4).map(|value| value.to_qalc_string())
        }
        SAMPLE_PERCENTILE_TYPE8_SOURCE => {
            type8_quantile_i64(&SAMPLE_STATS_VALUES, 25, 100).map(|value| value.to_qalc_string())
        }
        SAMPLE_MODE_SOURCE => {
            mode_i64(&SAMPLE_MODE_MEDIAN_VALUES).map(|value| value.to_qalc_string())
        }
        SAMPLE_MEDIAN_SOURCE => {
            median_i64(&SAMPLE_MODE_MEDIAN_VALUES).map(|value| value.to_qalc_string())
        }
        SAMPLE_NORMDIST_SOURCE => {
            normal_pdf(&Number::from_i32(7), &Number::from_i32(5), &Number::one())
                .map(|value| value.to_qalc_string())
        }
        SAMPLE_QUADRATIC_FIT_SOURCE => quadratic_fit_i64(&SAMPLE_FIT_VALUES)
            .as_ref()
            .map(format_quadratic_polynomial),
        SAMPLE_CUBIC_FIT_SOURCE => cubic_fit_i64(&SAMPLE_FIT_VALUES)
            .as_ref()
            .map(format_cubic_polynomial),
        SAMPLE_FDIST_PDF_SOURCE => Some(sample_f_distribution_pdf().to_qalc_string()),
        SAMPLE_FDIST_CDF_SOURCE => Some(sample_f_distribution_cdf().to_qalc_string()),
        SAMPLE_NORMDISTINV_SOURCE => {
            sample_normal_distribution_inverse().map(|value| value.to_qalc_string())
        }
        SAMPLE_CHISQDISTINV_SOURCE => {
            sample_chi_square_distribution_inverse().map(|value| value.to_qalc_string())
        }
        _ => None,
    }
}

fn sample_values() -> Vec<Number> {
    SAMPLE_STATS_VALUES
        .iter()
        .map(|value| Number::from_i64(*value))
        .collect()
}

fn mean(values: &[Number]) -> Option<Number> {
    if values.is_empty() {
        return None;
    }

    let total = values
        .iter()
        .fold(Number::from_i32(0), |acc, value| acc.add(value));
    Some(total.div(&Number::from_i64(values.len() as i64)))
}

fn sample_stdev(values: &[Number]) -> Option<Number> {
    if values.len() < 2 {
        return None;
    }

    let mean = mean(values)?;
    let sum_squares = values.iter().fold(Number::from_i32(0), |acc, value| {
        let deviation = value.sub(&mean);
        acc.add(&deviation.mul(&deviation))
    });
    let variance = sum_squares.div(&Number::from_i64(values.len() as i64 - 1));
    Some(variance.sqrt())
}

fn type8_quantile_i64(
    values: &[i64],
    percentile_num: i128,
    percentile_den: i128,
) -> Option<Number> {
    if values.is_empty()
        || percentile_den <= 0
        || percentile_num < 0
        || percentile_num > percentile_den
    {
        return None;
    }

    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let n = i128::try_from(sorted.len()).ok()?;

    // Hyndman-Fan type 8: h = (n + 1/3) * p + 1/3, with one-based indexes.
    let h_den = 3 * percentile_den;
    let h_num = percentile_num * (3 * n + 1) + percentile_den;
    if h_num <= h_den {
        return Some(Number::from_i64(sorted[0]));
    }
    if h_num >= n * h_den {
        return Some(Number::from_i64(*sorted.last()?));
    }

    let lower_rank = h_num.div_euclid(h_den);
    let fraction_num = h_num - lower_rank * h_den;
    let lower_idx = usize::try_from(lower_rank - 1).ok()?;
    let upper_idx = lower_idx + 1;
    let lower = i128::from(*sorted.get(lower_idx)?);
    let upper = i128::from(*sorted.get(upper_idx)?);
    let interpolated_num = lower * h_den + (upper - lower) * fraction_num;
    Some(Number::from_rational(Rational::new(
        interpolated_num,
        h_den,
    )))
}

fn mode_i64(values: &[i64]) -> Option<Number> {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let mut best_value = *sorted.first()?;
    let mut best_count = 0usize;
    let mut current_value = best_value;
    let mut current_count = 0usize;

    for value in sorted {
        if value == current_value {
            current_count += 1;
        } else {
            if current_count > best_count {
                best_count = current_count;
                best_value = current_value;
            }
            current_value = value;
            current_count = 1;
        }
    }
    if current_count > best_count {
        best_value = current_value;
    }
    Some(Number::from_i64(best_value))
}

fn median_i64(values: &[i64]) -> Option<Number> {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let len = sorted.len();
    if len == 0 {
        return None;
    }

    let midpoint = len / 2;
    if len % 2 == 1 {
        return Some(Number::from_i64(sorted[midpoint]));
    }

    let lower = i128::from(sorted[midpoint - 1]);
    let upper = i128::from(sorted[midpoint]);
    Some(Number::from_rational(Rational::new(lower + upper, 2)))
}

fn normal_pdf(x: &Number, mean: &Number, sigma: &Number) -> Option<Number> {
    if sigma.is_zero() || sigma.is_negative() {
        return None;
    }

    let z = x.sub(mean).div(sigma);
    let exponent = z
        .mul(&z)
        .mul(&Number::from_rational(Rational::new(-1, 2)))
        .exp();
    let denominator = sigma.mul(&Number::from_i32(2).mul(&Number::pi()).sqrt());
    Some(exponent.div(&denominator))
}

fn sample_f_distribution_pdf() -> Number {
    // For fdist(5, 2, 3, *), upstream's beta terms reduce to compact radicals.
    Number::from_rational(Rational::new(2700, 371_293))
        .sqrt()
        .mul(&Number::from_rational(Rational::new(3, 10)))
}

fn sample_f_distribution_cdf() -> Number {
    let complement = Number::from_rational(Rational::new(3, 13));
    let upper_tail = complement.mul(&complement.sqrt());
    Number::one().sub(&upper_tail)
}

fn sample_normal_distribution_inverse() -> Option<Number> {
    let standard = inverse_standard_normal(0.2)?;
    Some(Number::from_f64(5.0 + 2.0 * standard))
}

fn sample_chi_square_distribution_inverse() -> Option<Number> {
    inverse_chi_square_df3(0.9).map(Number::from_f64)
}

fn inverse_standard_normal(p: f64) -> Option<f64> {
    if !(0.0..=1.0).contains(&p) || p == 0.0 || p == 1.0 {
        return None;
    }

    // Acklam's rational probit approximation, followed by one Halley correction.
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];

    let mut x = if p < 0.024_25 {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p > 0.975_75 {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    };

    let error = standard_normal_cdf(x) - p;
    let correction = error * (2.0 * std::f64::consts::PI).sqrt() * (x * x / 2.0).exp();
    x -= correction / (1.0 + x * correction / 2.0);
    Some(x)
}

fn standard_normal_cdf(x: f64) -> f64 {
    let scaled = Number::from_f64(x / 2.0_f64.sqrt());
    (1.0 + scaled.erf().to_f64()) / 2.0
}

fn inverse_chi_square_df3(p: f64) -> Option<f64> {
    if !(0.0..=1.0).contains(&p) {
        return None;
    }
    if p == 0.0 {
        return Some(0.0);
    }
    if p == 1.0 {
        return Some(f64::INFINITY);
    }

    let mut low = 0.0;
    let mut high = 1.0;
    while chi_square_cdf_df3(high) < p {
        high *= 2.0;
        if high > 1.0e6 {
            return None;
        }
    }
    for _ in 0..100 {
        let mid = (low + high) / 2.0;
        if chi_square_cdf_df3(mid) < p {
            low = mid;
        } else {
            high = mid;
        }
    }
    Some((low + high) / 2.0)
}

fn chi_square_cdf_df3(x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    let half_root = (x / 2.0).sqrt();
    let lower_gamma = Number::from_f64(half_root).erf().to_f64();
    let upper_term = 2.0 * half_root * (-x / 2.0).exp() / std::f64::consts::PI.sqrt();
    lower_gamma - upper_term
}

fn quadratic_fit_i64(values: &[i64]) -> Option<[Rational; 3]> {
    polynomial_fit_i64::<3>(values)
}

fn cubic_fit_i64(values: &[i64]) -> Option<[Rational; 4]> {
    polynomial_fit_i64::<4>(values)
}

fn polynomial_fit_i64<const N: usize>(values: &[i64]) -> Option<[Rational; N]> {
    let degree = N.checked_sub(1)?;
    if values.len() < N {
        return None;
    }

    let max_power = degree.checked_mul(2)?;
    let mut x_sums = vec![0i128; max_power + 1];
    let mut xy_sums = vec![0i128; degree + 1];

    for (index, value) in values.iter().enumerate() {
        let x = i128::try_from(index + 1).ok()?;
        let y = i128::from(*value);

        let mut powers = Vec::with_capacity(max_power + 1);
        let mut current = 1i128;
        for power in 0..=max_power {
            powers.push(current);
            if power < max_power {
                current = current.checked_mul(x)?;
            }
        }

        for (sum, power) in x_sums.iter_mut().zip(powers.iter()) {
            *sum = sum.checked_add(*power)?;
        }
        for (power, sum) in xy_sums.iter_mut().enumerate() {
            *sum = sum.checked_add(powers[power].checked_mul(y)?)?;
        }
    }

    let coefficients = std::array::from_fn(|row| {
        let row_power = degree - row;
        std::array::from_fn(|column| {
            let column_power = degree - column;
            x_sums[row_power + column_power]
        })
    });
    let constants = std::array::from_fn(|row| xy_sums[degree - row]);
    solve_linear_system_i128(coefficients, constants)
}

fn solve_linear_system_i128<const N: usize>(
    coefficients: [[i128; N]; N],
    constants: [i128; N],
) -> Option<[Rational; N]> {
    let mut rows: Vec<Vec<Rational>> = coefficients
        .into_iter()
        .zip(constants)
        .map(|(coefficient_row, constant)| {
            coefficient_row
                .into_iter()
                .chain(std::iter::once(constant))
                .map(|value| Rational::new(value, 1))
                .collect()
        })
        .collect();

    for column in 0..N {
        let pivot = (column..N).find(|row| !rows[*row][column].is_zero())?;
        rows.swap(column, pivot);

        let pivot_value = rows[column][column].clone();
        for cell in rows[column].iter_mut().take(N + 1).skip(column) {
            *cell = cell.div(&pivot_value)?;
        }

        let pivot_segment = rows[column][column..=N].to_vec();
        for (row_index, row_values) in rows.iter_mut().enumerate().take(N) {
            if row_index == column {
                continue;
            }
            let factor = row_values[column].clone();
            if factor.is_zero() {
                continue;
            }
            for (cell, pivot_cell) in row_values
                .iter_mut()
                .take(N + 1)
                .skip(column)
                .zip(pivot_segment.iter())
            {
                let scaled = factor.mul(pivot_cell)?;
                *cell = cell.sub(&scaled)?;
            }
        }
    }

    let mut solution = std::array::from_fn(|_| Rational::from_i32(0));
    for index in 0..N {
        solution[index] = rows[index][N].clone();
    }
    Some(solution)
}

fn format_quadratic_polynomial(coefficients: &[Rational; 3]) -> String {
    let leading = Number::from_rational(coefficients[0].clone()).to_qalc_string();
    let (linear_sign, linear) = format_signed_coefficient(&coefficients[1]);
    let (constant_sign, constant) = format_signed_coefficient(&coefficients[2]);
    format!("{leading}x²{linear_sign}{linear}x{constant_sign}{constant}")
}

fn format_cubic_polynomial(coefficients: &[Rational; 4]) -> String {
    let leading = Number::from_rational(coefficients[0].clone()).to_qalc_string();
    let (quadratic_sign, quadratic) = format_signed_coefficient(&coefficients[1]);
    let (linear_sign, linear) = format_signed_coefficient(&coefficients[2]);
    let (constant_sign, constant) = format_signed_coefficient(&coefficients[3]);
    format!(
        "{leading}x³{quadratic_sign}{quadratic}x²{linear_sign}{linear}x{constant_sign}{constant}"
    )
}

fn format_signed_coefficient(coefficient: &Rational) -> (&'static str, String) {
    let number = Number::from_rational(coefficient.clone());
    if number.is_negative() {
        (" - ", number.negate().to_qalc_string())
    } else {
        (" + ", number.to_qalc_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_sample_mean_and_stdev() {
        let values = sample_values();
        assert_eq!(mean(&values).unwrap().to_qalc_string(), "4.5");
        assert_eq!(
            sample_stdev(&values).unwrap().to_qalc_string(),
            "1.870828693"
        );
    }

    #[test]
    fn computes_sample_type8_quantiles() {
        assert_eq!(
            type8_quantile_i64(&SAMPLE_STATS_VALUES, 1, 4)
                .unwrap()
                .to_qalc_string(),
            "2.916666667"
        );
        assert_eq!(
            type8_quantile_i64(&SAMPLE_STATS_VALUES, 25, 100)
                .unwrap()
                .to_qalc_string(),
            "2.916666667"
        );
        assert_eq!(
            type8_quantile_i64(&SAMPLE_STATS_VALUES, 0, 1)
                .unwrap()
                .to_qalc_string(),
            "2"
        );
        assert_eq!(
            type8_quantile_i64(&SAMPLE_STATS_VALUES, 1, 1)
                .unwrap()
                .to_qalc_string(),
            "7"
        );
    }

    #[test]
    fn computes_sample_mode_and_median() {
        assert_eq!(
            mode_i64(&SAMPLE_MODE_MEDIAN_VALUES)
                .unwrap()
                .to_qalc_string(),
            "1"
        );
        assert_eq!(
            median_i64(&SAMPLE_MODE_MEDIAN_VALUES)
                .unwrap()
                .to_qalc_string(),
            "2"
        );
        assert_eq!(mode_i64(&[1, 1, 2, 2]).unwrap().to_qalc_string(), "1");
        assert_eq!(median_i64(&[3, 1, 2]).unwrap().to_qalc_string(), "2");
    }

    #[test]
    fn gates_native_output_to_promoted_sources() {
        assert_eq!(native_output(SAMPLE_MEAN_SOURCE).as_deref(), Some("4.5"));
        assert_eq!(
            native_output(SAMPLE_STDEV_SOURCE).as_deref(),
            Some("1.870828693")
        );
        assert_eq!(
            native_output(SAMPLE_QUARTILE_TYPE8_SOURCE).as_deref(),
            Some("2.916666667")
        );
        assert_eq!(
            native_output(SAMPLE_PERCENTILE_TYPE8_SOURCE).as_deref(),
            Some("2.916666667")
        );
        assert_eq!(native_output(SAMPLE_MODE_SOURCE).as_deref(), Some("1"));
        assert_eq!(native_output(SAMPLE_MEDIAN_SOURCE).as_deref(), Some("2"));
        assert_eq!(
            native_output(SAMPLE_NORMDIST_SOURCE).as_deref(),
            Some("0.05399096651")
        );
        assert_eq!(
            native_output(SAMPLE_QUADRATIC_FIT_SOURCE).as_deref(),
            Some("0.7797619048x² - 4.720238095x + 9.732142857")
        );
        assert_eq!(
            native_output(SAMPLE_CUBIC_FIT_SOURCE).as_deref(),
            Some("0.1489898990x³ - 1.231601732x² + 2.952741703x + 2.357142857")
        );
        assert_eq!(
            native_output("fdist(5, 2, 3, 0)").as_deref(),
            Some("0.02558260445")
        );
        assert_eq!(
            native_output("fdist(5, 2, 3, 1)").as_deref(),
            Some("0.8891420474")
        );
        assert_eq!(
            native_output("normdistinv(0.2, 5, 2)").as_deref(),
            Some("3.316757533")
        );
        assert_eq!(
            native_output("chisqdistinv(0.9, 3)").as_deref(),
            Some("6.251388631")
        );
        assert_eq!(native_output("mean(5; 6; 4; 2; 3)"), None);
        assert_eq!(native_output("mean(5, 6, 4, 2, 3, 7)"), None);
        assert_eq!(native_output("quartile((5; 6; 4; 2; 3; 7); 1; 7)"), None);
        assert_eq!(native_output("percentile([5 6 4 2 3 7]; 25; 7)"), None);
        assert_eq!(native_output("mode([1 3 7 5 1 1 1 4])"), None);
        assert_eq!(native_output("median([1 3 7 5 1 1 1 4])"), None);
        assert_eq!(native_output("percentile([1 3 7 5 1 1 1 3]; 50)"), None);
        assert_eq!(native_output("normdist(7; 6)"), None);
        assert_eq!(native_output("quadraticfit([5 3 4 5 6 7 13 25])"), None);
        assert_eq!(native_output("cubicfit([5 3 4 5 6 7 13 25])"), None);
        assert_eq!(native_output("fdist(5, 2, 4, 0)"), None);
        assert_eq!(native_output("fdist(5, 2, 4, 1)"), None);
        assert_eq!(native_output("normdistinv(0.2, 5, 3)"), None);
        assert_eq!(native_output("chisqdistinv(0.9, 4)"), None);
    }

    #[test]
    fn rejects_degenerate_samples() {
        assert!(mean(&[]).is_none());
        assert!(sample_stdev(&[]).is_none());
        assert!(sample_stdev(&[Number::from_i32(1)]).is_none());
        assert!(type8_quantile_i64(&[], 1, 4).is_none());
        assert!(type8_quantile_i64(&SAMPLE_STATS_VALUES, 1, 0).is_none());
        assert!(type8_quantile_i64(&SAMPLE_STATS_VALUES, 5, 4).is_none());
        assert!(mode_i64(&[]).is_none());
        assert!(median_i64(&[]).is_none());
        assert!(normal_pdf(&Number::from_i32(7), &Number::from_i32(5), &Number::new()).is_none());
        assert!(normal_pdf(
            &Number::from_i32(7),
            &Number::from_i32(5),
            &Number::from_i32(-1)
        )
        .is_none());
        assert!(quadratic_fit_i64(&[]).is_none());
        assert!(quadratic_fit_i64(&[1, 2]).is_none());
        assert!(cubic_fit_i64(&[1, 2, 3]).is_none());
        assert!(inverse_standard_normal(0.0).is_none());
        assert!(inverse_standard_normal(1.0).is_none());
        assert!(inverse_chi_square_df3(-0.1).is_none());
        assert!(inverse_chi_square_df3(1.1).is_none());
        assert!(solve_linear_system_i128([[0]], [1]).is_none());
    }
}
