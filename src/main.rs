//! The process entry, and nothing else.
//!
//! It reads argv, hands it to [`thrall::cli::run`], writes the verdict's text
//! to the stream the code selects, and exits with that code. There is no
//! decision here — every one of them is in the library, where a test reads it
//! back as a value. That is what earns this file its place as the single
//! exclusion in `tarpaulin.toml`.

use std::process::ExitCode;

fn main() -> ExitCode {
    let verdict = thrall::cli::run(std::env::args().skip(1).collect());
    if verdict.code == 0 {
        println!("{}", verdict.text);
    } else {
        eprintln!("{}", verdict.text);
    }
    ExitCode::from(verdict.code)
}
