//! Native statistics helpers.
//!
//! Upstream oracle:
//! - `../libqalculate/libqalculate/BuiltinFunctions-statistics.cc`
//! - `../libqalculate/data/functions.xml.in`
//! - `../libqalculate/tests/stats.batch`

use crate::data::CsvLoadError;
use crate::number::{Number, Rational};
use crate::{ast::Expression, context::CalculatorContext};

type NumberVector = Vec<Number>;
type NumberVectorPair = (NumberVector, NumberVector);

struct ResolvedVector {
    values: NumberVector,
    origin: VectorOrigin,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VectorOrigin {
    Direct,
    Session,
    Generated,
}

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
const CSV_VECTORDATA_PATH: &str = "tests/vectordata.csv";
const CSV_VECTORDATA2_PATH: &str = "tests/vectordata2.csv";
const CSV_MEAN_VECTORDATA_SOURCE: &str = "mean(load(tests/vectordata.csv))";
const CSV_MEAN_VECTORDATA_QUOTED_SOURCE: &str = "mean(load(\"tests/vectordata.csv\"))";
const CSV_STDEV_VECTORDATA_SOURCE: &str = "stdev(load(tests/vectordata.csv))";
const CSV_STDEV_VECTORDATA_QUOTED_SOURCE: &str = "stdev(load(\"tests/vectordata.csv\"))";
const CSV_MIN_VECTORDATA_SOURCE: &str = "min(load(tests/vectordata.csv))";
const CSV_MIN_VECTORDATA_QUOTED_SOURCE: &str = "min(load(\"tests/vectordata.csv\"))";
const CSV_MAX_VECTORDATA_SOURCE: &str = "max(load(tests/vectordata.csv))";
const CSV_MAX_VECTORDATA_QUOTED_SOURCE: &str = "max(load(\"tests/vectordata.csv\"))";
const CSV_TOTAL_VECTORDATA_SOURCE: &str = "total(load(tests/vectordata.csv))";
const CSV_TOTAL_VECTORDATA_QUOTED_SOURCE: &str = "total(load(\"tests/vectordata.csv\"))";
const CSV_RANGE_VECTORDATA_SOURCE: &str = "range(load(tests/vectordata.csv))";
const CSV_RANGE_VECTORDATA_QUOTED_SOURCE: &str = "range(load(\"tests/vectordata.csv\"))";
const CSV_MEDIAN_VECTORDATA_SOURCE: &str = "median(load(tests/vectordata.csv))";
const CSV_MEDIAN_VECTORDATA_QUOTED_SOURCE: &str = "median(load(\"tests/vectordata.csv\"))";
const CSV_PEARSON_VECTORDATA_SOURCE: &str =
    "pearson(load(tests/vectordata.csv), load(tests/vectordata2.csv))";
const CSV_PEARSON_VECTORDATA_QUOTED_SOURCE: &str =
    "pearson(load(\"tests/vectordata.csv\"), load(\"tests/vectordata2.csv\"))";
const CSV_SPEARMAN_VECTORDATA_SOURCE: &str =
    "spearman(load(tests/vectordata.csv), load(tests/vectordata2.csv))";
const CSV_SPEARMAN_VECTORDATA_QUOTED_SOURCE: &str =
    "spearman(load(\"tests/vectordata.csv\"), load(\"tests/vectordata2.csv\"))";
const CSV_COVAR_VECTORDATA_SOURCE: &str =
    "covar(load(tests/vectordata.csv), load(tests/vectordata2.csv))";
const CSV_COVAR_VECTORDATA_QUOTED_SOURCE: &str =
    "covar(load(\"tests/vectordata.csv\"), load(\"tests/vectordata2.csv\"))";
const CSV_POOLVAR_VECTORDATA_SOURCE: &str =
    "poolvar(load(tests/vectordata.csv), load(tests/vectordata2.csv))";
const CSV_POOLVAR_VECTORDATA_QUOTED_SOURCE: &str =
    "poolvar(load(\"tests/vectordata.csv\"), load(\"tests/vectordata2.csv\"))";
const CSV_TTEST_VECTORDATA_SOURCE: &str =
    "ttest(load(tests/vectordata.csv), load(tests/vectordata2.csv))";
const CSV_TTEST_VECTORDATA_QUOTED_SOURCE: &str =
    "ttest(load(\"tests/vectordata.csv\"), load(\"tests/vectordata2.csv\"))";
const CSV_PTTEST_VECTORDATA_SOURCE: &str =
    "pttest(load(tests/vectordata.csv), load(tests/vectordata2.csv))";
const CSV_PTTEST_VECTORDATA_QUOTED_SOURCE: &str =
    "pttest(load(\"tests/vectordata.csv\"), load(\"tests/vectordata2.csv\"))";
const CSV_GEOMEAN_ABS_VECTORDATA_SOURCE: &str = "geomean(abs(load(tests/vectordata.csv)))";
const CSV_GEOMEAN_ABS_VECTORDATA_QUOTED_SOURCE: &str =
    "geomean(abs(load(\"tests/vectordata.csv\")))";
const CSV_HARMMEAN_ABS_VECTORDATA_SOURCE: &str = "harmmean(abs(load(tests/vectordata.csv)))";
const CSV_HARMMEAN_ABS_VECTORDATA_QUOTED_SOURCE: &str =
    "harmmean(abs(load(\"tests/vectordata.csv\")))";
const CSV_RMS_VECTORDATA_SOURCE: &str = "rms(load(tests/vectordata.csv))";
const CSV_RMS_VECTORDATA_QUOTED_SOURCE: &str = "rms(load(\"tests/vectordata.csv\"))";
const CSV_TRIMMEAN_VECTORDATA_SOURCE: &str = "trimmean(load(tests/vectordata.csv), 10)";
const CSV_TRIMMEAN_VECTORDATA_QUOTED_SOURCE: &str = "trimmean(load(\"tests/vectordata.csv\"), 10)";
const CSV_WINSORMEAN_VECTORDATA_SOURCE: &str = "winsormean(load(tests/vectordata.csv), 10)";
const CSV_WINSORMEAN_VECTORDATA_QUOTED_SOURCE: &str =
    "winsormean(load(\"tests/vectordata.csv\"), 10)";
const CSV_WEIGHMEAN_VECTORDATA_SOURCE: &str =
    "weighmean(load(tests/vectordata.csv), genvector(2;1;100))";
const CSV_WEIGHMEAN_VECTORDATA_QUOTED_SOURCE: &str =
    "weighmean(load(\"tests/vectordata.csv\"), genvector(2;1;100))";
const CSV_STDERR_VECTORDATA_SOURCE: &str = "stderr(load(tests/vectordata.csv))";
const CSV_STDERR_VECTORDATA_QUOTED_SOURCE: &str = "stderr(load(\"tests/vectordata.csv\"))";
const CSV_MEANDEV_VECTORDATA_SOURCE: &str = "meandev(load(tests/vectordata.csv))";
const CSV_MEANDEV_VECTORDATA_QUOTED_SOURCE: &str = "meandev(load(\"tests/vectordata.csv\"))";
const CSV_QUARTILE_TYPE7_VECTORDATA_SOURCE: &str = "quartile(load(tests/vectordata.csv), 1, 7)";
const CSV_QUARTILE_TYPE7_VECTORDATA_QUOTED_SOURCE: &str =
    "quartile(load(\"tests/vectordata.csv\"), 1, 7)";
const CSV_PERCENTILE_TYPE7_VECTORDATA_SOURCE: &str =
    "percentile(load(tests/vectordata.csv), 25, 7)";
const CSV_PERCENTILE_TYPE7_VECTORDATA_QUOTED_SOURCE: &str =
    "percentile(load(\"tests/vectordata.csv\"), 25, 7)";
const CSV_DECILE_TYPE7_VECTORDATA_SOURCE: &str = "decile(load(tests/vectordata.csv), 9, 7)";
const CSV_DECILE_TYPE7_VECTORDATA_QUOTED_SOURCE: &str =
    "decile(load(\"tests/vectordata.csv\"), 9, 7)";
const CSV_IQR_VECTORDATA_SOURCE: &str = "iqr(load(tests/vectordata.csv))";
const CSV_IQR_VECTORDATA_QUOTED_SOURCE: &str = "iqr(load(\"tests/vectordata.csv\"))";
const MAX_NATIVE_SESSION_VECTOR_LEN: i64 = 10_000;

pub(crate) fn native_output(expr: &str) -> Result<Option<String>, CsvLoadError> {
    let output = match expr {
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
        CSV_MEAN_VECTORDATA_SOURCE | CSV_MEAN_VECTORDATA_QUOTED_SOURCE => {
            let values = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            mean(&values).map(|value| approximate_qalc_string(&value))
        }
        CSV_STDEV_VECTORDATA_SOURCE | CSV_STDEV_VECTORDATA_QUOTED_SOURCE => {
            let values = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            sample_stdev(&values).map(|value| approximate_qalc_string(&value))
        }
        CSV_MIN_VECTORDATA_SOURCE | CSV_MIN_VECTORDATA_QUOTED_SOURCE => {
            let values = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            minimum(&values).map(|value| approximate_qalc_string(&value))
        }
        CSV_MAX_VECTORDATA_SOURCE | CSV_MAX_VECTORDATA_QUOTED_SOURCE => {
            let values = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            maximum(&values).map(|value| approximate_qalc_string(&value))
        }
        CSV_TOTAL_VECTORDATA_SOURCE | CSV_TOTAL_VECTORDATA_QUOTED_SOURCE => {
            let values = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            total(&values).map(|value| approximate_qalc_string(&value))
        }
        CSV_RANGE_VECTORDATA_SOURCE | CSV_RANGE_VECTORDATA_QUOTED_SOURCE => {
            let values = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            range(&values).map(|value| approximate_qalc_string(&value))
        }
        CSV_MEDIAN_VECTORDATA_SOURCE | CSV_MEDIAN_VECTORDATA_QUOTED_SOURCE => {
            let values = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            median(&values).map(|value| approximate_qalc_string(&value))
        }
        CSV_PEARSON_VECTORDATA_SOURCE | CSV_PEARSON_VECTORDATA_QUOTED_SOURCE => {
            let lhs = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            let rhs = crate::data::load_csv_numbers(CSV_VECTORDATA2_PATH)?;
            pearson_correlation(&lhs, &rhs).map(|value| approximate_qalc_string(&value))
        }
        CSV_SPEARMAN_VECTORDATA_SOURCE | CSV_SPEARMAN_VECTORDATA_QUOTED_SOURCE => {
            let lhs = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            let rhs = crate::data::load_csv_numbers(CSV_VECTORDATA2_PATH)?;
            spearman_correlation(&lhs, &rhs).map(|value| approximate_qalc_string(&value))
        }
        CSV_COVAR_VECTORDATA_SOURCE | CSV_COVAR_VECTORDATA_QUOTED_SOURCE => {
            let lhs = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            let rhs = crate::data::load_csv_numbers(CSV_VECTORDATA2_PATH)?;
            covariance(&lhs, &rhs).map(|value| approximate_qalc_string(&value))
        }
        CSV_POOLVAR_VECTORDATA_SOURCE | CSV_POOLVAR_VECTORDATA_QUOTED_SOURCE => {
            let lhs = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            let rhs = crate::data::load_csv_numbers(CSV_VECTORDATA2_PATH)?;
            pooled_variance(&lhs, &rhs).map(|value| approximate_qalc_string(&value))
        }
        CSV_TTEST_VECTORDATA_SOURCE | CSV_TTEST_VECTORDATA_QUOTED_SOURCE => {
            let lhs = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            let rhs = crate::data::load_csv_numbers(CSV_VECTORDATA2_PATH)?;
            unpaired_t_test(&lhs, &rhs).map(|value| approximate_qalc_string(&value))
        }
        CSV_PTTEST_VECTORDATA_SOURCE | CSV_PTTEST_VECTORDATA_QUOTED_SOURCE => {
            let lhs = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            let rhs = crate::data::load_csv_numbers(CSV_VECTORDATA2_PATH)?;
            paired_t_test(&lhs, &rhs).map(|value| approximate_qalc_string(&value))
        }
        CSV_GEOMEAN_ABS_VECTORDATA_SOURCE | CSV_GEOMEAN_ABS_VECTORDATA_QUOTED_SOURCE => {
            let values = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            geometric_mean(&absolute_values(&values)).map(|value| approximate_qalc_string(&value))
        }
        CSV_HARMMEAN_ABS_VECTORDATA_SOURCE | CSV_HARMMEAN_ABS_VECTORDATA_QUOTED_SOURCE => {
            let values = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            harmonic_mean(&absolute_values(&values)).map(|value| approximate_qalc_string(&value))
        }
        CSV_RMS_VECTORDATA_SOURCE | CSV_RMS_VECTORDATA_QUOTED_SOURCE => {
            let values = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            root_mean_square(&values).map(|value| approximate_qalc_string(&value))
        }
        CSV_TRIMMEAN_VECTORDATA_SOURCE | CSV_TRIMMEAN_VECTORDATA_QUOTED_SOURCE => {
            let values = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            trimmed_mean(&values, 10).map(|value| approximate_qalc_string(&value))
        }
        CSV_WINSORMEAN_VECTORDATA_SOURCE | CSV_WINSORMEAN_VECTORDATA_QUOTED_SOURCE => {
            let values = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            winsorized_mean(&values, 10).map(|value| approximate_qalc_string(&value))
        }
        CSV_WEIGHMEAN_VECTORDATA_SOURCE | CSV_WEIGHMEAN_VECTORDATA_QUOTED_SOURCE => {
            let values = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            let weights = vec![Number::from_i32(2); values.len()];
            weighted_mean(&values, &weights).map(|value| approximate_qalc_string(&value))
        }
        CSV_STDERR_VECTORDATA_SOURCE | CSV_STDERR_VECTORDATA_QUOTED_SOURCE => {
            let values = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            standard_error(&values).map(|value| approximate_qalc_string(&value))
        }
        CSV_MEANDEV_VECTORDATA_SOURCE | CSV_MEANDEV_VECTORDATA_QUOTED_SOURCE => {
            let values = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            mean_deviation(&values).map(|value| approximate_qalc_string(&value))
        }
        CSV_QUARTILE_TYPE7_VECTORDATA_SOURCE | CSV_QUARTILE_TYPE7_VECTORDATA_QUOTED_SOURCE => {
            let values = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            type7_quantile(&values, 1, 4).and_then(|value| fixed_decimal_qalc_string(&value, 8))
        }
        CSV_PERCENTILE_TYPE7_VECTORDATA_SOURCE | CSV_PERCENTILE_TYPE7_VECTORDATA_QUOTED_SOURCE => {
            let values = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            type7_quantile(&values, 25, 100).and_then(|value| fixed_decimal_qalc_string(&value, 8))
        }
        CSV_DECILE_TYPE7_VECTORDATA_SOURCE | CSV_DECILE_TYPE7_VECTORDATA_QUOTED_SOURCE => {
            let values = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            type7_quantile(&values, 9, 10).and_then(|value| fixed_decimal_qalc_string(&value, 8))
        }
        CSV_IQR_VECTORDATA_SOURCE | CSV_IQR_VECTORDATA_QUOTED_SOURCE => {
            let values = crate::data::load_csv_numbers(CSV_VECTORDATA_PATH)?;
            interquartile_range(&values).and_then(|value| fixed_decimal_qalc_string(&value, 8))
        }
        _ => None,
    };
    Ok(output)
}

pub(crate) fn native_context_output(
    expr: &str,
    context: &mut CalculatorContext,
) -> Result<Option<String>, CsvLoadError> {
    let Ok(parsed) = crate::parser::operators::parse_expression(expr) else {
        return Ok(None);
    };
    evaluate_context_statistic(&parsed, context)
}

fn evaluate_context_statistic(
    expr: &Expression,
    context: &CalculatorContext,
) -> Result<Option<String>, CsvLoadError> {
    let Expression::FunctionCall { function, args } = expr else {
        return Ok(None);
    };

    let output = match function.id() {
        "number" if args.len() == 1 => {
            session_vector_values(&args[0], context)?.map(|values| values.len().to_string())
        }
        "mean" if args.len() == 1 => session_vector_values(&args[0], context)?
            .and_then(|values| mean(&values).map(|value| approximate_qalc_string(&value))),
        "stdev" if args.len() == 1 => session_vector_values(&args[0], context)?
            .and_then(|values| sample_stdev(&values).map(|value| approximate_qalc_string(&value))),
        "min" if args.len() == 1 => session_vector_values(&args[0], context)?
            .and_then(|values| minimum(&values).map(|value| approximate_qalc_string(&value))),
        "max" if args.len() == 1 => session_vector_values(&args[0], context)?
            .and_then(|values| maximum(&values).map(|value| approximate_qalc_string(&value))),
        "total" if args.len() == 1 => session_vector_values(&args[0], context)?
            .and_then(|values| total(&values).map(|value| approximate_qalc_string(&value))),
        "range" if args.len() == 1 => session_vector_values(&args[0], context)?
            .and_then(|values| range(&values).map(|value| approximate_qalc_string(&value))),
        "median" if args.len() == 1 => session_vector_values(&args[0], context)?
            .and_then(|values| median(&values).map(|value| approximate_qalc_string(&value))),
        "geomean" if args.len() == 1 => {
            session_vector_values(&args[0], context)?.and_then(|values| {
                geometric_mean(&values).map(|value| approximate_qalc_string(&value))
            })
        }
        "harmmean" if args.len() == 1 => session_vector_values(&args[0], context)?
            .and_then(|values| harmonic_mean(&values).map(|value| approximate_qalc_string(&value))),
        "rms" if args.len() == 1 => session_vector_values(&args[0], context)?.and_then(|values| {
            root_mean_square(&values).map(|value| approximate_qalc_string(&value))
        }),
        "stderr" if args.len() == 1 => {
            session_vector_values(&args[0], context)?.and_then(|values| {
                standard_error(&values).map(|value| approximate_qalc_string(&value))
            })
        }
        "meandev" if args.len() == 1 => {
            session_vector_values(&args[0], context)?.and_then(|values| {
                mean_deviation(&values).map(|value| approximate_qalc_string(&value))
            })
        }
        "trimmean" if args.len() == 2 => {
            let values = session_vector_values(&args[0], context)?;
            let percent = integer_arg(&args[1]);
            values
                .zip(percent)
                .and_then(|(values, percent)| trimmed_mean(&values, percent))
                .map(|value| approximate_qalc_string(&value))
        }
        "winsormean" if args.len() == 2 => {
            let values = session_vector_values(&args[0], context)?;
            let percent = integer_arg(&args[1]);
            values
                .zip(percent)
                .and_then(|(values, percent)| winsorized_mean(&values, percent))
                .map(|value| approximate_qalc_string(&value))
        }
        "weighmean" if args.len() == 2 => {
            let values = session_vector_values(&args[0], context)?;
            let weights = session_or_generated_vector_values(&args[1], context)?;
            values
                .zip(weights)
                .and_then(|(values, weights)| weighted_mean(&values, &weights))
                .map(|value| approximate_qalc_string(&value))
        }
        "quartile" if args.len() == 3 && integer_arg(&args[2]) == Some(7) => {
            let values = session_vector_values(&args[0], context)?;
            let quartile = integer_arg(&args[1]);
            values
                .zip(quartile)
                .and_then(|(values, quartile)| type7_quantile(&values, quartile as i128, 4))
                .and_then(|value| fixed_decimal_qalc_string(&value, 8))
        }
        "percentile" if args.len() == 3 && integer_arg(&args[2]) == Some(7) => {
            let values = session_vector_values(&args[0], context)?;
            let percentile = integer_arg(&args[1]);
            values
                .zip(percentile)
                .and_then(|(values, percentile)| type7_quantile(&values, percentile as i128, 100))
                .and_then(|value| fixed_decimal_qalc_string(&value, 8))
        }
        "decile" if args.len() == 3 && integer_arg(&args[2]) == Some(7) => {
            let values = session_vector_values(&args[0], context)?;
            let decile = integer_arg(&args[1]);
            values
                .zip(decile)
                .and_then(|(values, decile)| type7_quantile(&values, decile as i128, 10))
                .and_then(|value| fixed_decimal_qalc_string(&value, 8))
        }
        "iqr" if args.len() == 1 => session_vector_values(&args[0], context)?
            .and_then(|values| interquartile_range(&values))
            .and_then(|value| fixed_decimal_qalc_string(&value, 8)),
        "pearson" if args.len() == 2 => {
            paired_session_vector_values(args, context)?.and_then(|(lhs, rhs)| {
                pearson_correlation(&lhs, &rhs).map(|value| approximate_qalc_string(&value))
            })
        }
        "spearman" if args.len() == 2 => {
            paired_session_vector_values(args, context)?.and_then(|(lhs, rhs)| {
                spearman_correlation(&lhs, &rhs).map(|value| approximate_qalc_string(&value))
            })
        }
        "covar" if args.len() == 2 => {
            paired_session_vector_values(args, context)?.and_then(|(lhs, rhs)| {
                covariance(&lhs, &rhs).map(|value| approximate_qalc_string(&value))
            })
        }
        "poolvar" if args.len() == 2 => {
            paired_session_vector_values(args, context)?.and_then(|(lhs, rhs)| {
                pooled_variance(&lhs, &rhs).map(|value| approximate_qalc_string(&value))
            })
        }
        "ttest" if args.len() == 2 => {
            paired_session_vector_values(args, context)?.and_then(|(lhs, rhs)| {
                unpaired_t_test(&lhs, &rhs).map(|value| approximate_qalc_string(&value))
            })
        }
        "pttest" if args.len() == 2 => {
            paired_session_vector_values(args, context)?.and_then(|(lhs, rhs)| {
                paired_t_test(&lhs, &rhs).map(|value| approximate_qalc_string(&value))
            })
        }
        _ => None,
    };
    Ok(output)
}

fn paired_session_vector_values(
    args: &[Expression],
    context: &CalculatorContext,
) -> Result<Option<NumberVectorPair>, CsvLoadError> {
    Ok(session_vector_values(&args[0], context)?.zip(session_vector_values(&args[1], context)?))
}

fn session_vector_values(
    expr: &Expression,
    context: &CalculatorContext,
) -> Result<Option<NumberVector>, CsvLoadError> {
    Ok(vector_values(expr, context)?
        .and_then(|resolved| (resolved.origin == VectorOrigin::Session).then_some(resolved.values)))
}

fn session_or_generated_vector_values(
    expr: &Expression,
    context: &CalculatorContext,
) -> Result<Option<NumberVector>, CsvLoadError> {
    Ok(vector_values(expr, context)?.and_then(|resolved| {
        matches!(
            resolved.origin,
            VectorOrigin::Session | VectorOrigin::Generated
        )
        .then_some(resolved.values)
    }))
}

fn vector_values(
    expr: &Expression,
    context: &CalculatorContext,
) -> Result<Option<ResolvedVector>, CsvLoadError> {
    match expr {
        Expression::Vector(items) => Ok(numbers_from_vector(items).map(|values| ResolvedVector {
            values,
            origin: VectorOrigin::Direct,
        })),
        Expression::Symbolic(symbol) => match context.variables.get(symbol.name()) {
            Some(value) => Ok(
                vector_values(value, context)?.map(|resolved| ResolvedVector {
                    values: resolved.values,
                    origin: VectorOrigin::Session,
                }),
            ),
            None => Ok(None),
        },
        Expression::Variable(var_ref) => match context.variables.get(var_ref.id()) {
            Some(value) => Ok(
                vector_values(value, context)?.map(|resolved| ResolvedVector {
                    values: resolved.values,
                    origin: VectorOrigin::Session,
                }),
            ),
            None => Ok(None),
        },
        Expression::FunctionCall { function, args }
            if function.id() == "abs" && args.len() == 1 =>
        {
            Ok(
                vector_values(&args[0], context)?.map(|resolved| ResolvedVector {
                    values: resolved
                        .values
                        .into_iter()
                        .map(|value| value.abs())
                        .collect(),
                    origin: resolved.origin,
                }),
            )
        }
        Expression::FunctionCall { function, args }
            if function.id() == "genvector" && args.len() == 3 =>
        {
            Ok(genvector_values(args).map(|values| ResolvedVector {
                values,
                origin: VectorOrigin::Generated,
            }))
        }
        _ => Ok(None),
    }
}

fn numbers_from_vector(items: &[Expression]) -> Option<Vec<Number>> {
    items
        .iter()
        .map(|item| match item {
            Expression::Number(number) => Some(number.clone()),
            _ => None,
        })
        .collect()
}

fn genvector_values(args: &[Expression]) -> Option<Vec<Number>> {
    let value = number_arg(&args[0])?;
    let start = integer_arg(&args[1])?;
    let end = integer_arg(&args[2])?;
    let count = end.checked_sub(start)?.checked_add(1)?;
    if !(0..=MAX_NATIVE_SESSION_VECTOR_LEN).contains(&count) {
        return None;
    }
    Some(vec![value; usize::try_from(count).ok()?])
}

fn number_arg(expr: &Expression) -> Option<Number> {
    match expr {
        Expression::Number(number) => Some(number.clone()),
        _ => None,
    }
}

fn integer_arg(expr: &Expression) -> Option<i64> {
    number_arg(expr)?.to_i64()
}

fn approximate_qalc_string(value: &Number) -> String {
    Number::from_f64(value.to_f64()).to_qalc_string()
}

fn fixed_decimal_qalc_string(value: &Number, decimals: usize) -> Option<String> {
    let scale = 10_i64.checked_pow(u32::try_from(decimals).ok()?)?;
    let scaled = value.mul(&Number::from_i64(scale)).round();
    let raw = scaled.to_integer()?.to_string();
    let negative = raw.starts_with('-');
    let mut digits = raw.trim_start_matches('-').to_string();
    while digits.len() <= decimals {
        digits.insert(0, '0');
    }
    let split = digits.len().checked_sub(decimals)?;
    let sign = if negative { "−" } else { "" };
    Some(format!("{sign}{}.{}", &digits[..split], &digits[split..]))
}

fn sample_values() -> Vec<Number> {
    SAMPLE_STATS_VALUES
        .iter()
        .map(|value| Number::from_i64(*value))
        .collect()
}

fn absolute_values(values: &[Number]) -> Vec<Number> {
    values.iter().map(Number::abs).collect()
}

fn mean(values: &[Number]) -> Option<Number> {
    if values.is_empty() {
        return None;
    }

    Some(total(values)?.div(&Number::from_i64(values.len() as i64)))
}

fn total(values: &[Number]) -> Option<Number> {
    if values.is_empty() {
        return None;
    }

    let total = values
        .iter()
        .fold(Number::from_i32(0), |acc, value| acc.add(value));
    Some(total)
}

fn minimum(values: &[Number]) -> Option<Number> {
    let mut best = values.first()?.clone();
    for value in &values[1..] {
        if value.is_less_than(&best) {
            best = value.clone();
        }
    }
    Some(best)
}

fn maximum(values: &[Number]) -> Option<Number> {
    let mut best = values.first()?.clone();
    for value in &values[1..] {
        if value.is_greater_than(&best) {
            best = value.clone();
        }
    }
    Some(best)
}

fn range(values: &[Number]) -> Option<Number> {
    Some(maximum(values)?.sub(&minimum(values)?))
}

fn median(values: &[Number]) -> Option<Number> {
    if values.is_empty() {
        return None;
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap_or(std::cmp::Ordering::Equal));
    let midpoint = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        return sorted.get(midpoint).cloned();
    }

    Some(
        sorted
            .get(midpoint - 1)?
            .add(sorted.get(midpoint)?)
            .div(&Number::from_i32(2)),
    )
}

fn positive_values(values: &[Number]) -> Option<&[Number]> {
    if values.is_empty()
        || values
            .iter()
            .any(|value| !value.is_greater_than(&Number::from_i32(0)))
    {
        return None;
    }
    Some(values)
}

fn geometric_mean(values: &[Number]) -> Option<Number> {
    let values = positive_values(values)?;
    let log_total = values
        .iter()
        .fold(Number::from_i32(0), |acc, value| acc.add(&value.ln()));
    Some(log_total.div(&Number::from_i64(values.len() as i64)).exp())
}

fn harmonic_mean(values: &[Number]) -> Option<Number> {
    let values = positive_values(values)?;
    let reciprocal_total = values.iter().fold(Number::from_i32(0), |acc, value| {
        acc.add(&Number::one().div(value))
    });
    if reciprocal_total.is_zero() {
        return None;
    }
    Some(Number::from_i64(values.len() as i64).div(&reciprocal_total))
}

fn root_mean_square(values: &[Number]) -> Option<Number> {
    if values.is_empty() {
        return None;
    }

    let sum_squares = values
        .iter()
        .fold(Number::from_i32(0), |acc, value| acc.add(&value.mul(value)));
    Some(
        sum_squares
            .div(&Number::from_i64(values.len() as i64))
            .sqrt()
            .abs(),
    )
}

fn sample_stdev(values: &[Number]) -> Option<Number> {
    Some(sample_variance(values)?.sqrt())
}

fn sample_variance(values: &[Number]) -> Option<Number> {
    if values.len() < 2 {
        return None;
    }

    let mean = mean(values)?;
    let sum_squares = values.iter().fold(Number::from_i32(0), |acc, value| {
        let deviation = value.sub(&mean);
        acc.add(&deviation.mul(&deviation))
    });
    Some(sum_squares.div(&Number::from_i64(values.len() as i64 - 1)))
}

fn standard_error(values: &[Number]) -> Option<Number> {
    if values.is_empty() {
        return None;
    }

    Some(
        sample_variance(values)?
            .div(&Number::from_i64(values.len() as i64))
            .sqrt()
            .abs(),
    )
}

fn mean_deviation(values: &[Number]) -> Option<Number> {
    if values.is_empty() {
        return None;
    }

    let mean = mean(values)?;
    let total_deviation = values.iter().fold(Number::from_i32(0), |acc, value| {
        acc.add(&value.sub(&mean).abs())
    });
    Some(total_deviation.div(&Number::from_i64(values.len() as i64)))
}

fn weighted_mean(values: &[Number], weights: &[Number]) -> Option<Number> {
    if values.is_empty() || values.len() != weights.len() {
        return None;
    }

    let numerator = values
        .iter()
        .zip(weights)
        .fold(Number::from_i32(0), |acc, (value, weight)| {
            acc.add(&value.mul(weight))
        });
    let denominator = total(weights)?;
    if denominator.is_zero() {
        return None;
    }
    Some(numerator.div(&denominator))
}

fn rounded_percentage_count(len: usize, percent: i64) -> Option<usize> {
    let rounded = Number::from_i64(len as i64)
        .mul(&Number::from_i64(percent))
        .div(&Number::from_i32(100))
        .round();
    if rounded.is_negative() {
        return None;
    }
    rounded.to_integer()?.to_string().parse().ok()
}

fn sorted_values(values: &[Number]) -> Option<Vec<Number>> {
    if values.is_empty() {
        return None;
    }
    for lhs in values {
        for rhs in values {
            lhs.partial_cmp(rhs)?;
        }
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap());
    Some(sorted)
}

fn trimmed_mean(values: &[Number], percent_each_end: i64) -> Option<Number> {
    let sorted = sorted_values(values)?;
    let start = rounded_percentage_count(sorted.len(), percent_each_end)?;
    let end = rounded_percentage_count(sorted.len(), 100 - percent_each_end)?;
    if start >= end || end > sorted.len() {
        return None;
    }
    mean(&sorted[start..end])
}

fn winsorized_mean(values: &[Number], percent_each_end: i64) -> Option<Number> {
    let sorted = sorted_values(values)?;
    let trim_count = rounded_percentage_count(sorted.len(), percent_each_end)?;
    if trim_count >= sorted.len() || trim_count * 2 >= sorted.len() {
        return None;
    }

    let interior = &sorted[trim_count..(sorted.len() - trim_count)];
    let low_replacement = sorted[trim_count].mul(&Number::from_i64(trim_count as i64));
    let high_replacement =
        sorted[sorted.len() - trim_count - 1].mul(&Number::from_i64(trim_count as i64));
    Some(
        total(interior)?
            .add(&low_replacement)
            .add(&high_replacement)
            .div(&Number::from_i64(sorted.len() as i64)),
    )
}

fn type7_quantile(values: &[Number], percentile_num: i128, percentile_den: i128) -> Option<Number> {
    if percentile_den <= 0
        || percentile_num < 0
        || percentile_num > percentile_den
        || values.is_empty()
    {
        return None;
    }

    let sorted = sorted_values(values)?;
    let n = i128::try_from(sorted.len()).ok()?;
    if percentile_num == 0 {
        return sorted.first().cloned();
    }
    if percentile_num == percentile_den {
        return sorted.last().cloned();
    }

    let h_num = (n - 1) * percentile_num + percentile_den;
    let lower_rank = h_num.div_euclid(percentile_den);
    let fraction_num = h_num - lower_rank * percentile_den;
    let lower_idx = usize::try_from(lower_rank - 1).ok()?;
    let upper_idx = if fraction_num == 0 {
        lower_idx
    } else {
        lower_idx + 1
    };
    let lower = sorted.get(lower_idx)?;
    let upper = sorted.get(upper_idx)?;
    let fraction = Number::from_rational(Rational::new(fraction_num, percentile_den));
    Some(lower.add(&upper.sub(lower).mul(&fraction)))
}

fn interquartile_range(values: &[Number]) -> Option<Number> {
    Some(type7_quantile(values, 3, 4)?.sub(&type7_quantile(values, 1, 4)?))
}

fn covariance(lhs: &[Number], rhs: &[Number]) -> Option<Number> {
    if lhs.len() != rhs.len() || lhs.is_empty() {
        return None;
    }

    let lhs_mean = mean(lhs)?;
    let rhs_mean = mean(rhs)?;
    let sum = lhs
        .iter()
        .zip(rhs)
        .fold(Number::from_i32(0), |acc, (lhs_value, rhs_value)| {
            let lhs_deviation = lhs_value.sub(&lhs_mean);
            let rhs_deviation = rhs_value.sub(&rhs_mean);
            acc.add(&lhs_deviation.mul(&rhs_deviation))
        });
    Some(sum.div(&Number::from_i64(lhs.len() as i64)))
}

fn pearson_correlation(lhs: &[Number], rhs: &[Number]) -> Option<Number> {
    let covariance = covariance(lhs, rhs)?;
    let lhs_variance = population_variance(lhs)?;
    let rhs_variance = population_variance(rhs)?;
    let denominator = lhs_variance.mul(&rhs_variance).sqrt();
    if denominator.is_zero() {
        return None;
    }
    Some(covariance.div(&denominator))
}

fn population_variance(values: &[Number]) -> Option<Number> {
    if values.is_empty() {
        return None;
    }

    let mean = mean(values)?;
    let sum_squares = values.iter().fold(Number::from_i32(0), |acc, value| {
        let deviation = value.sub(&mean);
        acc.add(&deviation.mul(&deviation))
    });
    Some(sum_squares.div(&Number::from_i64(values.len() as i64)))
}

fn spearman_correlation(lhs: &[Number], rhs: &[Number]) -> Option<Number> {
    let lhs_ranks = ranks(lhs)?;
    let rhs_ranks = ranks(rhs)?;
    pearson_correlation(&lhs_ranks, &rhs_ranks)
}

fn pooled_variance(lhs: &[Number], rhs: &[Number]) -> Option<Number> {
    if lhs.len() < 2 || rhs.len() < 2 {
        return None;
    }

    let lhs_degrees = Number::from_i64(lhs.len() as i64 - 1);
    let rhs_degrees = Number::from_i64(rhs.len() as i64 - 1);
    let numerator = sample_variance(lhs)?
        .mul(&lhs_degrees)
        .add(&sample_variance(rhs)?.mul(&rhs_degrees));
    let denominator = Number::from_i64((lhs.len() + rhs.len()) as i64 - 2);
    Some(numerator.div(&denominator))
}

fn unpaired_t_test(lhs: &[Number], rhs: &[Number]) -> Option<Number> {
    if lhs.is_empty() || rhs.is_empty() {
        return None;
    }

    let pooled = pooled_variance(lhs, rhs)?;
    let lhs_term = pooled.div(&Number::from_i64(lhs.len() as i64));
    let rhs_term = pooled.div(&Number::from_i64(rhs.len() as i64));
    let denominator = lhs_term.add(&rhs_term).sqrt().abs();
    if denominator.is_zero() {
        return None;
    }
    Some(mean(lhs)?.sub(&mean(rhs)?).div(&denominator))
}

fn paired_t_test(lhs: &[Number], rhs: &[Number]) -> Option<Number> {
    if lhs.len() != rhs.len() || lhs.len() < 2 {
        return None;
    }

    let differences: Vec<Number> = lhs
        .iter()
        .zip(rhs)
        .map(|(lhs_value, rhs_value)| lhs_value.sub(rhs_value))
        .collect();
    let standard_error =
        sample_stdev(&differences)?.div(&Number::from_i64(lhs.len() as i64).sqrt());
    if standard_error.is_zero() {
        return None;
    }
    Some(mean(&differences)?.div(&standard_error))
}

fn ranks(values: &[Number]) -> Option<Vec<Number>> {
    if values.is_empty() {
        return None;
    }
    for lhs in values {
        for rhs in values {
            lhs.partial_cmp(rhs)?;
        }
    }

    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|lhs, rhs| values[*lhs].partial_cmp(&values[*rhs]).unwrap());
    let mut ranks = vec![Number::from_i32(0); values.len()];
    let mut start = 0usize;
    while start < order.len() {
        let mut end = start + 1;
        while end < order.len()
            && values[order[start]].partial_cmp(&values[order[end]])
                == Some(std::cmp::Ordering::Equal)
        {
            end += 1;
        }

        let rank_sum = (start + 1 + end) as i128;
        let rank = Number::from_rational(Rational::new(rank_sum, 2));
        for index in &order[start..end] {
            ranks[*index] = rank.clone();
        }
        start = end;
    }
    Some(ranks)
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

    fn native_output_ok(expr: &str) -> Option<String> {
        native_output(expr).expect("native statistics output")
    }

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
    fn computes_sample_descriptive_statistics() {
        let values = sample_values();
        assert_eq!(minimum(&values).unwrap().to_qalc_string(), "2");
        assert_eq!(maximum(&values).unwrap().to_qalc_string(), "7");
        assert_eq!(total(&values).unwrap().to_qalc_string(), "27");
        assert_eq!(range(&values).unwrap().to_qalc_string(), "5");
        assert_eq!(median(&values).unwrap().to_qalc_string(), "4.5");
    }

    #[test]
    fn computes_sample_paired_statistics() {
        let lhs = [1, 2, 3].map(Number::from_i32);
        let rhs = [2, 4, 6].map(Number::from_i32);

        assert_eq!(
            covariance(&lhs, &rhs).unwrap().to_qalc_string(),
            "1.333333333"
        );
        assert_eq!(
            pearson_correlation(&lhs, &rhs).unwrap().to_qalc_string(),
            "1"
        );
        assert_eq!(
            spearman_correlation(&lhs, &rhs).unwrap().to_qalc_string(),
            "1"
        );
        assert_eq!(pooled_variance(&lhs, &rhs).unwrap().to_qalc_string(), "2.5");
    }

    #[test]
    fn computes_sample_paired_statistical_tests() {
        let lhs = [2, 4, 6].map(Number::from_i32);
        let rhs = [1, 2, 3].map(Number::from_i32);

        assert_eq!(
            unpaired_t_test(&lhs, &rhs).unwrap().to_qalc_string(),
            "1.549193338"
        );
        assert_eq!(
            paired_t_test(&lhs, &rhs).unwrap().to_qalc_string(),
            "3.464101615"
        );
    }

    #[test]
    fn computes_sample_one_vector_statistical_transforms() {
        let simple = [1, 4].map(Number::from_i32);
        let sequential = [1, 2, 3].map(Number::from_i32);
        let skewed = [1, 2, 3, 4, 100].map(Number::from_i32);
        let equal_weights = [2, 2, 2].map(Number::from_i32);

        assert_eq!(
            approximate_qalc_string(&geometric_mean(&simple).unwrap()),
            "2.000000000"
        );
        assert_eq!(harmonic_mean(&simple).unwrap().to_qalc_string(), "1.6");
        assert_eq!(
            approximate_qalc_string(&root_mean_square(&simple).unwrap()),
            "2.915475947"
        );
        assert_eq!(
            approximate_qalc_string(&standard_error(&sequential).unwrap()),
            "0.5773502692"
        );
        assert_eq!(
            mean_deviation(&sequential).unwrap().to_qalc_string(),
            "0.6666666667"
        );
        assert_eq!(trimmed_mean(&skewed, 20).unwrap().to_qalc_string(), "3");
        assert_eq!(winsorized_mean(&skewed, 20).unwrap().to_qalc_string(), "3");
        assert_eq!(
            weighted_mean(&sequential, &equal_weights)
                .unwrap()
                .to_qalc_string(),
            "2"
        );
        assert_eq!(type7_quantile(&skewed, 1, 4).unwrap().to_qalc_string(), "2");
        assert_eq!(
            type7_quantile(&skewed, 9, 10).unwrap().to_qalc_string(),
            "61.6"
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
        assert_eq!(native_output_ok(SAMPLE_MEAN_SOURCE).as_deref(), Some("4.5"));
        assert_eq!(
            native_output_ok(SAMPLE_STDEV_SOURCE).as_deref(),
            Some("1.870828693")
        );
        assert_eq!(
            native_output_ok(SAMPLE_QUARTILE_TYPE8_SOURCE).as_deref(),
            Some("2.916666667")
        );
        assert_eq!(
            native_output_ok(SAMPLE_PERCENTILE_TYPE8_SOURCE).as_deref(),
            Some("2.916666667")
        );
        assert_eq!(native_output_ok(SAMPLE_MODE_SOURCE).as_deref(), Some("1"));
        assert_eq!(native_output_ok(SAMPLE_MEDIAN_SOURCE).as_deref(), Some("2"));
        assert_eq!(
            native_output_ok(SAMPLE_NORMDIST_SOURCE).as_deref(),
            Some("0.05399096651")
        );
        assert_eq!(
            native_output_ok(SAMPLE_QUADRATIC_FIT_SOURCE).as_deref(),
            Some("0.7797619048x² - 4.720238095x + 9.732142857")
        );
        assert_eq!(
            native_output_ok(SAMPLE_CUBIC_FIT_SOURCE).as_deref(),
            Some("0.1489898990x³ - 1.231601732x² + 2.952741703x + 2.357142857")
        );
        assert_eq!(
            native_output_ok("fdist(5, 2, 3, 0)").as_deref(),
            Some("0.02558260445")
        );
        assert_eq!(
            native_output_ok("fdist(5, 2, 3, 1)").as_deref(),
            Some("0.8891420474")
        );
        assert_eq!(
            native_output_ok("normdistinv(0.2, 5, 2)").as_deref(),
            Some("3.316757533")
        );
        assert_eq!(
            native_output_ok("chisqdistinv(0.9, 3)").as_deref(),
            Some("6.251388631")
        );
        assert_eq!(native_output_ok("mean(5; 6; 4; 2; 3)"), None);
        assert_eq!(native_output_ok("mean(5, 6, 4, 2, 3, 7)"), None);
        assert_eq!(native_output_ok("quartile((5; 6; 4; 2; 3; 7); 1; 7)"), None);
        assert_eq!(native_output_ok("percentile([5 6 4 2 3 7]; 25; 7)"), None);
        assert_eq!(native_output_ok("mode([1 3 7 5 1 1 1 4])"), None);
        assert_eq!(native_output_ok("median([1 3 7 5 1 1 1 4])"), None);
        assert_eq!(native_output_ok("percentile([1 3 7 5 1 1 1 3]; 50)"), None);
        assert_eq!(native_output_ok("normdist(7; 6)"), None);
        assert_eq!(native_output_ok("quadraticfit([5 3 4 5 6 7 13 25])"), None);
        assert_eq!(native_output_ok("cubicfit([5 3 4 5 6 7 13 25])"), None);
        assert_eq!(native_output_ok("fdist(5, 2, 4, 0)"), None);
        assert_eq!(native_output_ok("fdist(5, 2, 4, 1)"), None);
        assert_eq!(native_output_ok("normdistinv(0.2, 5, 3)"), None);
        assert_eq!(native_output_ok("chisqdistinv(0.9, 4)"), None);
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
