//! Tests for session command parser.

use libqalculate_rust::parser::commands::{
    parse_command, ApproximationMode, AssumeKind, SessionCommand, SetSetting,
};
use libqalculate_rust::parser::operators::ParseErrorKind;

#[test]
fn test_parse_set_unicode() {
    let cmd = parse_command("/set unicode 1").unwrap();
    match cmd {
        SessionCommand::Set(c) => {
            assert_eq!(c.setting, SetSetting::Unicode(true));
        }
        _ => panic!("Expected Set command"),
    }

    let cmd = parse_command("set unicode false").unwrap();
    match cmd {
        SessionCommand::Set(c) => {
            assert_eq!(c.setting, SetSetting::Unicode(false));
        }
        _ => panic!("Expected Set command"),
    }
}

#[test]
fn test_parse_set_approximation() {
    let cmd = parse_command("/set approximation exact").unwrap();
    match cmd {
        SessionCommand::Set(c) => {
            assert_eq!(
                c.setting,
                SetSetting::Approximation(ApproximationMode::Exact)
            );
        }
        _ => panic!("Expected Set command"),
    }

    let cmd = parse_command("set approx try exact").unwrap();
    match cmd {
        SessionCommand::Set(c) => {
            assert_eq!(
                c.setting,
                SetSetting::Approximation(ApproximationMode::TryExact)
            );
        }
        _ => panic!("Expected Set command"),
    }

    let cmd = parse_command("/set approximation approximate").unwrap();
    match cmd {
        SessionCommand::Set(c) => {
            assert_eq!(
                c.setting,
                SetSetting::Approximation(ApproximationMode::Approximate)
            );
        }
        _ => panic!("Expected Set command"),
    }
}

#[test]
fn test_parse_set_bases() {
    let cmd = parse_command("set input base 16").unwrap();
    match cmd {
        SessionCommand::Set(c) => {
            assert_eq!(c.setting, SetSetting::InputBase(16));
        }
        _ => panic!("Expected Set command"),
    }

    let cmd = parse_command("/set outbase 10").unwrap();
    match cmd {
        SessionCommand::Set(c) => {
            assert_eq!(c.setting, SetSetting::OutputBase(10));
        }
        _ => panic!("Expected Set command"),
    }
}

#[test]
fn test_parse_set_fraction_format() {
    let cmd = parse_command("/set fr 2").unwrap();
    match cmd {
        SessionCommand::Set(c) => {
            assert_eq!(c.setting, SetSetting::FractionFormat(2));
        }
        _ => panic!("Expected Set command"),
    }
}

#[test]
fn test_parse_set_interval_calculation() {
    let cmd = parse_command("/set ic 2").unwrap();
    match cmd {
        SessionCommand::Set(c) => {
            assert_eq!(c.setting, SetSetting::IntervalCalculation(2));
        }
        _ => panic!("Expected Set command"),
    }
}

#[test]
fn test_parse_set_precision() {
    let cmd = parse_command("/set precision 128").unwrap();
    match cmd {
        SessionCommand::Set(c) => {
            assert_eq!(c.setting, SetSetting::Precision(128));
        }
        _ => panic!("Expected Set command"),
    }

    let err = parse_command("/set precision 0").unwrap_err();
    assert_eq!(err.kind(), ParseErrorKind::InvalidSettingValue);

    // Decoupled syntax limit: 4097 is a valid syntactical precision, parsed into the AST.
    let cmd = parse_command("/set precision 4097").unwrap();
    match cmd {
        SessionCommand::Set(c) => {
            assert_eq!(c.setting, SetSetting::Precision(4097));
        }
        _ => panic!("Expected Set command"),
    }
}

#[test]
fn test_parse_assume() {
    let cmd = parse_command("/assume positive").unwrap();
    match cmd {
        SessionCommand::Assume(c) => {
            assert_eq!(c.kind, AssumeKind::Positive);
        }
        _ => panic!("Expected Assume command"),
    }

    let cmd = parse_command("assume unknown").unwrap();
    match cmd {
        SessionCommand::Assume(c) => {
            assert_eq!(c.kind, AssumeKind::Unknown);
        }
        _ => panic!("Expected Assume command"),
    }
}

#[test]
fn test_parse_errors() {
    let err = parse_command("/foo bar").unwrap_err();
    assert_eq!(err.kind(), ParseErrorKind::UnknownCommand);

    let err = parse_command("/set unknown_setting 1").unwrap_err();
    assert_eq!(err.kind(), ParseErrorKind::UnknownSetting);

    let err = parse_command("/set unicode hello").unwrap_err();
    assert_eq!(err.kind(), ParseErrorKind::InvalidSettingValue);

    let err = parse_command("/assume invalid_assumption").unwrap_err();
    assert_eq!(err.kind(), ParseErrorKind::InvalidSettingValue);
}
