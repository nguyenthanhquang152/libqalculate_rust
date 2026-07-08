use libqalculate_rust::ast::{DateTimeLiteral, Expression};
use libqalculate_rust::datetime::{DateTime, DateTimeError};
use libqalculate_rust::number::Number;

fn n(value: i64) -> Number {
    Number::from_i64(value)
}

#[test]
fn constructs_valid_dates_and_rejects_invalid_calendar_fields() {
    let date = DateTime::from_ymd(2020, 5, 20).expect("valid date");
    assert_eq!((date.year(), date.month(), date.day()), (2020, 5, 20));
    assert_eq!((date.hour(), date.minute()), (0, 0));
    assert_eq!(date.second(), &Number::new());
    assert!(!date.time_is_set());

    let leap_day = DateTime::from_ymd(2020, 2, 29).expect("valid leap day");
    assert_eq!(
        (leap_day.year(), leap_day.month(), leap_day.day()),
        (2020, 2, 29)
    );

    assert!(matches!(
        DateTime::from_ymd(2020, 13, 1),
        Err(DateTimeError::InvalidMonth { month: 13 })
    ));
    assert!(matches!(
        DateTime::from_ymd(2020, 2, 30),
        Err(DateTimeError::InvalidDay {
            year: 2020,
            month: 2,
            day: 30
        })
    ));
    assert!(matches!(
        DateTime::from_ymd(2019, 2, 29),
        Err(DateTimeError::InvalidDay {
            year: 2019,
            month: 2,
            day: 29
        })
    ));
    assert!(matches!(
        DateTime::from_ymd(i64::MAX, 1, 1),
        Err(DateTimeError::OutOfRange)
    ));
}

#[test]
fn validates_time_fields_and_leap_second_position() {
    let date_time = DateTime::from_ymd_hms(2020, 5, 20, 23, 59, n(59)).expect("valid time");
    assert_eq!((date_time.hour(), date_time.minute()), (23, 59));
    assert_eq!(date_time.second(), &n(59));
    assert!(date_time.time_is_set());

    assert!(matches!(
        DateTime::from_ymd_hms(2020, 5, 20, 24, 0, Number::new()),
        Err(DateTimeError::InvalidHour { hour: 24 })
    ));
    assert!(matches!(
        DateTime::from_ymd_hms(2020, 5, 20, 23, 60, Number::new()),
        Err(DateTimeError::InvalidMinute { minute: 60 })
    ));
    assert!(matches!(
        DateTime::from_ymd_hms(2020, 5, 20, 23, 59, n(60)),
        Err(DateTimeError::InvalidSecond { .. })
    ));

    let leap_second =
        DateTime::from_ymd_hms(2016, 12, 31, 23, 59, n(60)).expect("upstream leap second boundary");
    assert_eq!(leap_second.second(), &n(60));
}

#[test]
fn preserves_upstream_day_month_and_year_carry_arithmetic() {
    let shifted = DateTime::from_ymd(2020, 5, 20)
        .unwrap()
        .add_days(&n(523))
        .unwrap();
    assert_eq!(shifted, DateTime::from_ymd(2021, 10, 25).unwrap());

    let month_carry = DateTime::from_ymd(2020, 1, 31)
        .unwrap()
        .add_months(&n(1))
        .unwrap();
    assert_eq!(month_carry, DateTime::from_ymd(2020, 3, 2).unwrap());

    let year_carry = DateTime::from_ymd(2020, 2, 29)
        .unwrap()
        .add_years(&n(1))
        .unwrap();
    assert_eq!(year_carry, DateTime::from_ymd(2021, 3, 1).unwrap());

    let half_month = DateTime::from_ymd(2020, 1, 15)
        .unwrap()
        .add_months(&"0.5".parse::<Number>().unwrap())
        .unwrap();
    assert_eq!(
        half_month,
        DateTime::from_ymd_hms(2020, 1, 30, 12, 0, Number::new()).unwrap()
    );

    let half_year = DateTime::from_ymd(2020, 1, 1)
        .unwrap()
        .add_years(&"0.5".parse::<Number>().unwrap())
        .unwrap();
    assert_eq!(half_year, DateTime::from_ymd(2020, 7, 2).unwrap());

    let year_end_half_year = DateTime::from_ymd(2020, 12, 31)
        .unwrap()
        .add_years(&"0.5".parse::<Number>().unwrap())
        .unwrap();
    assert_eq!(
        (
            year_end_half_year.year(),
            year_end_half_year.month(),
            year_end_half_year.day(),
            year_end_half_year.hour(),
            year_end_half_year.minute()
        ),
        (2021, 7, 2, 11, 56)
    );
    let expected_second = n(7_862_400).div(&n(183)).sub(&n(42_960));
    assert_eq!(year_end_half_year.second(), &expected_second);
}

#[test]
fn converts_to_and_from_utc_timestamps_exactly() {
    let date = DateTime::from_ymd_hms(2020, 5, 20, 0, 0, Number::new()).unwrap();
    assert_eq!(date.timestamp_utc(), n(1_589_932_800));

    let round_trip = DateTime::from_timestamp_utc(&n(1_589_932_800)).unwrap();
    assert_eq!(round_trip, date);

    let fractional = DateTime::from_timestamp_utc(&"1.5".parse::<Number>().unwrap()).unwrap();
    assert_eq!(
        (fractional.year(), fractional.month(), fractional.day()),
        (1970, 1, 1)
    );
    assert_eq!((fractional.hour(), fractional.minute()), (0, 0));
    assert_eq!(fractional.second(), &"1.5".parse::<Number>().unwrap());
}

#[test]
fn computes_ordering_and_positive_or_negative_differences() {
    let early = DateTime::from_ymd(2020, 10, 5).unwrap();
    let late = DateTime::from_ymd(2020, 11, 5).unwrap();

    assert!(early < late);
    assert_eq!(early.days_to(&late), n(31));
    assert_eq!(late.days_to(&early), n(-31));

    let ten_days_later = DateTime::from_ymd(2020, 10, 15).unwrap();
    assert_eq!(early.days_to(&ten_days_later), n(10));
    assert_eq!(ten_days_later.days_to(&early), n(-10));

    let one_minute = DateTime::from_ymd_hms(1970, 1, 1, 0, 1, Number::new()).unwrap();
    assert_eq!(DateTime::epoch_utc().seconds_to(&one_minute), n(60));
    assert_eq!(one_minute.seconds_to(&DateTime::epoch_utc()), n(-60));
}

#[test]
fn ast_datetime_leaf_can_carry_validated_value_without_losing_source_text() {
    let value = DateTime::from_ymd(2020, 5, 20).unwrap();
    let literal = DateTimeLiteral::from_value(value.clone());
    assert_eq!(literal.source(), "2020-05-20");
    assert_eq!(literal.value(), Some(&value));

    let expr = Expression::DateTime(literal);
    assert_eq!(
        expr.structure_kind(),
        libqalculate_rust::ast::StructureKind::DateTime
    );
    assert_eq!(expr.child_count(), 0);
}
