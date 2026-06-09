#![forbid(unsafe_code)]

use std::env;
use std::path::{Path, PathBuf};

use libqalculate_rust::batch::read_batch_cases;
use libqalculate_rust::UPSTREAM_LIBQALCULATE_VERSION;

fn main() {
    let mut args = env::args().skip(1);
    let result = match args.next().as_deref() {
        Some("--version") | Some("-V") => {
            println!(
                "qalc-rs {} (upstream libqalculate {})",
                env!("CARGO_PKG_VERSION"),
                UPSTREAM_LIBQALCULATE_VERSION
            );
            Ok(())
        }
        Some("--self-check") => self_check(),
        Some("--list-upstream-tests") => list_upstream_tests(),
        Some("--parse-batch") => match args.next() {
            Some(path) => parse_batch(Path::new(&path)),
            None => Err("--parse-batch requires a file path".to_owned()),
        },
        Some("--help") | Some("-h") | None => {
            print_help();
            Ok(())
        }
        Some(other) => Err(format!("unknown argument: {other}")),
    };

    if let Err(error) = result {
        exit_with_error(&error);
    }
}

fn print_help() {
    println!("qalc-rs quality scaffold");
    println!("  --version              Print scaffold and upstream versions");
    println!("  --self-check           Verify upstream fixture inventory is readable");
    println!("  --list-upstream-tests  List upstream .batch fixtures");
    println!("  --parse-batch <path>   Parse a libqalculate .batch fixture");
}

fn self_check() -> Result<(), String> {
    let tests = upstream_batch_files()?;
    if tests.is_empty() {
        return Err("no upstream .batch fixtures found".to_owned());
    }
    println!("upstream_version={UPSTREAM_LIBQALCULATE_VERSION}");
    println!("upstream_batch_files={}", tests.len());
    Ok(())
}

fn list_upstream_tests() -> Result<(), String> {
    for path in upstream_batch_files()? {
        println!("{}", path.display());
    }
    Ok(())
}

fn parse_batch(path: &Path) -> Result<(), String> {
    let cases = read_batch_cases(path).map_err(|error| error.to_string())?;
    println!("cases={}", cases.len());
    Ok(())
}

fn upstream_batch_files() -> Result<Vec<PathBuf>, String> {
    let upstream = env::var_os("LIBQALCULATE_UPSTREAM_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../libqalculate"));
    let tests_dir = upstream.join("tests");
    let entries = std::fs::read_dir(&tests_dir)
        .map_err(|error| format!("failed to read {}: {error}", tests_dir.display()))?;
    let mut batches = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "batch"))
        .collect::<Vec<_>>();
    batches.sort();
    Ok(batches)
}

fn exit_with_error(message: &str) -> ! {
    eprintln!("error: {message}");
    std::process::exit(2);
}
