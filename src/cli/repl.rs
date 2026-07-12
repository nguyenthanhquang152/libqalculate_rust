use super::commands::{parse_interactive_command, InteractiveCommand};
use super::{CliInvocation, ListRequest, ListType};
use libqalculate_rust::parser::commands::SessionCommand;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

const PROMPT: &str = "> ";
const MAX_HISTORY_ENTRIES: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReplEvaluation {
    pub(crate) output: String,
    pub(crate) answer_rendering: Option<String>,
    pub(crate) assignment_renderings: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReplRequest {
    Evaluate(String),
    Delete(String),
    ReformatLastAnswer,
}

#[derive(Debug)]
struct History {
    path: Option<PathBuf>,
    entries: Vec<String>,
    clear_on_exit: bool,
}

impl History {
    fn load() -> Self {
        let path = history_path();
        let entries = match fs::read_to_string(&path) {
            Ok(contents) => {
                let mut entries = contents
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                entries.drain(..entries.len().saturating_sub(MAX_HISTORY_ENTRIES));
                entries
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(_) => {
                return Self {
                    path: None,
                    entries: Vec::new(),
                    clear_on_exit: false,
                };
            }
        };
        Self {
            path: Some(path),
            entries,
            clear_on_exit: false,
        }
    }

    fn record(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        self.entries.retain(|entry| entry != line);
        self.entries.push(line.to_string());
        if self.entries.len() > MAX_HISTORY_ENTRIES {
            self.entries
                .drain(..self.entries.len() - MAX_HISTORY_ENTRIES);
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.clear_on_exit = false;
    }

    fn set_clear_on_exit(&mut self, clear: bool) {
        self.clear_on_exit = clear;
    }

    fn save(&self) -> io::Result<()> {
        let Some(path) = self.path.as_deref() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if self.clear_on_exit || self.entries.is_empty() {
            return fs::write(path, "");
        }
        let mut contents = self.entries.join("\n");
        contents.push('\n');
        fs::write(path, contents)
    }
}

pub(crate) fn run<R, W, E, F>(
    invocation: &mut CliInvocation,
    input: &mut R,
    output: &mut W,
    error: &mut E,
    echo_input: bool,
    initial_expression: Option<String>,
    mut evaluate: F,
) -> i32
where
    R: BufRead,
    W: Write,
    E: Write,
    F: FnMut(&CliInvocation, ReplRequest) -> Result<Option<ReplEvaluation>, String>,
{
    let mut history = History::load();
    let mut local_variables = BTreeMap::new();
    let mut last_expression = None;

    if let Some(expression) = initial_expression {
        let evaluated_expression = expression.clone();
        match evaluate(invocation, ReplRequest::Evaluate(expression)) {
            Ok(Some(evaluation)) => {
                update_local_variables(&mut local_variables, &evaluation.assignment_renderings);
                last_expression = Some(evaluated_expression);
                if render_evaluation(output, &evaluation).is_err() {
                    return 2;
                }
            }
            Ok(None) => {}
            Err(message) => {
                if writeln!(error, "error: {message}").is_err() {
                    return 2;
                }
            }
        }
    }

    loop {
        if write!(output, "{PROMPT}")
            .and_then(|()| output.flush())
            .is_err()
        {
            return 2;
        }

        let mut line = String::new();
        let read = match input.read_line(&mut line) {
            Ok(read) => read,
            Err(io_error) if io_error.kind() == io::ErrorKind::Interrupted => {
                let _ = writeln!(output);
                continue;
            }
            Err(io_error) => {
                let _ = writeln!(error, "error: failed to read interactive input: {io_error}");
                return finish_history(&history, error, 2);
            }
        };
        if read == 0 {
            return finish_history(&history, error, 0);
        }

        let line = line.trim_end_matches(['\r', '\n']).to_string();
        if echo_input && writeln!(output, "{line}").is_err() {
            return 2;
        }

        let command = match parse_interactive_command(&line) {
            Ok(command) => command,
            Err(message) => {
                if writeln!(error, "error: {message}").is_err() {
                    return 2;
                }
                continue;
            }
        };

        match command {
            InteractiveCommand::Noop => {}
            InteractiveCommand::Quit => return finish_history(&history, error, 0),
            InteractiveCommand::ClearHistory => history.clear(),
            InteractiveCommand::SetClearHistory(clear) => {
                history.set_clear_on_exit(clear);
                history.record(&line);
            }
            InteractiveCommand::History => {
                for entry in &history.entries {
                    if writeln!(output, "{entry}").is_err() {
                        return 2;
                    }
                }
                history.record(&line);
            }
            InteractiveCommand::Unknown => {
                history.record(&line);
                if writeln!(output, "Unknown command.\n").is_err() {
                    return 2;
                }
            }
            InteractiveCommand::Settings(settings) => {
                let recalculate_last_expression = settings
                    .iter()
                    .any(|setting| matches!(setting, SessionCommand::Assume(_)));
                let previous_len = invocation.interactive_settings.len();
                invocation.interactive_settings.extend(settings);
                history.record(&line);
                let reevaluated_expression = if recalculate_last_expression {
                    let Some(expression) = last_expression.clone() else {
                        continue;
                    };
                    (!expression_uses_managed_answer_alias(&expression)).then_some(expression)
                } else {
                    None
                };
                let request = reevaluated_expression
                    .as_ref()
                    .map_or(ReplRequest::ReformatLastAnswer, |expression| {
                        ReplRequest::Evaluate(expression.clone())
                    });
                match evaluate(invocation, request) {
                    Ok(Some(evaluation)) => {
                        if reevaluated_expression.is_some() {
                            update_local_variables(
                                &mut local_variables,
                                &evaluation.assignment_renderings,
                            );
                        }
                        if render_evaluation(output, &evaluation).is_err() {
                            return 2;
                        }
                    }
                    Ok(None) => {}
                    Err(message) => {
                        invocation.interactive_settings.truncate(previous_len);
                        if writeln!(error, "error: {message}").is_err() {
                            return 2;
                        }
                    }
                }
            }
            InteractiveCommand::Help(topic) => {
                history.record(&line);
                if render_help(output, topic.as_deref()).is_err() {
                    return 2;
                }
            }
            InteractiveCommand::List { list_type, query } => {
                history.record(&line);
                let request = ListRequest {
                    list_type,
                    search_term: query,
                };
                let local_can_be_authoritative =
                    request.list_type == ListType::All && request.search_term.is_none();
                let local_rendering = crate::listing::render_local_variable_list(
                    &request,
                    &local_variables,
                    local_can_be_authoritative,
                );
                let has_local_rendering = local_rendering.is_some();
                let local_is_authoritative = has_local_rendering && local_can_be_authoritative;
                if let Some(rendered) = local_rendering {
                    if write!(output, "{rendered}").is_err() {
                        return 2;
                    }
                }
                if local_is_authoritative {
                    continue;
                }
                let data_dir = libqalculate_rust::rates::definitions_dir();
                match crate::listing::render_list(
                    &data_dir,
                    &request,
                    &invocation.definitions,
                    super::super::cli_unicode_enabled(invocation),
                ) {
                    Ok(rendered) => {
                        if has_local_rendering && crate::listing::is_no_match_rendering(&rendered) {
                            if writeln!(output, "\n{}\n", crate::listing::list_footer()).is_err() {
                                return 2;
                            }
                            continue;
                        }
                        if write!(output, "{rendered}").is_err() {
                            return 2;
                        }
                    }
                    Err(message) => {
                        if writeln!(error, "error: {message}").is_err() {
                            return 2;
                        }
                    }
                }
            }
            InteractiveCommand::Info(query) => {
                history.record(&line);
                if let Some(rendered) =
                    crate::listing::render_local_variable_info(&query, &local_variables)
                {
                    if write!(output, "{rendered}").is_err() {
                        return 2;
                    }
                    continue;
                }
                let data_dir = libqalculate_rust::rates::definitions_dir();
                match crate::listing::render_info(
                    &data_dir,
                    &query,
                    &invocation.definitions,
                    super::super::cli_unicode_enabled(invocation),
                ) {
                    Ok(rendered) => {
                        if write!(output, "{rendered}").is_err() {
                            return 2;
                        }
                    }
                    Err(message) => {
                        if writeln!(error, "error: {message}").is_err() {
                            return 2;
                        }
                    }
                }
            }
            InteractiveCommand::Delete(name) => {
                history.record(&line);
                match evaluate(invocation, ReplRequest::Delete(name.clone())) {
                    Ok(_) => {
                        local_variables.remove(&name);
                    }
                    Err(message) => {
                        if writeln!(error, "error: {message}").is_err() {
                            return 2;
                        }
                    }
                }
            }
            InteractiveCommand::Expression(expression) => {
                history.record(&line);
                let evaluated_expression = expression.clone();
                match evaluate(invocation, ReplRequest::Evaluate(expression)) {
                    Ok(Some(evaluation)) => {
                        update_local_variables(
                            &mut local_variables,
                            &evaluation.assignment_renderings,
                        );
                        last_expression = Some(evaluated_expression);
                        if render_evaluation(output, &evaluation).is_err() {
                            return 2;
                        }
                    }
                    Ok(None) => {}
                    Err(message) => {
                        if writeln!(error, "error: {message}").is_err() {
                            return 2;
                        }
                    }
                }
            }
        }
    }
}

fn update_local_variables(
    local_variables: &mut BTreeMap<String, String>,
    assignment_renderings: &[(String, String)],
) {
    for (name, value) in assignment_renderings {
        local_variables.insert(name.clone(), value.clone());
    }
}

fn expression_uses_managed_answer_alias(expression: &str) -> bool {
    let Ok(parsed) = libqalculate_rust::parser::operators::parse_expression(expression) else {
        return false;
    };
    expression_tree_contains(&parsed, &|node| {
        let name = match node {
            libqalculate_rust::ast::Expression::Symbolic(symbol) => symbol.name(),
            libqalculate_rust::ast::Expression::Variable(variable) => variable.id(),
            libqalculate_rust::ast::Expression::Assignment { variable, .. } => variable,
            _ => return false,
        };
        matches!(
            name,
            "ans" | "answer" | "ans1" | "ans2" | "ans3" | "ans4" | "ans5"
        )
    })
}

fn expression_tree_contains(
    expression: &libqalculate_rust::ast::Expression,
    predicate: &impl Fn(&libqalculate_rust::ast::Expression) -> bool,
) -> bool {
    predicate(expression)
        || (0..expression.child_count()).any(|index| {
            expression
                .child(index)
                .is_some_and(|child| expression_tree_contains(child, predicate))
        })
}

fn render_evaluation<W: Write>(output: &mut W, evaluation: &ReplEvaluation) -> io::Result<()> {
    if evaluation.output.is_empty() {
        return Ok(());
    }
    writeln!(output)?;
    for line in evaluation.output.lines() {
        writeln!(output, "  {line}")?;
    }
    writeln!(output)
}

fn render_help<W: Write>(output: &mut W, topic: Option<&str>) -> io::Result<()> {
    let description = match topic.map(str::to_ascii_lowercase).as_deref() {
        Some("history") => "Lists the expression history.",
        Some("quit") | Some("exit") => "Terminates the current session.",
        Some("set") => "Changes a setting for the current session.",
        Some("list") => "Lists definitions, optionally filtered by type and name.",
        Some("info") => "Shows information about a definition.",
        Some(_) => "No detailed help is available for that topic.",
        None => {
            "Available commands: help, info, list, set, assume, history, clear history, quit, exit."
        }
    };
    writeln!(output, "\n{description}\n")
}

fn history_path() -> PathBuf {
    if let Some(user_dir) = std::env::var_os("QALCULATE_USER_DIR") {
        return PathBuf::from(user_dir).join("qalc.history");
    }
    if let Some(state_home) = std::env::var_os("XDG_STATE_HOME") {
        return PathBuf::from(state_home)
            .join("qalculate")
            .join("qalc.history");
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".local")
        .join("state")
        .join("qalculate")
        .join("qalc.history")
}

fn finish_history<E: Write>(history: &History, _error: &mut E, success_code: i32) -> i32 {
    // Upstream treats history as best-effort. A failed load disables persistence,
    // while a failed final write must not change the calculator's exit status.
    let _ = history.save();
    success_code
}
