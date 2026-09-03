//! The process entry, and nothing else.
//!
//! It reads argv, hands it to [`thrall::cli::run`], performs whatever that
//! decided, writes the verdict's text to the stream the code selects, and exits
//! with that code. There is no decision here — every one of them is in the
//! library, where a test reads it back as a value. That is what earns this file
//! its place as the single exclusion in `tarpaulin.toml`.
//!
//! Three things are the entry point's rather than the library's, and all three
//! are the reason the exclusion is honest: **this process's own environment**,
//! folded once into the data root; **serving**, which is not a value a test can
//! read back but a program dialling engines until they stop; and **where a
//! serving foot's notices go**, which is this process's stderr — an effect, and
//! the one thing `serve` cannot answer as a `Verdict` because it has to be said
//! while the channels are still up rather than after they have all stopped.

use std::process::ExitCode;
use std::sync::Arc;

use thrall::cli::{Decided, Verdict};
use thrall::run::Notice;

fn main() -> ExitCode {
    let verdict = match thrall::cli::run(std::env::args().skip(1).collect()) {
        Decided::Say(verdict) => verdict,
        Decided::Serve => match thrall::paths::data_root() {
            Ok(root) => {
                let notice: Notice = Arc::new(|line: &str| eprintln!("{line}"));
                thrall::serve::serve(&root, &notice)
            }
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
