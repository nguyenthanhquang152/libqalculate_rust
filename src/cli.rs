pub(crate) const HELP_TEXT: &str = concat!(include_str!("cli/help.txt"), "\n");

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliInvocation {
    pub(crate) immediate: Option<ImmediateAction>,
    pub(crate) interactive: bool,
    pub(crate) command_file: Option<CommandFile>,
    pub(crate) list: Option<ListRequest>,
    pub(crate) programming_mode: bool,
    pub(crate) unicode: Option<bool>,
    pub(crate) expression: Option<String>,
    pub(crate) settings: Vec<String>,
    pub(crate) terse: bool,
    pub(crate) timeout_ms: i32,
    pub(crate) output_mode: OutputMode,
    pub(crate) color: ColorMode,
    pub(crate) definitions: DefinitionSelection,
    pub(crate) diagnostics: Vec<String>,
    pub(crate) defaults: bool,
    pub(crate) exrates: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImmediateAction {
    Help,
    Version,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandFileMode {
    Commands,
    Test,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandFile {
    pub(crate) path: String,
    pub(crate) mode: CommandFileMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ListRequest {
    pub(crate) list_type: ListType,
    pub(crate) search_term: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListType {
    All,
    Functions,
    Units,
    Variables,
    Prefixes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputMode {
    Text,
    Latex,
    Html,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColorMode {
    Default,
    Off,
    On,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DefinitionSelection {
    pub(crate) units: bool,
    pub(crate) currencies: bool,
    pub(crate) functions: bool,
    pub(crate) variables: bool,
    pub(crate) datasets: bool,
    pub(crate) global_defs: bool,
}

fn decimal_prefix_bytes(bytes: impl IntoIterator<Item = u8>) -> i64 {
    let mut bytes = bytes.into_iter().peekable();
    while bytes.peek().is_some_and(|byte| byte.is_ascii_whitespace()) {
        bytes.next();
    }

    let negative = match bytes.peek().copied() {
        Some(b'-') => {
            bytes.next();
            true
        }
        Some(b'+') => {
            bytes.next();
            false
        }
        _ => false,
    };

    let mut saw_digit = false;
    let mut magnitude = 0_i128;
    for byte in bytes {
        if !byte.is_ascii_digit() {
            break;
        }
        saw_digit = true;
        magnitude = magnitude
            .saturating_mul(10)
            .saturating_add(i128::from(byte - b'0'));
    }
    if !saw_digit {
        return 0;
    }

    let signed = if negative { -magnitude } else { magnitude };
    signed.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

fn decimal_prefix(value: &str) -> i64 {
    decimal_prefix_bytes(value.bytes())
}

fn color_decimal_prefix(value: &str) -> i64 {
    if value.contains(' ') {
        decimal_prefix_bytes(
            value
                .bytes()
                .filter(|byte| !matches!(*byte, b' ' | b'\t' | b'\n')),
        )
    } else {
        decimal_prefix(value)
    }
}

fn split_option(arg: &str) -> (&str, &str) {
    let Some(split_at) = arg.find(|character: char| character.is_ascii_digit() || character == '=')
    else {
        return (arg, "");
    };
    // `str::find` returns a UTF-8 character boundary, and the matched digit or
    // '=' is one ASCII byte, so both slices below are boundary-safe.
    debug_assert!(arg.is_char_boundary(split_at));
    let split_byte = arg.as_bytes()[split_at];
    if split_at == 0
        || arg.starts_with('+')
        || (split_byte != b'=' && split_at != 2)
        || (split_byte == b'=' && split_at == arg.len() - 1)
    {
        return (arg, "");
    }

    let value_start = if split_byte == b'=' {
        split_at + 1
    } else {
        split_at
    };
    debug_assert!(arg.is_char_boundary(value_start));
    (&arg[..split_at], &arg[value_start..])
}

fn list_request(
    list_type: ListType,
    previous: Option<&ListRequest>,
    inline_value: &str,
    argv: &[String],
    index: &mut usize,
) -> ListRequest {
    let mut search_term = previous.and_then(|request| request.search_term.clone());
    if !inline_value.is_empty() {
        search_term = Some(inline_value.trim().to_string());
    } else if argv
        .get(*index + 1)
        .is_some_and(|next| !next.is_empty() && !next.starts_with('-') && !next.starts_with('+'))
    {
        *index += 1;
        search_term = Some(argv[*index].trim().to_string());
    }

    ListRequest {
        list_type,
        search_term,
    }
}

pub(crate) fn parse_args<I, S>(args: I) -> CliInvocation
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args_iter = args.into_iter();
    let _prog_name = args_iter.next(); // skip argv[0]
    let argv: Vec<String> = args_iter.map(|s| s.into()).collect();
    let argc = argv.len();

    let mut immediate: Option<ImmediateAction> = None;
    let mut interactive = false;
    let mut command_file: Option<CommandFile> = None;
    let mut list: Option<ListRequest> = None;
    let mut programming_mode = false;
    let mut unicode = None;

    let mut settings: Vec<String> = Vec::new();
    let mut terse = false;
    let mut timeout_ms: i32 = 0;
    let mut output_mode = OutputMode::Text;
    let mut color = ColorMode::Default;

    let mut units = true;
    let mut currencies = true;
    let mut functions = true;
    let mut variables = true;
    let mut datasets = true;
    let mut global_defs = true;

    let mut defaults = false;
    let mut exrates = false;
    let mut diagnostics: Vec<String> = Vec::new();

    let expression_after_argc = argv.iter().position(|arg| arg == "--");

    let mut i = 0;
    let mut calc_arg_begun = false;
    let mut calc_args: Vec<String> = Vec::new();

    while i < argc {
        let arg = &argv[i];

        if calc_arg_begun {
            calc_args.push(arg.clone());
            i += 1;
            continue;
        }

        if arg == "--" {
            calc_arg_begun = true;
            i += 1;
            continue;
        }

        // Match raw arg.as_str() first for raw branches
        let raw_matched = match arg.as_str() {
            "-help" | "--help" | "-h" => {
                immediate = Some(ImmediateAction::Help);
                true
            }
            "-version" | "--version" | "-v" | "-V" => {
                immediate = Some(ImmediateAction::Version);
                true
            }
            "-terse" | "--terse" | "-t" => {
                terse = true;
                true
            }
            "-interactive" | "--interactive" | "-i" => {
                interactive = true;
                true
            }
            "-u8" => {
                unicode = Some(true);
                true
            }
            "+u8" => {
                unicode = Some(false);
                true
            }
            "-exrates" | "--exrates" | "-e" => {
                exrates = true;
                true
            }
            "+p" => {
                settings.push("base 10 10".to_string());
                settings.push("xor^ 0".to_string());
                true
            }
            "-nounits" | "--nounits" => {
                units = false;
                true
            }
            "-nocurrencies" | "--nocurrencies" => {
                currencies = false;
                true
            }
            "-nofunctions" | "--nofunctions" => {
                functions = false;
                true
            }
            "-novariables" | "--novariables" => {
                variables = false;
                true
            }
            "-nodatasets" | "--nodatasets" => {
                datasets = false;
                true
            }
            "-nodefs" | "--nodefs" | "-n" => {
                global_defs = false;
                true
            }
            "-latex" | "--latex" => {
                output_mode = OutputMode::Latex;
                true
            }
            "-html" | "--html" => {
                output_mode = OutputMode::Html;
                true
            }
            _ => false,
        };

        if raw_matched {
            if immediate.is_some() {
                break;
            }
            i += 1;
            continue;
        }

        // Otherwise, split for normalized branches.
        let (svar, svalue) = split_option(arg);

        let is_option_like = {
            let first_char = svar.chars().next();
            let second_char = svar.chars().nth(1);
            svar.len() > 1
                && (first_char == Some('-') || first_char == Some('+'))
                && second_char.is_some_and(|c| !c.is_ascii_digit() && c != '.' && c != ':')
        };

        let mut matched = true;
        match svar {
            "-base" | "--base" | "-b" => {
                let mut base_val = String::new();
                if !svalue.is_empty() {
                    base_val = svalue.to_string();
                } else if i + 1 < argc {
                    i += 1;
                    base_val = argv[i].clone();
                }
                settings.push(format!("base {}", base_val));
            }
            "-color" | "--color" | "-c" => {
                let mut c_val = ColorMode::On;
                if !svalue.is_empty() {
                    let parsed = color_decimal_prefix(svalue);
                    if parsed < 0 {
                        c_val = ColorMode::Default;
                    } else if parsed == 0 {
                        c_val = ColorMode::Off;
                    } else {
                        c_val = ColorMode::On;
                    }
                }
                color = c_val;
            }
            "-p" => {
                programming_mode = true;
                let base_setting = if !svalue.is_empty() {
                    format!("base {0} {0}", svalue)
                } else if i + 1 < argc {
                    i += 1;
                    if argv[i] == "--" {
                        diagnostics.push("Illegal base.".to_string());
                        "base -- --".to_string()
                    } else {
                        format!("base {0} {0}", argv[i])
                    }
                } else {
                    "base ".to_string()
                };
                settings.push(base_setting);
                settings.push("xor^ 1".to_string());
            }
            "-list" | "--list" | "-l" => {
                list = Some(list_request(
                    ListType::All,
                    list.as_ref(),
                    svalue,
                    &argv,
                    &mut i,
                ));
            }
            "--list-functions" => {
                list = Some(list_request(
                    ListType::Functions,
                    list.as_ref(),
                    svalue,
                    &argv,
                    &mut i,
                ));
            }
            "--list-units" => {
                list = Some(list_request(
                    ListType::Units,
                    list.as_ref(),
                    svalue,
                    &argv,
                    &mut i,
                ));
            }
            "--list-variables" => {
                list = Some(list_request(
                    ListType::Variables,
                    list.as_ref(),
                    svalue,
                    &argv,
                    &mut i,
                ));
            }
            "--list-prefixes" => {
                list = Some(list_request(
                    ListType::Prefixes,
                    list.as_ref(),
                    svalue,
                    &argv,
                    &mut i,
                ));
            }
            "-defaults" | "--defaults" => {
                defaults = true;
            }
            "-time" | "--time" | "-m" => {
                let mut val_str = String::new();
                if !svalue.is_empty() {
                    val_str = svalue.to_string();
                } else if i + 1 < argc {
                    i += 1;
                    val_str = argv[i].clone();
                }
                let parsed =
                    decimal_prefix(&val_str).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
                timeout_ms = timeout_ms.saturating_add(parsed);
                if timeout_ms < 0 {
                    timeout_ms = 0;
                }
            }
            "-set" | "--set" | "-s" => {
                let mut val_str = String::new();
                if !svalue.is_empty() {
                    val_str = svalue.to_string();
                } else if i + 1 < argc {
                    i += 1;
                    val_str = argv[i].clone();
                }
                if !val_str.is_empty() {
                    for part in val_str.split_terminator(';') {
                        let normalized = part
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .to_ascii_lowercase();
                        let normalized = normalized
                            .strip_prefix("/set ")
                            .or_else(|| normalized.strip_prefix("set "))
                            .unwrap_or(&normalized);
                        if normalized.starts_with("programming mode ") {
                            // qalc's programming mode is a dedicated `-p`
                            // flag, not a user-settable session option.
                            diagnostics.push("Unrecognized option.\n".to_string());
                        } else {
                            settings.push(part.to_string());
                        }
                    }
                } else {
                    diagnostics.push("No option and value specified for set command.".to_string());
                }
            }
            "-file" | "-f" | "--file" => {
                let mut file_path = String::new();
                if !svalue.is_empty() {
                    file_path = svalue.trim().to_string();
                } else if i + 1 < argc {
                    i += 1;
                    file_path = argv[i].trim().to_string();
                }
                if file_path.is_empty() {
                    diagnostics.push("No file specified.".to_string());
                }
                command_file = Some(CommandFile {
                    path: file_path,
                    mode: CommandFileMode::Commands,
                });
            }
            "--test-file" => {
                let mut file_path = String::new();
                if !svalue.is_empty() {
                    file_path = svalue.trim().to_string();
                } else if i + 1 < argc {
                    i += 1;
                    file_path = argv[i].trim().to_string();
                }
                if file_path.is_empty() {
                    diagnostics.push("No file specified.".to_string());
                }
                defaults = true;
                terse = true;
                unicode = Some(false);
                command_file = Some(CommandFile {
                    path: file_path,
                    mode: CommandFileMode::Test,
                });
                interactive = false;
                break; // Stop parsing immediately
            }
            _ => {
                matched = false;
            }
        }

        if matched {
            i += 1;
            continue;
        }

        if is_option_like {
            let is_unrecognized = if let Some(sep_idx) = expression_after_argc {
                i < sep_idx
            } else {
                false
            };
            if is_unrecognized {
                diagnostics.push(format!("Unrecognized option: {}.", svar));
            } else {
                calc_arg_begun = true;
                calc_args.push(arg.clone());
            }
            i += 1;
        } else {
            calc_arg_begun = true;
            calc_args.push(arg.clone());
            i += 1;
        }
    }

    let expression = if calc_args.is_empty() {
        None
    } else {
        Some(calc_args.join(" "))
    };

    CliInvocation {
        immediate,
        interactive,
        command_file,
        list,
        programming_mode,
        unicode,
        expression,
        settings,
        terse,
        timeout_ms,
        output_mode,
        color,
        definitions: DefinitionSelection {
            units,
            currencies,
            functions,
            variables,
            datasets,
            global_defs,
        },
        diagnostics,
        defaults,
        exrates,
    }
}

#[cfg(test)]
mod tests;
