//! The process entry, and nothing else.
//!
//! It reads argv, hands it to [`thrall::cli::run`], performs whatever that
//! decided, writes the verdict's text to the stream the code selects, and exits
//! with that code. There is no decision here — every one of them is in the
//! library, where a test reads it back as a value. That is what earns this file
//! its place as the single exclusion in `tarpaulin.toml`.
//!
//! Two things are the entry point's rather than the library's, and both are the
//! reason the exclusion is honest: **this process's own environment**, folded
//! once into the data root, and **serving**, which is not a value a test can
//! read back but a program dialling engines until they stop.

use std::process::ExitCode;

use thrall::cli::{Decided, Verdict};

fn main() -> ExitCode {
    let verdict = match thrall::cli::run(std::env::args().skip(1).collect()) {
        Decided::Say(verdict) => verdict,
        Decided::Serve => match thrall::paths::data_root() {
            Ok(root) => thrall::serve::serve(&root),
            Err(reason) => Verdict::failed(reason),
        },
    };
    if verdict.code == 0 {
        println!("{}", verdict.text);
    } else {
        eprintln!("{}", verdict.text);
    }
    ExitCode::from(verdict.code)
}
