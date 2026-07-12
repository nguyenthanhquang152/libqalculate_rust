use super::ListType;
use libqalculate_rust::parser::commands::{
    parse_command, ApproximationMode, AssumeKind, SessionCommand, SetCommand, SetSetting,
};
use libqalculate_rust::parser::lexer::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InteractiveCommand {
    Noop,
    Quit,
    History,
    ClearHistory,
    SetClearHistory(bool),
    Settings(Vec<SessionCommand>),
    Help(Option<String>),
    List {
        list_type: ListType,
        query: Option<String>,
    },
    Info(String),
    Delete(String),
    DefineVariable {
        name: String,
        expression: String,
    },
    DefineFunction {
        name: String,
        expression: String,
    },
    Unknown,
    Expression(String),
}

pub(crate) fn parse_interactive_command(line: &str) -> Result<InteractiveCommand, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(InteractiveCommand::Noop);
    }

    let without_slash = trimmed.strip_prefix('/').unwrap_or(trimmed).trim();
    let lower = without_slash.to_ascii_lowercase();
    if matches!(lower.as_str(), "quit" | "exit") {
        return Ok(InteractiveCommand::Quit);
    }
    if lower == "history" {
        return Ok(InteractiveCommand::History);
    }
    if lower == "clear history" {
        return Ok(InteractiveCommand::ClearHistory);
    }

    if let Some(value) = lower.strip_prefix("set clear history ") {
        return match value.trim() {
            "1" | "on" | "true" | "yes" => Ok(InteractiveCommand::SetClearHistory(true)),
            "0" | "off" | "false" | "no" => Ok(InteractiveCommand::SetClearHistory(false)),
            _ => Err("Invalid value for clear history setting.".to_string()),
        };
    }

    if let Some(rest) = lower.strip_prefix("set base ") {
        let bases = rest
            .split_whitespace()
            .map(parse_base)
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| "Illegal base.".to_string())?;
        let setting = |setting| {
            SessionCommand::Set(SetCommand {
                setting,
                span: Span::new(0, trimmed.len()),
            })
        };
        return match bases.as_slice() {
            [output] => Ok(InteractiveCommand::Settings(vec![setting(
                SetSetting::OutputBase(*output),
            )])),
            [output, input] => Ok(InteractiveCommand::Settings(vec![
                setting(SetSetting::OutputBase(*output)),
                setting(SetSetting::InputBase(*input)),
            ])),
            _ => Err("No base specified.".to_string()),
        };
    }

    if lower.starts_with("set ")
        || lower.starts_with("assume ")
        || lower.starts_with("assumptions ")
    {
        return parse_command(trimmed)
            .map(|setting| InteractiveCommand::Settings(vec![setting]))
            .map_err(|error| error.to_string());
    }

    let mut command_parts = without_slash.splitn(2, char::is_whitespace);
    let command_name = command_parts.next().unwrap_or_default();
    let command_argument = command_parts.next().map(str::trim).unwrap_or_default();

    if command_name.eq_ignore_ascii_case("variable") {
        let (name, expression) = parse_named_definition(command_argument)?;
        return Ok(InteractiveCommand::DefineVariable { name, expression });
    }

    if command_name.eq_ignore_ascii_case("function") {
        let (name, expression) = parse_named_definition(command_argument)?;
        return Ok(InteractiveCommand::DefineFunction { name, expression });
    }

    if command_name.eq_ignore_ascii_case("help") {
        return Ok(InteractiveCommand::Help(
            (!command_argument.is_empty()).then(|| command_argument.to_string()),
        ));
    }

    if command_name.eq_ignore_ascii_case("info") && !command_argument.is_empty() {
        return Ok(InteractiveCommand::Info(command_argument.to_string()));
    }

    if command_name.eq_ignore_ascii_case("delete") {
        if command_argument.is_empty() {
            return Err("No variable specified.".to_string());
        }
        return Ok(InteractiveCommand::Delete(command_argument.to_string()));
    }

    if command_name.eq_ignore_ascii_case("list") || command_name.eq_ignore_ascii_case("find") {
        if command_argument.is_empty() {
            return Ok(InteractiveCommand::List {
                list_type: ListType::All,
                query: None,
            });
        }
        let rest = command_argument;
        let mut parts = rest.split_whitespace();
        let first = parts.next().unwrap_or_default();
        let (list_type, query_first) = match first.to_ascii_lowercase().as_str() {
            "functions" | "function" => (ListType::Functions, None),
            "units" | "unit" => (ListType::Units, None),
            "variables" | "variable" => (ListType::Variables, None),
            "prefixes" | "prefix" => (ListType::Prefixes, None),
            _ => (ListType::All, Some(first)),
        };
        let mut query = query_first.map(str::to_string).unwrap_or_default();
        for part in parts {
            if !query.is_empty() {
                query.push(' ');
            }
            query.push_str(part);
        }
        return Ok(InteractiveCommand::List {
            list_type,
            query: (!query.is_empty()).then_some(query),
        });
    }

    let conversion_target = without_slash.strip_prefix("->").map(str::trim).or_else(|| {
        (command_name.eq_ignore_ascii_case("to") || command_name.eq_ignore_ascii_case("convert"))
            .then_some(command_argument)
    });
    if let Some(target) = conversion_target {
        if target.is_empty() {
            return Err("No conversion target specified.".to_string());
        }
        return Ok(InteractiveCommand::Expression(format!("ans to {target}")));
    }

    if trimmed.starts_with('/') {
        return Ok(InteractiveCommand::Unknown);
    }

    Ok(InteractiveCommand::Expression(trimmed.to_string()))
}

fn parse_named_definition(argument: &str) -> Result<(String, String), String> {
    let argument = argument.trim();
    if argument.is_empty() {
        return Err("Illegal name.".to_string());
    }

    let (name, expression) = if let Some(quoted) = argument.strip_prefix('"') {
        let Some(closing_quote) = quoted.find('"') else {
            return Err("Illegal name.".to_string());
        };
        (&quoted[..closing_quote], quoted[closing_quote + 1..].trim())
    } else {
        argument
            .find(char::is_whitespace)
            .map_or((argument, ""), |separator| {
                (&argument[..separator], argument[separator..].trim())
            })
    };

    if name.is_empty() {
        return Err("Illegal name.".to_string());
    }
    let expression = expression
        .strip_prefix('"')
        .and_then(|expression| expression.strip_suffix('"'))
        .unwrap_or(expression);
    Ok((name.to_string(), expression.to_string()))
}

pub(crate) fn serialize_setting(command: &SessionCommand) -> String {
    match command {
        SessionCommand::Set(command) => match &command.setting {
            SetSetting::Approximation(mode) => format!(
                "approximation {}",
                match mode {
                    ApproximationMode::Exact => "exact",
                    ApproximationMode::TryExact => "try exact",
                    ApproximationMode::Approximate => "approximate",
                }
            ),
            SetSetting::FractionFormat(value) => format!("fraction format {value}"),
            SetSetting::Unicode(value) => format!("unicode {}", u8::from(*value)),
            SetSetting::IntervalCalculation(value) => {
                format!("interval calculation {value}")
            }
            SetSetting::InputBase(value) => format!("input base {value}"),
            SetSetting::OutputBase(value) => format!("output base {value}"),
            SetSetting::Precision(value) => format!("precision {value}"),
            SetSetting::IntervalDisplay(value) => format!("interval display {value}"),
            SetSetting::ConciseUncertainty(value) => {
                format!("concise uncertainty {}", u8::from(*value))
            }
            SetSetting::Complex(value) => format!("complex {value}"),
            SetSetting::DecimalComma(value) => format!("decimal comma {}", u8::from(*value)),
            SetSetting::CurrencyConversion(value) => format!("curconv {value}"),
            SetSetting::Percent(value) => format!("percent {value}"),
            SetSetting::Abbreviations(value) => format!("abbreviations {}", u8::from(*value)),
            SetSetting::EngineeringDisplay(value) => format!("edisp {value}"),
            SetSetting::MinExponent(value) => format!("exp {value}"),
            SetSetting::MinDecimals(value) => format!("min decimals {value}"),
            SetSetting::MaxDecimals(value) => format!("max decimals {value}"),
        },
        SessionCommand::Assume(command) => match command.kind {
            AssumeKind::Positive => "assume positive".to_string(),
            AssumeKind::Unknown => "assume unknown".to_string(),
        },
    }
}

fn parse_base(value: &str) -> Option<u32> {
    let base = match value.to_ascii_lowercase().as_str() {
        "bin" | "binary" => 2,
        "oct" | "octal" => 8,
        "dec" | "decimal" => 10,
        "hex" | "hexadecimal" => 16,
        other => other.parse().ok()?,
    };
    (2..=36).contains(&base).then_some(base)
}

#[cfg(test)]
mod tests {
    use super::{parse_interactive_command, serialize_setting, InteractiveCommand};
    use libqalculate_rust::parser::commands::{SessionCommand, SetSetting};

    #[test]
    fn parses_repl_control_and_setting_commands() {
        assert_eq!(
            parse_interactive_command("/quit"),
            Ok(InteractiveCommand::Quit)
        );
        let base = parse_interactive_command("set base 16").expect("base command");
        let InteractiveCommand::Settings(mut base) = base else {
            panic!("expected typed base setting");
        };
        let SessionCommand::Set(base) = base.pop().expect("one base setting") else {
            panic!("expected set command");
        };
        assert_eq!(base.setting, SetSetting::OutputBase(16));
        assert_eq!(
            serialize_setting(&SessionCommand::Set(base)),
            "output base 16"
        );
        assert!(matches!(
            parse_interactive_command("set base 10 16"),
            Ok(InteractiveCommand::Settings(settings))
                if settings.len() == 2
                    && serialize_setting(&settings[0]) == "output base 10"
                    && serialize_setting(&settings[1]) == "input base 16"
        ));
        let unicode = parse_interactive_command("/set unicode off").expect("Unicode command");
        let InteractiveCommand::Settings(mut unicode) = unicode else {
            panic!("expected typed Unicode setting");
        };
        let unicode = unicode.pop().expect("one Unicode setting");
        assert_eq!(serialize_setting(&unicode), "unicode 0");
        assert_eq!(
            parse_interactive_command("clear history"),
            Ok(InteractiveCommand::ClearHistory)
        );
        assert_eq!(
            parse_interactive_command("set clear history 1"),
            Ok(InteractiveCommand::SetClearHistory(true))
        );
        assert_eq!(
            parse_interactive_command("HeLp history"),
            Ok(InteractiveCommand::Help(Some("history".to_string())))
        );
        assert_eq!(
            parse_interactive_command("InFo CaseSensitiveName"),
            Ok(InteractiveCommand::Info("CaseSensitiveName".to_string()))
        );
        assert_eq!(
            parse_interactive_command("delete CaseSensitiveName"),
            Ok(InteractiveCommand::Delete("CaseSensitiveName".to_string()))
        );
        assert_eq!(
            parse_interactive_command("LiSt functions CaseSensitiveName"),
            Ok(InteractiveCommand::List {
                list_type: super::ListType::Functions,
                query: Some("CaseSensitiveName".to_string()),
            })
        );
        assert_eq!(
            parse_interactive_command("FiNd variables CaseSensitiveName"),
            Ok(InteractiveCommand::List {
                list_type: super::ListType::Variables,
                query: Some("CaseSensitiveName".to_string()),
            })
        );
        assert_eq!(
            parse_interactive_command("to cm"),
            Ok(InteractiveCommand::Expression("ans to cm".to_string()))
        );
        assert_eq!(
            parse_interactive_command("->cm"),
            Ok(InteractiveCommand::Expression("ans to cm".to_string()))
        );
        assert_eq!(
            parse_interactive_command("/typo"),
            Ok(InteractiveCommand::Unknown)
        );
    }

    #[test]
    fn parses_variable_and_function_definition_commands() {
        assert_eq!(
            parse_interactive_command("variable rate 5"),
            Ok(InteractiveCommand::DefineVariable {
                name: "rate".to_string(),
                expression: "5".to_string(),
            })
        );
        assert_eq!(
            parse_interactive_command(r#"function "twice" "2*\x""#),
            Ok(InteractiveCommand::DefineFunction {
                name: "twice".to_string(),
                expression: r"2*\x".to_string(),
            })
        );
        assert_eq!(
            parse_interactive_command("variable zero"),
            Ok(InteractiveCommand::DefineVariable {
                name: "zero".to_string(),
                expression: String::new(),
            })
        );
        assert_eq!(
            parse_interactive_command(r#"variable date "2024-01-01""#),
            Ok(InteractiveCommand::DefineVariable {
                name: "date".to_string(),
                expression: "2024-01-01".to_string(),
            })
        );
        assert_eq!(
            parse_interactive_command("function empty"),
            Ok(InteractiveCommand::DefineFunction {
                name: "empty".to_string(),
                expression: String::new(),
            })
        );
    }
}
