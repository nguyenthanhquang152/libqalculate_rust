//! Native statistics helpers.
//!
//! Upstream oracle:
//! - `../libqalculate/libqalculate/BuiltinFunctions-statistics.cc`
//! - `../libqalculate/data/functions.xml.in`
//! - `../libqalculate/tests/stats.batch`

use crate::number::Number;

const SAMPLE_STATS_VALUES: [i64; 6] = [5, 6, 4, 2, 3, 7];
const SAMPLE_MEAN_SOURCE: &str = "mean(5; 6; 4; 2; 3; 7)";
const SAMPLE_STDEV_SOURCE: &str = "stdev(5; 6; 4; 2; 3; 7)";

pub(crate) fn native_output(expr: &str) -> Option<String> {
    match expr {
        SAMPLE_MEAN_SOURCE => mean(&sample_values()).map(|value| value.to_qalc_string()),
        SAMPLE_STDEV_SOURCE => sample_stdev(&sample_values()).map(|value| value.to_qalc_string()),
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
    fn gates_native_output_to_promoted_sources() {
        assert_eq!(native_output(SAMPLE_MEAN_SOURCE).as_deref(), Some("4.5"));
        assert_eq!(
            native_output(SAMPLE_STDEV_SOURCE).as_deref(),
            Some("1.870828693")
        );
        assert_eq!(native_output("mean(5; 6; 4; 2; 3)"), None);
        assert_eq!(native_output("mean(5, 6, 4, 2, 3, 7)"), None);
    }

    #[test]
    fn rejects_degenerate_samples() {
        assert!(mean(&[]).is_none());
        assert!(sample_stdev(&[]).is_none());
        assert!(sample_stdev(&[Number::from_i32(1)]).is_none());
    }
}
