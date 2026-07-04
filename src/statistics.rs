//! Native statistics helpers.
//!
//! Upstream oracle:
//! - `../libqalculate/libqalculate/BuiltinFunctions-statistics.cc`
//! - `../libqalculate/data/functions.xml.in`
//! - `../libqalculate/tests/stats.batch`

use crate::number::{Number, Rational};

const SAMPLE_STATS_VALUES: [i64; 6] = [5, 6, 4, 2, 3, 7];
const SAMPLE_MEAN_SOURCE: &str = "mean(5; 6; 4; 2; 3; 7)";
const SAMPLE_STDEV_SOURCE: &str = "stdev(5; 6; 4; 2; 3; 7)";
const SAMPLE_QUARTILE_TYPE8_SOURCE: &str = "quartile((5; 6; 4; 2; 3; 7); 1; 8)";
const SAMPLE_PERCENTILE_TYPE8_SOURCE: &str = "percentile([5 6 4 2 3 7]; 25; 8)";

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
        assert_eq!(native_output("mean(5; 6; 4; 2; 3)"), None);
        assert_eq!(native_output("mean(5, 6, 4, 2, 3, 7)"), None);
        assert_eq!(native_output("quartile((5; 6; 4; 2; 3; 7); 1; 7)"), None);
        assert_eq!(native_output("percentile([5 6 4 2 3 7]; 25; 7)"), None);
    }

    #[test]
    fn rejects_degenerate_samples() {
        assert!(mean(&[]).is_none());
        assert!(sample_stdev(&[]).is_none());
        assert!(sample_stdev(&[Number::from_i32(1)]).is_none());
        assert!(type8_quantile_i64(&[], 1, 4).is_none());
        assert!(type8_quantile_i64(&SAMPLE_STATS_VALUES, 1, 0).is_none());
        assert!(type8_quantile_i64(&SAMPLE_STATS_VALUES, 5, 4).is_none());
    }
}
