//! The process entry, and nothing else.
//!
//! It reads argv, hands it to [`thrall::cli::run`], performs whatever that
//! decided, writes the verdict's text to the stream the code selects, and exits
//! with that code. There is no decision here — every one of them is in the
//! library, where a test reads it back as a value. That is what earns this file
//! its place as the single exclusion in `tarpaulin.toml`.
//!
//! Five things are the entry point's rather than the library's, and all five
//! are the reason the exclusion is honest: **this process's own environment**,
//! folded once into the data root; **serving**, which is not a value a test can
//! read back but a program dialling engines until they stop; **where a serving
//! foot's notices go**, which is this process's stderr — an effect, and the one
//! thing `serve` cannot answer as a `Verdict` because it has to be said while
//! the channels are still up rather than after they have all stopped; **the
//! real sleep between dials**, which is the same kind of thing one register
//! down (`run::Pause`): a suite that slept a redial's backoff out would spend a
//! minute proving arithmetic; and **this process's own stdin**, which the
//! bridge's input arrives on (DESIGN §6.2). All five are effects, and the
//! decision of WHICH of them one argv calls for is `cli::run`'s, where a test
//! reads it back as a value.
//!
//! **A bridged call answers in three facts and not in a `Verdict`**, because
//! it is being run as a tool: its content goes to stdout and its exit code is
//! the verdict even when that code is not zero (a tool that failed still says
//! why, and the model at the far end reads it). So the two shapes are written
//! out separately here rather than folded into one — the fold would be a
//! `Verdict` that carries two streams, which is a second answer to a question
//! `Capture` already answers.

use std::process::ExitCode;
use std::sync::Arc;

use thrall::cli::{Decided, Verdict};
use thrall::invocation::Capture;
use thrall::run::{Notice, Pause};

fn main() -> ExitCode {
    match thrall::cli::run(std::env::args().skip(1).collect()) {
        Decided::Say(verdict) => said(verdict),
        Decided::Serve => said(serve()),
        Decided::Bridge { tool, server } => {
            captured(&thrall::mcp::bridge(&tool, &server, &input()))
        }
    }
}

/// Serve every channel this box holds, until they have all stopped.
fn serve() -> Verdict {
    match thrall::paths::data_root() {
        Ok(root) => {
            let notice: Notice = Arc::new(|line: &str| eprintln!("{line}"));
            let pause: Pause = Arc::new(std::thread::sleep);
            thrall::serve::serve(&root, &notice, &pause)
        }
        Err(reason) => Verdict::failed(reason),
    }
}

/// A verdict, on the stream its own code selects.
fn said(verdict: Verdict) -> ExitCode {
    if verdict.code == 0 {
        println!("{}", verdict.text);
    } else {
        eprintln!("{}", verdict.text);
    }
    ExitCode::from(verdict.code)
}

/// A capture, as the tool contract this process was spawned under: stdout,
/// stderr and the exit code, each where it belongs and none of them elsewhere.
/// A code that will not fit a byte is a code no caller can read either, and 1
/// is what every failure here already means.
fn captured(capture: &Capture) -> ExitCode {
    print!("{}", capture.stdout);
    eprint!("{}", capture.stderr);
    ExitCode::from(u8::try_from(capture.exit_code).unwrap_or(1))
}

/// **This process's own stdin**, whole — the invocation's input JSON. Read
/// here because reading a stream is an effect; unreadable reads as empty,
/// which the bridge refuses by name.
fn input() -> String {
    std::io::read_to_string(std::io::stdin()).unwrap_or_default()
}
