use super::commands::{parse_interactive_command, InteractiveCommand};
use super::{CliInvocation, ListRequest, ListType};
use libqalculate_rust::parser::commands::SessionCommand;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

const PROMPT: &str = "> ";
const MAX_HISTORY_ENTRIES: usize = 100;
const COMMAND_STREAM_QUIT_CODE: i32 = -1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputStyle {
    Interactive,
    CommandStream,
}

#[derive(Debug, Clone, Copy)]
struct RunOptions {
    prompt: bool,
    echo_input: bool,
    persistent_history: bool,
    skip_comments: bool,
    output_style: OutputStyle,
    input_name: &'static str,
    quit_code: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandStreamExit {
    Eof(i32),
    Quit,
}

#[derive(Debug, Default)]
pub(crate) struct ReplSessionState {
    local_variables: BTreeMap<String, String>,
    last_expression: Option<String>,
}

impl ReplSessionState {
    pub(crate) fn record_evaluation(
        &mut self,
        expression: String,
        assignment_renderings: &[(String, String)],
    ) {
        update_local_variables(&mut self.local_variables, assignment_renderings);
        self.last_expression = Some(expression);
    }
}

pub(crate) struct ReplIo<'a, R, W, E> {
    input: &'a mut R,
    output: &'a mut W,
    error: &'a mut E,
}

impl<'a, R, W, E> ReplIo<'a, R, W, E> {
    pub(crate) fn new(input: &'a mut R, output: &'a mut W, error: &'a mut E) -> Self {
        Self {
            input,
            output,
            error,
        }
    }
}

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
    session: &mut ReplSessionState,
    io: ReplIo<'_, R, W, E>,
    echo_input: bool,
    initial_expression: Option<String>,
    evaluate: F,
) -> i32
where
    R: BufRead,
    W: Write,
    E: Write,
    F: FnMut(&CliInvocation, ReplRequest) -> Result<Option<ReplEvaluation>, String>,
{
    run_with_options(
        invocation,
        session,
        io,
        initial_expression,
        RunOptions {
            prompt: true,
            echo_input,
            persistent_history: true,
            skip_comments: false,
            output_style: OutputStyle::Interactive,
            input_name: "interactive input",
            quit_code: 0,
        },
        evaluate,
    )
}

pub(crate) fn run_command_stream<R, W, E, F>(
    invocation: &mut CliInvocation,
    session: &mut ReplSessionState,
    io: ReplIo<'_, R, W, E>,
    evaluate: F,
) -> CommandStreamExit
where
    R: BufRead,
    W: Write,
    E: Write,
    F: FnMut(&CliInvocation, ReplRequest) -> Result<Option<ReplEvaluation>, String>,
{
    let exit_code = run_with_options(
        invocation,
        session,
        io,
        None,
        RunOptions {
            prompt: false,
            echo_input: false,
            persistent_history: false,
            skip_comments: true,
            output_style: OutputStyle::CommandStream,
            input_name: "command input",
            quit_code: COMMAND_STREAM_QUIT_CODE,
        },
        evaluate,
    );
    if exit_code == COMMAND_STREAM_QUIT_CODE {
        CommandStreamExit::Quit
    } else {
        CommandStreamExit::Eof(exit_code)
    }
}

fn run_with_options<R, W, E, F>(
    invocation: &mut CliInvocation,
    session: &mut ReplSessionState,
    io: ReplIo<'_, R, W, E>,
    initial_expression: Option<String>,
    options: RunOptions,
    mut evaluate: F,
) -> i32
where
    R: BufRead,
    W: Write,
    E: Write,
    F: FnMut(&CliInvocation, ReplRequest) -> Result<Option<ReplEvaluation>, String>,
{
    let input = io.input;
    let output = io.output;
    let error = io.error;
    let mut history = if options.persistent_history {
        History::load()
    } else {
        History {
            path: None,
            entries: Vec::new(),
            clear_on_exit: false,
        }
    };
    if let Some(expression) = initial_expression {
        let evaluated_expression = expression.clone();
        match evaluate(invocation, ReplRequest::Evaluate(expression)) {
            Ok(Some(evaluation)) => {
                session.record_evaluation(evaluated_expression, &evaluation.assignment_renderings);
                if render_evaluation(output, &evaluation, options.output_style).is_err() {
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
        if options.prompt
            && write!(output, "{PROMPT}")
                .and_then(|()| output.flush())
                .is_err()
        {
            return 2;
        }

        let mut line = String::new();
        let read = match input.read_line(&mut line) {
            Ok(read) => read,
            Err(io_error) if io_error.kind() == io::ErrorKind::Interrupted => {
                if options.prompt {
                    let _ = writeln!(output);
                }
                continue;
            }
            Err(io_error) => {
                let _ = writeln!(
                    error,
                    "error: failed to read {}: {io_error}",
                    options.input_name
                );
                return finish_history(&history, error, 2);
            }
        };
        if read == 0 {
            return finish_history(&history, error, 0);
        }

        let line = line.trim_end_matches(['\r', '\n']).to_string();
        if options.echo_input && writeln!(output, "{line}").is_err() {
            return 2;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || (options.skip_comments && trimmed.starts_with("//")) {
            continue;
        }

        let command = match parse_interactive_command(&line) {
            Ok(command) => command,
            Err(message) => {
                if options.output_style == OutputStyle::CommandStream {
                    if writeln!(output, "{message}").is_err() {
                        return 2;
                    }
                } else if writeln!(error, "error: {message}").is_err() {
                    return 2;
                }
                continue;
            }
        };

        match command {
            InteractiveCommand::Noop => {}
            InteractiveCommand::Quit => {
                return finish_history(&history, error, options.quit_code);
            }
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
                    let Some(expression) = session.last_expression.clone() else {
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
                        if let Some(expression) = reevaluated_expression.as_ref() {
                            session.record_evaluation(
                                expression.clone(),
                                &evaluation.assignment_renderings,
                            );
                        }
                        if render_evaluation(output, &evaluation, options.output_style).is_err() {
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
                    &session.local_variables,
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
                    crate::listing::render_local_variable_info(&query, &session.local_variables)
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
                        session.local_variables.remove(&name);
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
                        session.record_evaluation(
                            evaluated_expression,
                            &evaluation.assignment_renderings,
                        );
                        if render_evaluation(output, &evaluation, options.output_style).is_err() {
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

fn render_evaluation<W: Write>(
    output: &mut W,
    evaluation: &ReplEvaluation,
    style: OutputStyle,
) -> io::Result<()> {
    if evaluation.output.is_empty() {
        return Ok(());
    }
    if style == OutputStyle::CommandStream {
        writeln!(output, "{}", evaluation.output)?;
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

#[cfg(test)]
mod tests {
    use super::{run_command_stream, CommandStreamExit, ReplEvaluation, ReplIo, ReplSessionState};
    use std::io::{self, BufRead, Cursor, Read, Write};

    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("read failed"))
        }
    }

    impl BufRead for FailingReader {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            Err(io::Error::other("read failed"))
        }

        fn consume(&mut self, _amount: usize) {}
    }

    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "write failed"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn command_stream_reports_read_failures() {
        let mut invocation = super::super::parse_args(["qalc-rs", "-f", "-"]);
        let mut session = ReplSessionState::default();
        let mut input = FailingReader;
        let mut output = Vec::new();
        let mut error = Vec::new();

        let result = run_command_stream(
            &mut invocation,
            &mut session,
            ReplIo::new(&mut input, &mut output, &mut error),
            |_, _| panic!("read failure must happen before evaluation"),
        );

        assert_eq!(result, CommandStreamExit::Eof(2));
        assert_eq!(
            String::from_utf8(error).expect("diagnostic should be UTF-8"),
            "error: failed to read command input: read failed\n"
        );
    }

    #[test]
    fn command_stream_reports_write_failures() {
        let mut invocation = super::super::parse_args(["qalc-rs", "-f", "-"]);
        let mut session = ReplSessionState::default();
        let mut input = Cursor::new("1+1\n");
        let mut output = FailingWriter;
        let mut error = Vec::new();

        let result = run_command_stream(
            &mut invocation,
            &mut session,
            ReplIo::new(&mut input, &mut output, &mut error),
            |_, _| {
                Ok(Some(ReplEvaluation {
                    output: "2".to_string(),
                    answer_rendering: None,
                    assignment_renderings: Vec::new(),
                }))
            },
        );

        assert_eq!(result, CommandStreamExit::Eof(2));
        assert!(error.is_empty());
    }
}
